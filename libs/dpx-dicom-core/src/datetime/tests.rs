//! Tests ported from the C++ reference `dicomdate.cpp`.
//!
//! Fractional seconds are adapted from the C++ millisecond model to
//! microseconds (e.g. ms literal `4` becomes `4000` us). Tests that depend on
//! the system timezone in C++ are exercised here against the deterministic
//! `Fixed` offset path, since `Local` resolution is deferred (see module).

use super::*;

fn tzp(h: i32, m: i32) -> DicomTimeZoneOffset {
    DicomTimeZoneOffset::Fixed(h * 3600 + m * 60)
}
fn tzm(h: i32, m: i32) -> DicomTimeZoneOffset {
    DicomTimeZoneOffset::Fixed(-(h * 3600 + m * 60))
}

fn date(y: u16, m: u8, d: u8) -> DicomDate {
    DicomDate { y: Some(y), m: Some(m), d: Some(d) }
}
fn time(h: u8, m: u8, s: u8, us: u32) -> DicomTime {
    DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: Some(us) }
}

fn write<F: FnOnce(&mut Vec<u8>)>(f: F) -> String {
    let mut v = Vec::new();
    f(&mut v);
    String::from_utf8(v).unwrap()
}

// ----------------------------------------------------------------------------
// DicomTimeZoneOffset
// ----------------------------------------------------------------------------

