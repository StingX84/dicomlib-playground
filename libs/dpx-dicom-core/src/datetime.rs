//! DICOM Date/Time value helper types (`DA`, `TM`, `DT` and their range forms).
//!
//! These are standalone value types modelling the DICOM PS3.5 Value
//! Representations for dates, times and date-times, plus the Range Matching
//! forms used in C-FIND queries (PS3.4). They handle text parsing/formatting,
//! partial precision, leap seconds, timezone offsets and the C-FIND specific
//! rules for offset handling.
//!
//! Unset components are modelled with [`Option`] (the C++ original used
//! `numeric_limits::max()` sentinels). Fractional seconds are kept at full
//! DICOM precision as microseconds (`FFFFFF`).

use crate::{dicom_err, ensure, error::Result};

// Field widths in the DICOM textual representation.
const FMT_YEAR_DIGITS: usize = 4;
const FMT_MONTH_DIGITS: usize = 2;
const FMT_DAY_DIGITS: usize = 2;
const FMT_HOUR_DIGITS: usize = 2;
const FMT_MINUTE_DIGITS: usize = 2;
const FMT_SECOND_DIGITS: usize = 2;
const FMT_FRACTION_DIGITS_MAX: usize = 6;
const FMT_TZ_LENGTH: usize = 1 + FMT_HOUR_DIGITS + FMT_MINUTE_DIGITS;

const LIM_YEAR_MIN: u16 = 1;
const LIM_YEAR_MAX: u16 = 9999;
const LIM_MONTH_MIN: u8 = 1;
const LIM_MONTH_MAX: u8 = 12;
const LIM_DAY_MIN: u8 = 1;
const LIM_HOUR_MAX: u8 = 23;
const LIM_MINUTE_MAX: u8 = 59;
const LIM_SECOND_MAX: u8 = 59;
const LIM_FRACTION_MAX: u32 = 999_999;

const TZ_MIN_SECONDS: i32 = -12 * 3600;
const TZ_MAX_SECONDS: i32 = 14 * 3600;

const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Number of days in `month` (1..=12) of `year`, honouring leap years.
///
/// Uses the proleptic Gregorian calendar for dates before 1582.
fn days_in_month(year: u16, month: u8) -> u8 {
    if month == 2 && is_leap_year(year) {
        29
    } else if (1..=12).contains(&month) {
        DAYS_IN_MONTH[(month - 1) as usize]
    } else {
        31
    }
}

// ============================================================================
// Civil date <-> days-since-epoch arithmetic
// ============================================================================
//
// Used only to recompute wall-clock across day boundaries in
// `DicomDateTime::adjust_to_offset`. Algorithm after Howard Hinnant's
// `days_from_civil` / `civil_from_days` (public domain), epoch 1970-01-01.

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let doy = ((153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5) + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((y + i64::from(m <= 2)) as i32, m, d)
}

// ============================================================================
// Textual parsing helpers
// ============================================================================

fn rtrim(b: &[u8]) -> &[u8] {
    let mut end = b.len();
    while end > 0 && b[end - 1] == b' ' {
        end -= 1;
    }
    &b[..end]
}

/// Parse exactly `width` ASCII decimal digits from the front of `b`, returning
/// the value and the remaining slice. Fails on short input or non-digits.
fn parse_fixed(b: &[u8], width: usize) -> Option<(u32, &[u8])> {
    if b.len() < width {
        return None;
    }
    let mut value: u32 = 0;
    for &c in &b[..width] {
        if !c.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(c - b'0');
    }
    Some((value, &b[width..]))
}

fn parse_field(b: &[u8], width: usize, min: u32, max: u32) -> Option<(u32, &[u8])> {
    let (value, rest) = parse_fixed(b, width)?;
    (value >= min && value <= max).then_some((value, rest))
}

/// Parse a `.FFFFFF` fractional-second suffix into microseconds (1..=6 digits).
///
/// Returns the rest of the slice after the consumed digits, or `None` if `b`
/// does not start with `.` followed by at least one digit.
fn parse_fraction(b: &[u8]) -> Option<(u32, &[u8])> {
    if b.first() != Some(&b'.') {
        return None;
    }
    let digits = &b[1..];
    let count = digits
        .iter()
        .take(FMT_FRACTION_DIGITS_MAX)
        .take_while(|c| c.is_ascii_digit())
        .count();
    if count == 0 {
        return None;
    }
    let mut us: u32 = 0;
    for &c in &digits[..count] {
        us = us * 10 + u32::from(c - b'0');
    }
    for _ in count..FMT_FRACTION_DIGITS_MAX {
        us *= 10;
    }
    Some((us, &digits[count..]))
}