#[test]
fn tz_parse() {
    let ok: &[(&str, DicomTimeZoneOffset, bool)] = &[
        ("+0000", DicomTimeZoneOffset::Fixed(0), false),
        ("-0000", DicomTimeZoneOffset::Fixed(0), false),
        ("+0001", tzp(0, 1), false),
        ("-0001", tzm(0, 1), true),
        ("+0100", tzp(1, 0), false),
        ("-0100", tzm(1, 0), true),
        ("+0059", tzp(0, 59), false),
        ("-0059", tzm(0, 59), true),
        ("+1400", tzp(14, 0), false),
        ("-1200", tzm(12, 0), true),
        ("+1359", tzp(13, 59), false),
        ("-1159", tzm(11, 59), true),
    ];
    for (input, value, neg) in ok {
        let parsed = DicomTimeZoneOffset::from_dicom(input.as_bytes()).unwrap().unwrap();
        assert_eq!(parsed, *value, "{input}");
        assert!(parsed.is_valid(), "{input}");
        assert_eq!(parsed.is_negative(), *neg, "{input}");
    }

    assert!(DicomTimeZoneOffset::from_dicom(b"").unwrap().is_none());

    let bad = [
        "+0060", "-0060", "+1500", "-1300", "+1401", "-1201", "0000", "z0000", "-", "+", "+0", "+00", "+000",
        "+00000", "+0000a", "+000a", "+00a", "+0a", "+a",
    ];
    for input in bad {
        assert!(DicomTimeZoneOffset::from_dicom(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn tz_write() {
    let cases: &[(Option<DicomTimeZoneOffset>, &str)] = &[
        (None, ""),
        (Some(tzp(0, 0)), "+0000"),
        (Some(tzp(0, 1)), "+0001"),
        (Some(tzm(0, 1)), "-0001"),
        (Some(tzp(1, 0)), "+0100"),
        (Some(tzm(1, 0)), "-0100"),
        (Some(tzp(0, 59)), "+0059"),
        (Some(tzm(0, 59)), "-0059"),
        (Some(tzp(14, 0)), "+1400"),
        (Some(tzm(12, 0)), "-1200"),
        // Out-of-range offsets write nothing.
        (Some(tzp(14, 1)), ""),
        (Some(tzm(12, 1)), ""),
    ];
    for (input, expected) in cases {
        let got = write(|v| {
            if let Some(o) = input {
                o.to_dicom(v);
            }
        });
        assert_eq!(got, *expected);
    }
}

// ----------------------------------------------------------------------------
// DicomDate
// ----------------------------------------------------------------------------

#[test]
fn date_parse() {
    assert_eq!(DicomDate::from_dicom(b"").unwrap(), DicomDate::default());

    let ok: &[(&str, DicomDate)] = &[
        ("20010203", date(2001, 2, 3)),
        ("00010101", date(1, 1, 1)),
        ("00010101 ", date(1, 1, 1)),
        ("99991231", date(9999, 12, 31)),
        ("20240229", date(2024, 2, 29)),
    ];
    for (input, value) in ok {
        let parsed = DicomDate::from_dicom(input.as_bytes()).unwrap();
        assert_eq!(parsed, *value, "{input}");
        assert!(!parsed.is_null());
    }

    let bad = [
        " 00010101", "00000101", "00010001", "00010100", "00011301", "00010132", "20230229", "0001010", "000101",
        "00010", "0001", "000", "00", "0", "a", "000z0101", "00010z01", "0001010z", "00010101z", "000101+1",
        "000101-1", "0001+101", "0001-101", "+0010101", "-0010101",
    ];
    for input in bad {
        assert!(DicomDate::from_dicom(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn date_write() {
    let cases: &[(DicomDate, &str)] = &[
        (DicomDate::default(), ""),
        (DicomDate { y: Some(2001), m: None, d: None }, "20010101"),
        (DicomDate { y: Some(2001), m: Some(2), d: None }, "20010201"),
        (date(2001, 2, 3), "20010203"),
    ];
    for (input, expected) in cases {
        assert_eq!(write(|v| input.to_dicom(v)), *expected);
    }
}

// ----------------------------------------------------------------------------
// DicomDateRange
// ----------------------------------------------------------------------------

#[test]
fn date_range_parse() {
    assert_eq!(DicomDateRange::from_dicom(b"").unwrap(), DicomDateRange::default());

    let d = |y, m, d| Some(date(y, m, d));
    let dy = |y| Some(DicomDate { y: Some(y), m: None, d: None });
    let dym = |y, m| Some(DicomDate { y: Some(y), m: Some(m), d: None });

    let ok: &[(&str, DicomDateRange)] = &[
        ("20010203", DicomDateRange { from: d(2001, 2, 3), to: d(2001, 2, 3) }),
        ("20010203 ", DicomDateRange { from: d(2001, 2, 3), to: d(2001, 2, 3) }),
        ("-20010203", DicomDateRange { from: None, to: d(2001, 2, 3) }),
        ("20010203-", DicomDateRange { from: d(2001, 2, 3), to: None }),
        ("200102-", DicomDateRange { from: dym(2001, 2), to: None }),
        ("2001-", DicomDateRange { from: dy(2001), to: None }),
        ("2001- ", DicomDateRange { from: dy(2001), to: None }),
        ("-2001", DicomDateRange { from: None, to: dy(2001) }),
        ("-200102", DicomDateRange { from: None, to: dym(2001, 2) }),
        ("2001-2002", DicomDateRange { from: dy(2001), to: dy(2002) }),
        ("20010203-20040506", DicomDateRange { from: d(2001, 2, 3), to: d(2004, 5, 6) }),
    ];
    for (input, value) in ok {
        assert_eq!(DicomDateRange::from_dicom(input.as_bytes()).unwrap(), *value, "{input}");
    }

    let bad = ["-", "200z- ", "-200z", "-2001z", "20010203-20040506z", " 20010203-20040506"];
    for input in bad {
        assert!(DicomDateRange::from_dicom(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn date_range_write() {
    let d = |y, m, d| Some(date(y, m, d));
    let dy = |y| Some(DicomDate { y: Some(y), m: None, d: None });
    let cases: &[(DicomDateRange, &str)] = &[
        (DicomDateRange::default(), ""),
        (DicomDateRange { from: d(2001, 2, 3), to: d(2004, 5, 6) }, "20010203-20040506"),
        (DicomDateRange { from: dy(2001), to: dy(2001) }, "20010101-20010101"),
        (DicomDateRange { from: dy(2001), to: dy(2021) }, "20010101-20210101"),
        (DicomDateRange { from: dy(2001), to: None }, "20010101-"),
        (DicomDateRange { from: None, to: dy(2001) }, "-20010101"),
    ];
    for (input, expected) in cases {
        assert_eq!(write(|v| input.to_dicom(v)), *expected);
    }
}

// ----------------------------------------------------------------------------
// DicomTime
// ----------------------------------------------------------------------------

#[test]
fn time_parse() {
    assert_eq!(DicomTime::from_dicom(b"").unwrap(), DicomTime::default());

    let t = |h, m, s, us| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: Some(us) };
    let th = |h| DicomTime { h: Some(h), m: None, s: None, frac_us: None };
    let thm = |h, m| DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None };
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };

    let ok: &[(&str, DicomTime)] = &[
        ("010203.004000", t(1, 2, 3, 4000)),
        ("010203.000004", t(1, 2, 3, 4)),
        ("010203.00004", t(1, 2, 3, 40)),
        ("010203.0004", t(1, 2, 3, 400)),
        ("010203.004", t(1, 2, 3, 4000)),
        ("010203.04", t(1, 2, 3, 40000)),
        ("010203.4", t(1, 2, 3, 400000)),
        ("010203", ths(1, 2, 3)),
        ("0102", thm(1, 2)),
        ("01", th(1)),
        ("010203.4 ", t(1, 2, 3, 400000)),
    ];
    for (input, value) in ok {
        assert_eq!(DicomTime::from_dicom(input.as_bytes()).unwrap(), *value, "{input}");
    }

    let bad = [
        "010203.", "01020", "0102.3", "010", "01.2", "0", " 010203.4", "010203.z", "01020z.4", "010z03.4",
        "0z0203.4", "+10203.4", "-10203.4", "01+203.4", "01-203.4", "0102+3.4", "0102-3.4", "010203.+4", "010203.-4",
        "010203z4", "010203+4", "010203-4",
    ];
    for input in bad {
        assert!(DicomTime::from_dicom(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn time_leap_second() {
    // SS == 60 is read as 59.
    let parsed = DicomTime::from_dicom(b"235960").unwrap();
    assert_eq!(parsed, DicomTime { h: Some(23), m: Some(59), s: Some(59), frac_us: None });
}

#[test]
fn time_write() {
    let cases: &[(DicomTime, &str)] = &[
        (DicomTime::default(), ""),
        (DicomTime { h: Some(1), m: None, s: None, frac_us: None }, "01"),
        (DicomTime { h: Some(1), m: Some(2), s: None, frac_us: None }, "0102"),
        (DicomTime { h: Some(1), m: Some(2), s: Some(3), frac_us: None }, "010203"),
        (time(1, 2, 3, 4000), "010203.004000"),
        (time(23, 59, 59, 999999), "235959.999999"),
        // Zero fraction is not written.
        (time(1, 2, 3, 0), "010203"),
    ];
    for (input, expected) in cases {
        assert_eq!(write(|v| input.to_dicom(v)), *expected, "{input:?}");
    }
}

// ----------------------------------------------------------------------------
// DicomTimeRange
// ----------------------------------------------------------------------------

#[test]
fn time_range_parse() {
    assert_eq!(DicomTimeRange::from_dicom(b"").unwrap(), DicomTimeRange::default());

    let ths = |h, m, s| Some(DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None });
    let th = |h| Some(DicomTime { h: Some(h), m: None, s: None, frac_us: None });
    let thm = |h, m| Some(DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None });
    let tf = |h, m, s, us| Some(time(h, m, s, us));

    let ok: &[(&str, DicomTimeRange)] = &[
        ("010203", DicomTimeRange { from: ths(1, 2, 3), to: ths(1, 2, 3) }),
        ("-010203", DicomTimeRange { from: None, to: ths(1, 2, 3) }),
        ("010203-", DicomTimeRange { from: ths(1, 2, 3), to: None }),
        ("0102-", DicomTimeRange { from: thm(1, 2), to: None }),
        ("01-", DicomTimeRange { from: th(1), to: None }),
        ("01- ", DicomTimeRange { from: th(1), to: None }),
        ("-01", DicomTimeRange { from: None, to: th(1) }),
        ("-0102", DicomTimeRange { from: None, to: thm(1, 2) }),
        ("01-02", DicomTimeRange { from: th(1), to: th(2) }),
        ("010203-040506", DicomTimeRange { from: ths(1, 2, 3), to: ths(4, 5, 6) }),
        // ms literal 333 -> 333000 us, 444 -> 444000 us.
        ("010203.333-040506.444", DicomTimeRange { from: tf(1, 2, 3, 333000), to: tf(4, 5, 6, 444000) }),
    ];
    for (input, value) in ok {
        assert_eq!(DicomTimeRange::from_dicom(input.as_bytes()).unwrap(), *value, "{input}");
    }

    let bad = ["-", "0z- ", "-0z", "-01z", "010203-040506z", " 010203-040506"];
    for input in bad {
        assert!(DicomTimeRange::from_dicom(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn time_range_write() {
    let th = |h| Some(DicomTime { h: Some(h), m: None, s: None, frac_us: None });
    let thm = |h, m| Some(DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None });
    let ths = |h, m, s| Some(DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None });
    let tf = |h, m, s, us| Some(time(h, m, s, us));

    let cases: &[(DicomTimeRange, &str)] = &[
        (DicomTimeRange::default(), ""),
        (DicomTimeRange { from: tf(1, 2, 3, 4000), to: tf(5, 6, 7, 8000) }, "010203.004000-050607.008000"),
        (DicomTimeRange { from: th(1), to: th(1) }, "01-01"),
        (DicomTimeRange { from: th(1), to: th(5) }, "01-05"),
        (DicomTimeRange { from: thm(1, 2), to: th(5) }, "0102-05"),
        (DicomTimeRange { from: th(1), to: thm(5, 6) }, "01-0506"),
        (DicomTimeRange { from: ths(1, 2, 3), to: ths(5, 6, 7) }, "010203-050607"),
        (DicomTimeRange { from: th(1), to: None }, "01-"),
        (DicomTimeRange { from: None, to: th(1) }, "-01"),
    ];
    for (input, expected) in cases {
        assert_eq!(write(|v| input.to_dicom(v)), *expected);
    }
}

// ----------------------------------------------------------------------------
// DicomDateTime
// ----------------------------------------------------------------------------

fn dt(date: DicomDate, time: DicomTime, offset: Option<DicomTimeZoneOffset>) -> DicomDateTime {
    DicomDateTime { date, time, offset, tz_from_dataset: false }
}

#[test]
fn datetime_parse() {
    assert_eq!(DicomDateTime::from_dicom(b"", false, None).unwrap(), DicomDateTime::default());

    let dy = |y| DicomDate { y: Some(y), m: None, d: None };
    let dym = |y, m| DicomDate { y: Some(y), m: Some(m), d: None };
    let th = |h| DicomTime { h: Some(h), m: None, s: None, frac_us: None };
    let thm = |h, m| DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None };
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let empty = DicomTime::default();

    // (input, is_cfind, dataset_offset, expected_value)
    let parsed = DicomDateTime::from_dicom(b"20010203040506+0300", false, None).unwrap();
    assert_eq!(parsed, dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(3, 0))));

    // No offset in text -> adopt dataset offset, tz_from_dataset set.
    let parsed = DicomDateTime::from_dicom(b"20010203040506", false, Some(tzp(2, 0))).unwrap();
    assert_eq!(parsed, dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(2, 0))));
    assert!(parsed.tz_from_dataset);

    let parsed = DicomDateTime::from_dicom(b"20010203040506", false, None).unwrap();
    assert_eq!(parsed, dt(date(2001, 2, 3), ths(4, 5, 6), None));
    assert!(!parsed.tz_from_dataset);

    let parsed = DicomDateTime::from_dicom(b"00010203040506.007000+0809", false, None).unwrap();
    assert_eq!(parsed, dt(date(1, 2, 3), time(4, 5, 6, 7000), Some(tzp(8, 9))));

    let cases: &[(&str, DicomDateTime)] = &[
        ("00010203040506+0809", dt(date(1, 2, 3), ths(4, 5, 6), Some(tzp(8, 9)))),
        ("000102030405+0809", dt(date(1, 2, 3), thm(4, 5), Some(tzp(8, 9)))),
        ("0001020304+0809", dt(date(1, 2, 3), th(4), Some(tzp(8, 9)))),
        ("00010203+0809", dt(date(1, 2, 3), empty, Some(tzp(8, 9)))),
        ("000102+0809", dt(dym(1, 2), empty, Some(tzp(8, 9)))),
        ("0001+0809", dt(dy(1), empty, Some(tzp(8, 9)))),
        ("0001-0809", dt(dy(1), empty, Some(tzm(8, 9)))),
        ("0001", dt(dy(1), empty, None)),
        ("0001 ", dt(dy(1), empty, None)),
    ];
    for (input, value) in cases {
        assert_eq!(DicomDateTime::from_dicom(input.as_bytes(), false, None).unwrap(), *value, "{input}");
    }

    // C-FIND rejects a negative explicit offset.
    assert!(DicomDateTime::from_dicom(b"0001-0809", true, None).is_err());

    let bad = ["001", " 0001"];
    for input in bad {
        assert!(DicomDateTime::from_dicom(input.as_bytes(), false, None).is_err(), "{input}");
    }
}

#[test]
fn datetime_write() {
    let dy = |y| DicomDate { y: Some(y), m: None, d: None };
    let dym = |y, m| DicomDate { y: Some(y), m: Some(m), d: None };
    let th = |h| DicomTime { h: Some(h), m: None, s: None, frac_us: None };
    let thm = |h, m| DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None };
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let empty = DicomTime::default();

    // Cases that do not depend on the system timezone (deterministic).
    // (value, is_cfind, dataset_offset, expected)
    struct C(DicomDateTime, bool, Option<DicomTimeZoneOffset>, &'static str);
    let wd = |c: C| {
        let got = write(|v| c.0.to_dicom(v, c.1, false, c.2).unwrap());
        assert_eq!(got, c.3);
    };

    wd(C(DicomDateTime::default(), false, None, ""));
    wd(C(dt(DicomDate::default(), th(1), None), false, None, ""));
    wd(C(dt(dy(1), empty, None), false, None, "0001"));
    wd(C(dt(dym(1, 2), empty, None), false, None, "000102"));
    wd(C(dt(date(1, 2, 3), empty, None), false, None, "00010203"));
    wd(C(dt(date(1, 2, 3), th(4), None), false, None, "0001020304"));
    wd(C(dt(date(1, 2, 3), thm(4, 5), None), false, None, "000102030405"));
    wd(C(dt(date(1, 2, 3), ths(4, 5, 6), None), false, None, "00010203040506"));
    wd(C(dt(date(1, 2, 3), time(4, 5, 6, 7000), None), false, None, "00010203040506.007000"));
    // Explicit offset equal to dataset offset -> not written.
    wd(C(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(3, 0))), false, Some(tzp(3, 0)), "20010203040506"));
    // Explicit offset, no dataset -> written.
    wd(C(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(3, 0))), false, None, "20010203040506+0300"));
    // Explicit offset differing from dataset -> written (no adjustment: not from dataset).
    wd(C(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(2, 0))), false, Some(tzp(3, 0)), "20010203040506+0200"));
    wd(C(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzm(1, 0))), false, None, "20010203040506-0100"));
    // C-FIND with negative offset -> converted to UTC (+1h), +0000.
    wd(C(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzm(1, 0))), true, None, "20010203050506+0000"));
}

#[test]
fn datetime_adjust_from_dataset_offset() {
    // Value parsed with offset taken from dataset, then written for a different
    // dataset offset -> wall-clock shifts. +0200 dataset, written for +0300 ->
    // +1h.
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let mut value = dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(2, 0)));
    value.tz_from_dataset = true;
    let got = write(|v| value.to_dicom(v, false, false, Some(tzp(3, 0))).unwrap());
    assert_eq!(got, "20010203050506");
}

#[test]
fn datetime_adjust_across_day_boundary() {
    // 00:30 at +0000, taken from dataset, re-expressed for -0100 dataset offset
    // -> 23:30 previous day.
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let mut value = dt(date(2001, 2, 3), ths(0, 30, 0), Some(tzp(0, 0)));
    value.tz_from_dataset = true;
    let (adjusted, _, _) = value.adjust_to_offset(false, false, Some(tzm(1, 0))).unwrap();
    assert_eq!(adjusted.date, date(2001, 2, 2));
    assert_eq!(adjusted.time, ths(23, 30, 0));
    assert_eq!(adjusted.offset, Some(tzm(1, 0)));
}

// ----------------------------------------------------------------------------
// DicomDateTimeRange
// ----------------------------------------------------------------------------

#[test]
fn datetime_range_parse() {
    assert_eq!(DicomDateTimeRange::from_dicom(b"", None).unwrap(), DicomDateTimeRange::default());

    let dy = |y| DicomDate { y: Some(y), m: None, d: None };
    let dym = |y, m| DicomDate { y: Some(y), m: Some(m), d: None };
    let th = |h| DicomTime { h: Some(h), m: None, s: None, frac_us: None };
    let thm = |h, m| DicomTime { h: Some(h), m: Some(m), s: None, frac_us: None };
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let empty = DicomTime::default();

    let r = |from: Option<DicomDateTime>, to: Option<DicomDateTime>| DicomDateTimeRange { from, to };

    // Single value -> from == to.
    let parsed = DicomDateTimeRange::from_dicom(b"20010203", None).unwrap();
    let v = dt(date(2001, 2, 3), empty, None);
    assert_eq!(parsed, r(Some(v), Some(v)));

    assert_eq!(
        DicomDateTimeRange::from_dicom(b"-20010203", None).unwrap(),
        r(None, Some(dt(date(2001, 2, 3), empty, None)))
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"20010203040506.007+0809-", None).unwrap(),
        r(Some(dt(date(2001, 2, 3), time(4, 5, 6, 7000), Some(tzp(8, 9)))), None)
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"20010203040506-", None).unwrap(),
        r(Some(dt(date(2001, 2, 3), ths(4, 5, 6), None)), None)
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"20010203-", None).unwrap(),
        r(Some(dt(date(2001, 2, 3), empty, None)), None)
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"200102-", None).unwrap(),
        r(Some(dt(dym(2001, 2), empty, None)), None)
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"2001-", None).unwrap(),
        r(Some(dt(dy(2001), empty, None)), None)
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"-2001", None).unwrap(),
        r(None, Some(dt(dy(2001), empty, None)))
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"-2001020304", None).unwrap(),
        r(None, Some(dt(date(2001, 2, 3), th(4), None)))
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"-200102030405", None).unwrap(),
        r(None, Some(dt(date(2001, 2, 3), thm(4, 5), None)))
    );
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"-20010203040506.007+0809", None).unwrap(),
        r(None, Some(dt(date(2001, 2, 3), time(4, 5, 6, 7000), Some(tzp(8, 9)))))
    );

    // Dataset offset adopted on the open-from endpoint.
    assert_eq!(
        DicomDateTimeRange::from_dicom(b"20010203040506.007-", Some(tzp(8, 9))).unwrap().from.unwrap().offset,
        Some(tzp(8, 9))
    );

    assert!(DicomDateTimeRange::from_dicom(b"-", None).is_err());
}