/// Parse a `&ZZXX` timezone offset that must occupy the whole of `b`.
fn parse_tz(b: &[u8]) -> Option<i32> {
    if b.len() != FMT_TZ_LENGTH {
        return None;
    }
    let sign = match b[0] {
        b'-' => -1,
        b'+' => 1,
        _ => return None,
    };
    let (hours, rest) = parse_fixed(&b[1..], FMT_HOUR_DIGITS)?;
    let (minutes, rest) = parse_fixed(rest, FMT_MINUTE_DIGITS)?;
    if !rest.is_empty() || minutes > u32::from(LIM_MINUTE_MAX) {
        return None;
    }
    let seconds = sign * (hours as i32 * 3600 + minutes as i32 * 60);
    (TZ_MIN_SECONDS..=TZ_MAX_SECONDS).contains(&seconds).then_some(seconds)
}

// ============================================================================
// Textual writing helpers
// ============================================================================

fn write_field(out: &mut Vec<u8>, value: u32, width: usize) {
    let mut buf = [0u8; 10];
    let mut n = value;
    let mut len = 0;
    loop {
        buf[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
        if n == 0 {
            break;
        }
    }
    for _ in len..width {
        out.push(b'0');
    }
    for i in (0..len).rev() {
        out.push(buf[i]);
    }
}

fn write_tz(out: &mut Vec<u8>, seconds: i32) {
    let (sign, abs) = if seconds < 0 { (b'-', -seconds) } else { (b'+', seconds) };
    out.push(sign);
    write_field(out, (abs / 3600) as u32, FMT_HOUR_DIGITS);
    write_field(out, ((abs / 60) % 60) as u32, FMT_MINUTE_DIGITS);
}

// ============================================================================
// DicomTimeZoneOffset
// ============================================================================

/// Timezone offset from UTC, used by the `DT` suffix and by the
/// Timezone Offset From UTC (0008,0201) attribute.
///
/// [`Fixed`](Self::Fixed) holds an explicit offset in **seconds** from UTC,
/// constrained to `-12:00 ..= +14:00`. [`Local`](Self::Local) denotes the
/// application's local timezone, a runtime concept that never appears in
/// parsed DICOM text (text only carries explicit `±HHMM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DicomTimeZoneOffset {
    /// Local timezone of the application (resolved at runtime).
    Local,
    /// Explicit offset from UTC in seconds (`-43200 ..= 50400`).
    Fixed(i32),
}

impl DicomTimeZoneOffset {
    /// `true` if this is a [`Fixed`](Self::Fixed) offset with seconds within
    /// the DICOM-permitted range.
    pub fn is_valid(self) -> bool {
        match self {
            Self::Local => false,
            Self::Fixed(s) => (TZ_MIN_SECONDS..=TZ_MAX_SECONDS).contains(&s),
        }
    }

    /// `true` for a negative fixed offset (earlier than UTC).
    pub fn is_negative(self) -> bool {
        matches!(self, Self::Fixed(s) if s < 0)
    }

    fn seconds(self) -> i32 {
        match self {
            Self::Local => 0,
            Self::Fixed(s) => s,
        }
    }

    /// Parse a `&ZZXX` offset string, e.g. `"+0300"`, `"-0500"`, `"+0000"`.
    ///
    /// Trailing spaces are stripped. An empty value yields `Ok(None)`.
    pub fn from_dicom(input: &[u8]) -> Result<Option<Self>> {
        let b = rtrim(input);
        if b.is_empty() {
            return Ok(None);
        }
        match parse_tz(b) {
            Some(s) => Ok(Some(Self::Fixed(s))),
            None => Err(dicom_err!(InvalidData, "invalid DICOM timezone offset")),
        }
    }

    /// Append the `&ZZXX` representation. [`Local`](Self::Local) and
    /// out-of-range offsets write nothing.
    pub fn to_dicom(self, out: &mut Vec<u8>) {
        if self.is_valid() {
            write_tz(out, self.seconds());
        }
    }
}

// ============================================================================
// DicomDate
// ============================================================================

/// A DICOM `DA` value: `YYYYMMDD` with optional partial precision
/// (year-only or year+month).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DicomDate {
    pub y: Option<u16>,
    pub m: Option<u8>,
    pub d: Option<u8>,
}

impl DicomDate {
    /// `true` when no year is set (the value is empty).
    pub fn is_null(self) -> bool {
        self.y.is_none()
    }

    /// `true` when year, month and day are all present.
    pub fn is_all_fields_set(self) -> bool {
        self.y.is_some() && self.m.is_some() && self.d.is_some()
    }

    /// Expand unset trailing components to their lowest valid value
    /// (month 1, day 1).
    pub fn minimized(self) -> Self {
        Self {
            y: Some(self.y.unwrap_or(LIM_YEAR_MIN)),
            m: Some(self.m.unwrap_or(LIM_MONTH_MIN)),
            d: Some(self.d.unwrap_or(LIM_DAY_MIN)),
        }
    }

    /// Expand unset trailing components to their highest valid value
    /// (month 12, last day of month).
    pub fn maximized(self) -> Self {
        let year = self.y.unwrap_or(LIM_YEAR_MAX);
        let month = self.m.unwrap_or(LIM_MONTH_MAX);
        Self {
            y: Some(year),
            m: Some(month),
            d: Some(self.d.unwrap_or_else(|| days_in_month(year, month))),
        }
    }

    /// Parse a date with optional partial precision (`YYYY`, `YYYYMM`,
    /// `YYYYMMDD`), used inside `DT` and the range forms.
    ///
    /// The month is retained independently of the day: if the month parses but
    /// the day does not, the result carries the month and stops after it. A day
    /// that exceeds the month's length fails the whole parse.
    fn parse(b: &[u8]) -> Option<(Self, &[u8])> {
        let (y, rest) = parse_field(b, FMT_YEAR_DIGITS, u32::from(LIM_YEAR_MIN), u32::from(LIM_YEAR_MAX))?;
        let mut date = Self { y: Some(y as u16), m: None, d: None };
        if let Some((m, rest_m)) = parse_field(rest, FMT_MONTH_DIGITS, u32::from(LIM_MONTH_MIN), u32::from(LIM_MONTH_MAX))
        {
            date.m = Some(m as u8);
            if let Some((d, rest_d)) = parse_field(rest_m, FMT_DAY_DIGITS, u32::from(LIM_DAY_MIN), 31) {
                if d as u8 > days_in_month(y as u16, m as u8) {
                    return None;
                }
                date.d = Some(d as u8);
                return Some((date, rest_d));
            }
            return Some((date, rest_m));
        }
        Some((date, rest))
    }

    /// Parse a complete `DA` value, which must be the full `YYYYMMDD` form.
    ///
    /// Trailing spaces are stripped; an empty value yields a null date. Unlike
    /// the partial-precision form accepted inside `DT`/ranges, a standalone
    /// `DA` requires all eight digits.
    pub fn from_dicom(input: &[u8]) -> Result<Self> {
        let b = rtrim(input);
        if b.is_empty() {
            return Ok(Self::default());
        }
        let (date, rest) = Self::parse(b).ok_or_else(|| dicom_err!(InvalidData, "invalid DICOM date"))?;
        ensure!(
            rest.is_empty() && date.is_all_fields_set(),
            InvalidData,
            "DICOM date must be the full YYYYMMDD form"
        );
        Ok(date)
    }

    /// Append `YYYYMMDD`, padding any unset month/day with `01`. Writes nothing
    /// for a null date.
    pub fn to_dicom(self, out: &mut Vec<u8>) {
        if let Some(y) = self.y {
            write_field(out, u32::from(y), FMT_YEAR_DIGITS);
            write_field(out, u32::from(self.m.unwrap_or(LIM_MONTH_MIN)), FMT_MONTH_DIGITS);
            write_field(out, u32::from(self.d.unwrap_or(LIM_DAY_MIN)), FMT_DAY_DIGITS);
        }
    }