#[test]
fn datetime_range_write() {
    let dy = |y| DicomDate { y: Some(y), m: None, d: None };
    let ths = |h, m, s| DicomTime { h: Some(h), m: Some(m), s: Some(s), frac_us: None };
    let empty = DicomTime::default();
    let r = |from: Option<DicomDateTime>, to: Option<DicomDateTime>| DicomDateTimeRange { from, to };

    let wd = |range: DicomDateTimeRange, ds: Option<DicomTimeZoneOffset>| {
        write(|v| range.to_dicom(v, false, ds).unwrap())
    };

    assert_eq!(wd(DicomDateTimeRange::default(), None), "");
    assert_eq!(wd(r(Some(dt(dy(2001), empty, None)), None), None), "2001-");
    // date year-only + time hour=2 -> date written in full padded form.
    let th2 = DicomTime { h: Some(2), m: None, s: None, frac_us: None };
    assert_eq!(wd(r(Some(dt(dy(2001), th2, None)), None), None), "2001010102-");
    assert_eq!(wd(r(None, Some(dt(dy(2001), empty, None))), None), "-2001");
    assert_eq!(wd(r(Some(dt(dy(2001), empty, None)), Some(dt(dy(2001), empty, None))), None), "2001-2001");
    assert_eq!(
        wd(
            r(
                Some(dt(date(2001, 2, 3), time(4, 5, 6, 7000), Some(tzp(8, 9)))),
                Some(dt(date(2001, 2, 3), time(4, 5, 6, 7000), Some(tzp(8, 9))))
            ),
            None
        ),
        "20010203040506.007000+0809-20010203040506.007000+0809"
    );
    assert_eq!(
        wd(r(Some(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzp(1, 2)))), None), None),
        "20010203040506+0102-"
    );
    // Negative offset in C-FIND range converted to UTC.
    assert_eq!(
        wd(r(Some(dt(date(2001, 2, 3), ths(4, 5, 6), Some(tzm(1, 2)))), None), None),
        "20010203050706+0000-"
    );
}

// ----------------------------------------------------------------------------
// minimized / maximized
// ----------------------------------------------------------------------------

#[test]
fn minimized_maximized() {
    let d = DicomDate { y: Some(2001), m: Some(2), d: None };
    assert_eq!(d.minimized(), date(2001, 2, 1));
    assert_eq!(d.maximized(), date(2001, 2, 28));

    let leap = DicomDate { y: Some(2024), m: Some(2), d: None };
    assert_eq!(leap.maximized(), date(2024, 2, 29));

    let yonly = DicomDate { y: Some(2001), m: None, d: None };
    assert_eq!(yonly.minimized(), date(2001, 1, 1));
    assert_eq!(yonly.maximized(), date(2001, 12, 31));

    let t = DicomTime { h: Some(23), m: Some(1), s: None, frac_us: None };
    assert_eq!(t.minimized(), time(23, 1, 0, 0));
    assert_eq!(t.maximized(), time(23, 1, 59, 999999));
}