    /// Append the date using only the components that are present
    /// (`YYYY`, `YYYYMM` or `YYYYMMDD`).
    fn to_dicom_partial(self, out: &mut Vec<u8>) {
        if let Some(y) = self.y {
            write_field(out, u32::from(y), FMT_YEAR_DIGITS);
            if let Some(m) = self.m {
                write_field(out, u32::from(m), FMT_MONTH_DIGITS);
                if let Some(d) = self.d {
                    write_field(out, u32::from(d), FMT_DAY_DIGITS);
                }
            }
        }
    }
}

// ============================================================================
// DicomTime
// ============================================================================

/// A DICOM `TM` value: `HHMMSS.FFFFFF` with optional partial precision.
///
/// Fractional seconds are microseconds (`0..=999_999`). A leap second
/// (`s == 60`) read from text is clamped to `59`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DicomTime {
    pub h: Option<u8>,
    pub m: Option<u8>,
    pub s: Option<u8>,
    pub frac_us: Option<u32>,
}

impl DicomTime {
    /// `true` when no hour is set (the value is empty).
    pub fn is_null(self) -> bool {
        self.h.is_none()
    }

    /// `true` when hour, minute, second and fraction are all present.
    pub fn is_all_fields_set(self) -> bool {
        self.h.is_some() && self.m.is_some() && self.s.is_some() && self.frac_us.is_some()
    }

    /// Expand unset trailing components to their lowest valid value (zero).
    pub fn minimized(self) -> Self {
        Self {
            h: Some(self.h.unwrap_or(0)),
            m: Some(self.m.unwrap_or(0)),
            s: Some(self.s.unwrap_or(0)),
            frac_us: Some(self.frac_us.unwrap_or(0)),
        }
    }

    /// Expand unset trailing components to their highest valid value
    /// (`23:59:59.999999`).
    pub fn maximized(self) -> Self {
        Self {
            h: Some(self.h.unwrap_or(LIM_HOUR_MAX)),
            m: Some(self.m.unwrap_or(LIM_MINUTE_MAX)),
            s: Some(self.s.unwrap_or(LIM_SECOND_MAX)),
            frac_us: Some(self.frac_us.unwrap_or(LIM_FRACTION_MAX)),
        }
    }

    fn parse(b: &[u8]) -> Option<(Self, &[u8])> {
        let (h, rest) = parse_field(b, FMT_HOUR_DIGITS, 0, u32::from(LIM_HOUR_MAX))?;
        let mut time = Self { h: Some(h as u8), m: None, s: None, frac_us: None };
        let mut rest = rest;
        if let Some((m, r)) = parse_field(rest, FMT_MINUTE_DIGITS, 0, u32::from(LIM_MINUTE_MAX)) {
            time.m = Some(m as u8);
            rest = r;
            // SS may legally be 60 for a leap second; clamp it to 59 on read.
            if let Some((s, r)) = parse_field(rest, FMT_SECOND_DIGITS, 0, 60) {
                time.s = Some((s as u8).min(LIM_SECOND_MAX));
                rest = r;
                if let Some((us, r)) = parse_fraction(rest) {
                    time.frac_us = Some(us);
                    rest = r;
                }
            }
        }
        Some((time, rest))
    }

    /// Parse a complete `TM` value. Trailing spaces are stripped; an empty
    /// value yields a null time.
    pub fn from_dicom(input: &[u8]) -> Result<Self> {
        let b = rtrim(input);
        if b.is_empty() {
            return Ok(Self::default());
        }
        let (time, rest) = Self::parse(b).ok_or_else(|| dicom_err!(InvalidData, "invalid DICOM time"))?;
        ensure!(rest.is_empty(), InvalidData, "trailing characters in DICOM time");
        Ok(time)
    }

    /// Append using only the components present. The fractional part is
    /// written (as `.FFFFFF`, 6 digits) only when non-zero.
    pub fn to_dicom(self, out: &mut Vec<u8>) {
        if let Some(h) = self.h {
            write_field(out, u32::from(h), FMT_HOUR_DIGITS);
            if let Some(m) = self.m {
                write_field(out, u32::from(m), FMT_MINUTE_DIGITS);
                if let Some(s) = self.s {
                    write_field(out, u32::from(s), FMT_SECOND_DIGITS);
                    if let Some(us) = self.frac_us.filter(|&us| us != 0) {
                        out.push(b'.');
                        write_field(out, us, FMT_FRACTION_DIGITS_MAX);
                    }
                }
            }
        }
    }
}

// ============================================================================
// DicomDateTime
// ============================================================================

/// A DICOM `DT` value: `YYYYMMDDHHMMSS.FFFFFF&ZZXX`.
///
/// `offset` is the timezone offset (`None` when absent from both the text and
/// the dataset). `tz_from_dataset` records whether `offset` was supplied by the
/// dataset's Timezone Offset From UTC rather than present in the text.
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomDateTime {
    pub date: DicomDate,
    pub time: DicomTime,
    pub offset: Option<DicomTimeZoneOffset>,
    pub tz_from_dataset: bool,
}

impl DicomDateTime {
    /// `true` when the date component carries no year.
    pub fn is_null(self) -> bool {
        self.date.is_null()
    }

    /// `true` when every date and time component is present.
    pub fn is_all_fields_set(self) -> bool {
        self.date.is_all_fields_set() && self.time.is_all_fields_set()
    }

    /// Expand unset trailing date and time components to their lowest values.
    pub fn minimized(self) -> Self {
        Self {
            date: self.date.minimized(),
            time: self.time.minimized(),
            ..self
        }
    }

    /// Expand unset trailing date and time components to their highest values.
    pub fn maximized(self) -> Self {
        Self {
            date: self.date.maximized(),
            time: self.time.maximized(),
            ..self
        }
    }

    /// `PartialEq` ignores `tz_from_dataset`, matching the C++ `operator==`.
    fn eq_value(self, other: Self) -> bool {
        self.date == other.date && self.time == other.time && self.offset == other.offset
    }

    fn parse(b: &[u8]) -> Option<(Self, &[u8])> {
        let (date, rest) = DicomDate::parse(b)?;
        let mut dt = Self { date, ..Self::default() };
        let mut rest = rest;
        if !rest.is_empty()
            && rest[0] != b'+'
            && rest[0] != b'-'
            && let Some((time, r)) = DicomTime::parse(rest)
        {
            dt.time = time;
            rest = r;
        }
        if !rest.is_empty() {
            let off = parse_tz(rest)?;
            dt.offset = Some(DicomTimeZoneOffset::Fixed(off));
            rest = &rest[rest.len()..];
        }
        Some((dt, rest))
    }

    /// Parse a complete `DT` value.
    ///
    /// When the text carries no offset, `offset_in_dataset` is adopted and
    /// `tz_from_dataset` is set. For C-FIND keys a negative explicit offset is
    /// rejected (PS3.4 C.2.2.2.1.3).
    pub fn from_dicom(
        input: &[u8],
        is_cfind_rq: bool,
        offset_in_dataset: Option<DicomTimeZoneOffset>,
    ) -> Result<Self> {
        let b = rtrim(input);
        if b.is_empty() {
            return Ok(Self::default());
        }
        let (mut dt, rest) = Self::parse(b).ok_or_else(|| dicom_err!(InvalidData, "invalid DICOM date-time"))?;
        ensure!(rest.is_empty(), InvalidData, "trailing characters in DICOM date-time");
        ensure!(
            !(is_cfind_rq && dt.offset.is_some_and(DicomTimeZoneOffset::is_negative)),
            InvalidData,
            "negative timezone offset not allowed in C-FIND DT key"
        );
        if dt.offset.is_none() {
            dt.offset = offset_in_dataset;
            dt.tz_from_dataset = offset_in_dataset.is_some();
        }
        Ok(dt)
    }

    /// Resolve [`Local`](DicomTimeZoneOffset::Local) to a concrete fixed offset
    /// for the value's wall-clock date.
    //
    // ponytail: needs OS timezone lookup (chrono Local); deferred. Returns an
    // error so callers relying on Local resolution surface it explicitly. The
    // pure DICOM parse/format/arithmetic paths never produce Local.
    fn resolve_offset(offset: DicomTimeZoneOffset) -> Result<i32> {
        match offset {
            DicomTimeZoneOffset::Fixed(s) => Ok(s),
            DicomTimeZoneOffset::Local => Err(dicom_err!(
                UnsupportedFeature,
                "resolving Local timezone offset requires an OS timezone database (not yet implemented)"
            )),
        }
    }

    /// Recompute this value's wall-clock for a (possibly different) dataset
    /// offset, returning the adjusted value plus two flags:
    ///
    /// - `becomes_more_specific`: the adjustment forced a previously-unset time
    ///   component to become significant (the caller should re-run for the
    ///   `minimized`/`maximized` variants to form a range).
    /// - `offset_write_required`: the resulting offset must be written out.
    ///
    /// For C-FIND, a negative resulting offset is converted to UTC `+0000`
    /// (PS3.4 C.2.2.2.1.3).
    pub fn adjust_to_offset(
        self,
        is_cfind_rq: bool,
        always_write_offset: bool,
        offset_in_dataset: Option<DicomTimeZoneOffset>,
    ) -> Result<(Self, bool, bool)> {
        let mut rv = self;
        let mut becomes_more_specific = false;
        let mut write_offset = always_write_offset;

        if self.is_null() {
            return Ok((rv, becomes_more_specific, write_offset));
        }

        // Seconds by which the wall-clock must shift to land in the target zone.
        let mut tz_adjustment_sec: i32 = 0;

        match rv.offset {
            None => {
                if offset_in_dataset.is_none() || !self.tz_from_dataset {
                    if always_write_offset {
                        rv.offset = Some(DicomTimeZoneOffset::Local);
                    }
                } else {
                    let ds = Self::resolve_offset(offset_in_dataset.unwrap_or(DicomTimeZoneOffset::Local))?;
                    let current = Self::resolve_offset(DicomTimeZoneOffset::Local)?;
                    tz_adjustment_sec = ds - current;
                    rv.offset = offset_in_dataset;
                }
            }
            Some(off) if self.tz_from_dataset => match offset_in_dataset {
                None => {
                    let current = Self::resolve_offset(DicomTimeZoneOffset::Local)?;
                    tz_adjustment_sec = current - Self::resolve_offset(off)?;
                    rv.offset = Some(DicomTimeZoneOffset::Local);
                }
                Some(ds) if ds != off => {
                    tz_adjustment_sec = Self::resolve_offset(ds)? - Self::resolve_offset(off)?;
                    rv.offset = offset_in_dataset;
                }
                Some(_) => {}
            },
            Some(off) => {
                // Explicit offset that did not come from the dataset: written
                // verbatim unless it already matches the dataset offset.
                write_offset = offset_in_dataset != Some(off);
            }
        }

        if is_cfind_rq && rv.offset.is_some_and(DicomTimeZoneOffset::is_negative) {
            tz_adjustment_sec -= rv.offset.map(Self::resolve_offset).transpose()?.unwrap_or(0);
            rv.offset = Some(DicomTimeZoneOffset::Fixed(0));
        }

        if tz_adjustment_sec != 0 {
            self.apply_shift(&mut rv, tz_adjustment_sec, &mut becomes_more_specific);
        }

        if always_write_offset {
            rv.tz_from_dataset = false;
        }

        Ok((rv, becomes_more_specific, write_offset))
    }

    /// Shift `rv`'s wall-clock by `shift_sec`, re-deriving date/time across any
    /// day boundary, then re-hide time components that were unset and remained
    /// insignificant after the shift.
    fn apply_shift(self, rv: &mut Self, shift_sec: i32, becomes_more_specific: &mut bool) {
        let mins = self.minimized();
        let total_secs = i64::from(mins.time.h.unwrap_or(0)) * 3600
            + i64::from(mins.time.m.unwrap_or(0)) * 60
            + i64::from(mins.time.s.unwrap_or(0))
            + i64::from(shift_sec);
        let us = mins.time.frac_us.unwrap_or(0);

        let base_days = days_from_civil(
            i32::from(mins.date.y.unwrap_or(LIM_YEAR_MIN)),
            u32::from(mins.date.m.unwrap_or(LIM_MONTH_MIN)),
            u32::from(mins.date.d.unwrap_or(LIM_DAY_MIN)),
        );
        let day_delta = total_secs.div_euclid(86_400);
        let secs_of_day = total_secs.rem_euclid(86_400);
        let (ny, nm, nd) = civil_from_days(base_days + day_delta);

        rv.date = DicomDate { y: Some(ny as u16), m: Some(nm as u16 as u8), d: Some(nd as u8) };
        rv.time = DicomTime {
            h: Some((secs_of_day / 3600) as u8),
            m: Some(((secs_of_day / 60) % 60) as u8),
            s: Some((secs_of_day % 60) as u8),
            frac_us: Some(us),
        };

        // Re-hide trailing time components that were unset and stayed
        // insignificant; flag the value as more specific where the shift made a
        // previously-unset component meaningful.
        if self.time.frac_us.is_none() {
            rv.time.frac_us = None;
            if self.time.s.is_none() {
                if shift_sec % 60 != 0 {
                    *becomes_more_specific = true;
                } else {
                    rv.time.s = None;
                    if self.time.m.is_none() {
                        if shift_sec % 3600 != 0 {
                            *becomes_more_specific = true;
                        } else {
                            rv.time.m = None;
                            if self.time.h.is_none() {
                                *becomes_more_specific = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn write_body(self, out: &mut Vec<u8>, write_offset: bool) {
        if self.is_null() {
            return;
        }
        let off = if write_offset { self.offset } else { None };
        if self.time.is_null() {
            self.date.to_dicom_partial(out);
        } else {
            self.date.to_dicom(out);
            self.time.to_dicom(out);
        }
        if let Some(off) = off {
            off.to_dicom(out);
        }
    }

    /// Append the `DT` representation.
    ///
    /// `is_cfind_rq`, `always_write_offset` and `offset_in_dataset` drive the
    /// offset-adjustment rules. For a C-FIND key whose adjustment makes the
    /// value more specific, a `from-maximized` range (`min-max`) is emitted.
    pub fn to_dicom(
        self,
        out: &mut Vec<u8>,
        is_cfind_rq: bool,
        always_write_offset: bool,
        offset_in_dataset: Option<DicomTimeZoneOffset>,
    ) -> Result<()> {
        if self.is_null() {
            return Ok(());
        }
        let (adjusted, becomes_more_specific, write_offset) =
            self.adjust_to_offset(is_cfind_rq, always_write_offset, offset_in_dataset)?;

        if is_cfind_rq && becomes_more_specific {
            let (lo, _, w_lo) =
                self.minimized().adjust_to_offset(is_cfind_rq, always_write_offset, offset_in_dataset)?;
            lo.write_body(out, w_lo);
            out.push(b'-');
            let (hi, _, w_hi) =
                self.maximized().adjust_to_offset(is_cfind_rq, always_write_offset, offset_in_dataset)?;
            hi.write_body(out, w_hi);
        } else {
            adjusted.write_body(out, write_offset);
        }
        Ok(())
    }
}

impl PartialEq for DicomDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.eq_value(*other)
    }
}

impl Eq for DicomDateTime {}

// ============================================================================
// Range types
// ============================================================================

/// Split a Range-Matching value into its `from` / `to` byte halves.
///
/// Returns `Ok(None)` for an empty (whitespace-only) input. For a value with no
/// `-`, both halves reference the whole input (single-value form). `Err` is
/// reserved for callers to map; this only classifies the shape.
fn split_range(input: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    let b = rtrim(input);
    if b.is_empty() {
        return None;
    }
    match b.iter().position(|&c| c == b'-') {
        Some(i) => Some((&b[..i], Some(&b[i + 1..]))),
        None => Some((b, None)),
    }
}

/// A DICOM `DA` Range-Matching value (`d1 - d2`, `- d2` or `d1 -`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DicomDateRange {
    pub from: Option<DicomDate>,
    pub to: Option<DicomDate>,
}

impl DicomDateRange {
    /// `true` when both endpoints are absent.
    pub fn is_null(self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    /// Parse a `DA` range. Both endpoints, or just one (open-ended), may be
    /// present; an empty value yields a null range.
    pub fn from_dicom(input: &[u8]) -> Result<Self> {
        let Some((left, right)) = split_range(input) else {
            return Ok(Self::default());
        };
        match right {
            Some(right) => {
                let from = parse_endpoint(left, parse_date_partial)?;
                let to = parse_endpoint(right, parse_date_partial)?;
                ensure!(from.is_some() || to.is_some(), InvalidData, "empty DICOM date range");
                Ok(Self { from, to })
            }
            None => {
                let value = parse_date_partial(left)?;
                Ok(Self { from: Some(value), to: Some(value) })
            }
        }
    }

    /// Append `from-to`. Either endpoint may be empty for open ranges.
    pub fn to_dicom(self, out: &mut Vec<u8>) {
        if self.is_null() {
            return;
        }
        if let Some(from) = self.from {
            from.to_dicom(out);
        }
        out.push(b'-');
        if let Some(to) = self.to {
            to.to_dicom(out);
        }
    }
}

/// A DICOM `TM` Range-Matching value (`t1 - t2`, `- t2` or `t1 -`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DicomTimeRange {
    pub from: Option<DicomTime>,
    pub to: Option<DicomTime>,
}

impl DicomTimeRange {
    /// `true` when both endpoints are absent.
    pub fn is_null(self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    /// Parse a `TM` range (see [`DicomDateRange::from_dicom`]).
    pub fn from_dicom(input: &[u8]) -> Result<Self> {
        let Some((left, right)) = split_range(input) else {
            return Ok(Self::default());
        };
        match right {
            Some(right) => {
                let from = parse_endpoint(left, DicomTime::from_dicom)?;
                let to = parse_endpoint(right, DicomTime::from_dicom)?;
                ensure!(from.is_some() || to.is_some(), InvalidData, "empty DICOM time range");
                Ok(Self { from, to })
            }
            None => {
                let value = DicomTime::from_dicom(left)?;
                Ok(Self { from: Some(value), to: Some(value) })
            }
        }
    }

    /// Append `from-to`.
    pub fn to_dicom(self, out: &mut Vec<u8>) {
        if self.is_null() {
            return;
        }
        if let Some(from) = self.from {
            from.to_dicom(out);
        }
        out.push(b'-');
        if let Some(to) = self.to {
            to.to_dicom(out);
        }
    }
}

/// A DICOM `DT` Range-Matching value (`dt1 - dt2`, `- dt2` or `dt1 -`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DicomDateTimeRange {
    pub from: Option<DicomDateTime>,
    pub to: Option<DicomDateTime>,
}

impl DicomDateTimeRange {
    /// `true` when both endpoints are absent.
    pub fn is_null(self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    /// Parse a `DT` range. Endpoints are parsed as C-FIND keys with the given
    /// dataset offset.
    pub fn from_dicom(input: &[u8], offset_in_dataset: Option<DicomTimeZoneOffset>) -> Result<Self> {
        let Some((left, right)) = split_range(input) else {
            return Ok(Self::default());
        };
        let parse = |b: &[u8]| DicomDateTime::from_dicom(b, true, offset_in_dataset);
        match right {
            Some(right) => {
                let from = parse_endpoint(left, parse)?;
                let to = parse_endpoint(right, parse)?;
                ensure!(from.is_some() || to.is_some(), InvalidData, "empty DICOM date-time range");
                Ok(Self { from, to })
            }
            None => {
                let value = parse(left)?;
                Ok(Self { from: Some(value), to: Some(value) })
            }
        }
    }

    /// Append the range, adjusting both endpoints to `offset_in_dataset`.
    pub fn to_dicom(
        self,
        out: &mut Vec<u8>,
        always_write_offset: bool,
        offset_in_dataset: Option<DicomTimeZoneOffset>,
    ) -> Result<()> {
        if self.is_null() {
            return Ok(());
        }
        if let Some(from) = self.from {
            let (adj, _, w) = from.adjust_to_offset(true, always_write_offset, offset_in_dataset)?;
            adj.write_body(out, w);
        }
        out.push(b'-');
        if let Some(to) = self.to {
            let (adj, _, w) = to.adjust_to_offset(true, always_write_offset, offset_in_dataset)?;
            adj.write_body(out, w);
        }
        Ok(())
    }
}

/// Parse a date with partial precision (for range endpoints), requiring the
/// whole slice to be consumed.
fn parse_date_partial(b: &[u8]) -> Result<DicomDate> {
    let (date, rest) = DicomDate::parse(b).ok_or_else(|| dicom_err!(InvalidData, "invalid DICOM date"))?;
    ensure!(rest.is_empty(), InvalidData, "trailing characters in DICOM date");
    Ok(date)
}

/// Parse one Range endpoint: `Ok(None)` for an empty half (open-ended),
/// `Ok(Some(value))` otherwise.
fn parse_endpoint<T>(b: &[u8], parse: impl Fn(&[u8]) -> Result<T>) -> Result<Option<T>> {
    let trimmed = rtrim(b);
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse(trimmed).map(Some)
    }
}

#[cfg(test)]
mod tests;
