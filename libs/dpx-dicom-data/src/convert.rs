//! Conversion between raw on-wire bytes and logical [`Value`]s, and between
//! [`Value`]s and typed Rust values.
//!
//! Decoding is driven by the element's `VR`: text VRs go through the dataset
//! charset, DA/TM/DT through the date/time parsers, IS/DS through ASCII number
//! parsing, and the binary numeric VRs through the dataset byte order.

use bytes::Bytes;
use dpx_dicom_core::vr::Kind;
use dpx_dicom_core::{DicomDate, DicomDateTime, DicomTime, Tag, Vr, dicom_err, ensure};
use dpx_dicom_core::error::Result;

use crate::dataset::{DatasetRole, Shared};
use crate::value::{OneOrMany, Value};

/// Extracts a typed value from a [`Value`]. `from_value` yields the first
/// value (the VM>1 read rule); `from_value_all` yields every value.
pub trait FromValue: Sized {
    fn from_value(v: &Value) -> Result<Self>;
    fn from_value_all(v: &Value) -> Result<Vec<Self>>;
}

/// Builds a [`Value`] from a typed value for storage.
pub trait IntoValue {
    fn into_value(self) -> Value;
}

fn rtrim(b: &[u8]) -> &[u8] {
    let mut end = b.len();
    while end > 0 && matches!(b[end - 1], b' ' | 0) {
        end -= 1;
    }
    &b[..end]
}

fn text_context(vr: Vr) -> dpx_dicom_charset::Context {
    dpx_dicom_charset::Context {
        is_multi_valued: !matches!(vr, Vr::LT | Vr::ST | Vr::UR | Vr::UT),
        is_pn: vr == Vr::PN,
    }
}

fn ints_value(v: Vec<i64>) -> Result<Value> {
    ensure!(!v.is_empty(), InvalidData, "empty numeric value");
    Ok(Value::Int(if v.len() == 1 {
        OneOrMany::One(v[0])
    } else {
        OneOrMany::Many(v)
    }))
}

fn uints_value(v: Vec<u64>) -> Result<Value> {
    ensure!(!v.is_empty(), InvalidData, "empty numeric value");
    Ok(Value::UInt(if v.len() == 1 {
        OneOrMany::One(v[0])
    } else {
        OneOrMany::Many(v)
    }))
}

fn floats_value(v: Vec<f64>) -> Result<Value> {
    ensure!(!v.is_empty(), InvalidData, "empty numeric value");
    Ok(Value::Float(if v.len() == 1 {
        OneOrMany::One(v[0])
    } else {
        OneOrMany::Many(v)
    }))
}

macro_rules! read_binary {
    ($name:ident, $t:ty, $w:expr, int) => {
        fn $name(bytes: &[u8], little_endian: bool) -> Result<Value> {
            ensure!(bytes.len().is_multiple_of($w), InvalidData, "value length is not a multiple of the element size");
            let mut out = Vec::with_capacity(bytes.len() / $w);
            for chunk in bytes.chunks_exact($w) {
                let mut a = [0u8; $w];
                a.copy_from_slice(chunk);
                let v = if little_endian { <$t>::from_le_bytes(a) } else { <$t>::from_be_bytes(a) };
                out.push(v as i64);
            }
            ints_value(out)
        }
    };
    ($name:ident, $t:ty, $w:expr, uint) => {
        fn $name(bytes: &[u8], little_endian: bool) -> Result<Value> {
            ensure!(bytes.len().is_multiple_of($w), InvalidData, "value length is not a multiple of the element size");
            let mut out = Vec::with_capacity(bytes.len() / $w);
            for chunk in bytes.chunks_exact($w) {
                let mut a = [0u8; $w];
                a.copy_from_slice(chunk);
                let v = if little_endian { <$t>::from_le_bytes(a) } else { <$t>::from_be_bytes(a) };
                out.push(v as u64);
            }
            uints_value(out)
        }
    };
    ($name:ident, $t:ty, $w:expr, float) => {
        fn $name(bytes: &[u8], little_endian: bool) -> Result<Value> {
            ensure!(bytes.len().is_multiple_of($w), InvalidData, "value length is not a multiple of the element size");
            let mut out = Vec::with_capacity(bytes.len() / $w);
            for chunk in bytes.chunks_exact($w) {
                let mut a = [0u8; $w];
                a.copy_from_slice(chunk);
                let v = if little_endian { <$t>::from_le_bytes(a) } else { <$t>::from_be_bytes(a) };
                out.push(v as f64);
            }
            floats_value(out)
        }
    };
}

read_binary!(read_u16, u16, 2, uint);
read_binary!(read_i16, i16, 2, int);
read_binary!(read_u32, u32, 4, uint);
read_binary!(read_i32, i32, 4, int);
read_binary!(read_u64, u64, 8, uint);
read_binary!(read_i64, i64, 8, int);
read_binary!(read_f32, f32, 4, float);
read_binary!(read_f64, f64, 8, float);

fn ints_from_text(bytes: &[u8]) -> Result<Value> {
    let s = std::str::from_utf8(bytes).map_err(|_| dicom_err!(InvalidData, "IS value is not ASCII"))?;
    let mut out = Vec::new();
    for part in s.split('\\') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        out.push(t.parse::<i64>().map_err(|_| dicom_err!(InvalidData, "invalid IS value"))?);
    }
    ints_value(out)
}

fn floats_from_text(bytes: &[u8]) -> Result<Value> {
    let s = std::str::from_utf8(bytes).map_err(|_| dicom_err!(InvalidData, "DS value is not ASCII"))?;
    let mut out = Vec::new();
    for part in s.split('\\') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        out.push(t.parse::<f64>().map_err(|_| dicom_err!(InvalidData, "invalid DS value"))?);
    }
    floats_value(out)
}

fn tags_from_bytes(bytes: &[u8], little_endian: bool) -> Result<Value> {
    ensure!(bytes.len().is_multiple_of(4), InvalidData, "AT value length is not a multiple of 4");
    let read_u16 = |b: &[u8]| -> u16 {
        let mut a = [0u8; 2];
        a.copy_from_slice(b);
        if little_endian { u16::from_le_bytes(a) } else { u16::from_be_bytes(a) }
    };
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        out.push(Tag::new_standard(read_u16(&chunk[0..2]), read_u16(&chunk[2..4])));
    }
    ensure!(!out.is_empty(), InvalidData, "empty AT value");
    Ok(Value::Tags(if out.len() == 1 {
        OneOrMany::One(out.swap_remove(0))
    } else {
        OneOrMany::Many(out)
    }))
}

/// Decodes raw text bytes to a string view for the fast `get_str` path,
/// borrowing when the charset allows it.
pub(crate) fn decode_str<'a>(shared: &Shared, vr: Vr, bytes: &'a [u8]) -> std::borrow::Cow<'a, str> {
    match vr.info().kind {
        Kind::Text { translatable: true, .. } => shared.charset().decode(bytes, &text_context(vr)),
        _ => String::from_utf8_lossy(rtrim(bytes)),
    }
}

/// Decodes raw on-wire bytes into a logical [`Value`] under the dataset context.
pub(crate) fn decode(shared: &Shared, vr: Vr, bytes: &[u8]) -> Result<Value> {
    match vr {
        Vr::DA => Ok(Value::Date(DicomDate::from_dicom(rtrim(bytes))?)),
        Vr::TM => Ok(Value::Time(DicomTime::from_dicom(rtrim(bytes))?)),
        Vr::DT => Ok(Value::DateTime(DicomDateTime::from_dicom(
            rtrim(bytes),
            matches!(shared.role(), DatasetRole::QueryRetrieve),
            Some(shared.effective_tz()),
        )?)),
        Vr::IS => ints_from_text(bytes),
        Vr::DS => floats_from_text(bytes),
        Vr::AT => tags_from_bytes(bytes, shared.is_little_endian()),
        _ => match vr.info().kind {
            Kind::Text { translatable, .. } => {
                let s = if translatable {
                    shared.charset().decode(bytes, &text_context(vr)).into_owned()
                } else {
                    String::from_utf8_lossy(rtrim(bytes)).into_owned()
                };
                Ok(Value::Str(s))
            }
            Kind::U16 => read_u16(bytes, shared.is_little_endian()),
            Kind::I16 => read_i16(bytes, shared.is_little_endian()),
            Kind::U32 => read_u32(bytes, shared.is_little_endian()),
            Kind::I32 => read_i32(bytes, shared.is_little_endian()),
            Kind::U64 => read_u64(bytes, shared.is_little_endian()),
            Kind::I64 => read_i64(bytes, shared.is_little_endian()),
            Kind::F32 => read_f32(bytes, shared.is_little_endian()),
            Kind::F64 => read_f64(bytes, shared.is_little_endian()),
            Kind::Bytes => Ok(Value::Bytes(Bytes::copy_from_slice(bytes))),
            Kind::Items | Kind::Invalid => Err(dicom_err!(InvalidData, "cannot decode VR {vr} from raw bytes")),
        },
    }
}

fn put_u16(out: &mut Vec<u8>, little_endian: bool, v: u16) {
    if little_endian { out.extend_from_slice(&v.to_le_bytes()) } else { out.extend_from_slice(&v.to_be_bytes()) }
}
fn put_u32(out: &mut Vec<u8>, little_endian: bool, v: u32) {
    if little_endian { out.extend_from_slice(&v.to_le_bytes()) } else { out.extend_from_slice(&v.to_be_bytes()) }
}
fn put_u64(out: &mut Vec<u8>, little_endian: bool, v: u64) {
    if little_endian { out.extend_from_slice(&v.to_le_bytes()) } else { out.extend_from_slice(&v.to_be_bytes()) }
}

/// A scalar that any numeric/textual [`Value`] can be coerced into: integer and
/// float variants by cast, `Str` by parsing each token. Implemented for the
/// integer and float primitives; the bound on the public `get_iter` accessor.
pub trait FromNumber: Sized {
    fn from_i64(v: i64) -> Self;
    fn from_u64(v: u64) -> Self;
    fn from_f64(v: f64) -> Self;
    fn parse(s: &str) -> Option<Self>;
}

macro_rules! from_number_int {
    ($($t:ty),*) => {$(
        impl FromNumber for $t {
            fn from_i64(v: i64) -> Self { v as $t }
            fn from_u64(v: u64) -> Self { v as $t }
            fn from_f64(v: f64) -> Self { v as $t }
            fn parse(s: &str) -> Option<Self> {
                // Tolerate a decimal point in a string targeting an integer VR.
                s.parse::<$t>().ok().or_else(|| s.parse::<f64>().ok().map(|f| f as $t))
            }
        }
    )*};
}
from_number_int!(i16, u16, i32, u32, i64, u64);

macro_rules! from_number_float {
    ($($t:ty),*) => {$(
        impl FromNumber for $t {
            fn from_i64(v: i64) -> Self { v as $t }
            fn from_u64(v: u64) -> Self { v as $t }
            fn from_f64(v: f64) -> Self { v as $t }
            fn parse(s: &str) -> Option<Self> { s.parse::<$t>().ok() }
        }
    )*};
}
from_number_float!(f32, f64);

/// Lazy, allocation-free coercion of a [`Value`]'s scalars to a stream of `T`,
/// used by [`encode`] (and reusable by callers). Numeric variants cast; `Str`
/// splits on `'\'` and parses each non-empty token, skipping any that fail.
pub(crate) struct Numbers<'a, T> {
    inner: NumbersInner<'a>,
    _marker: std::marker::PhantomData<T>,
}

enum NumbersInner<'a> {
    Ints(std::slice::Iter<'a, i64>),
    UInts(std::slice::Iter<'a, u64>),
    Floats(std::slice::Iter<'a, f64>),
    Text(std::str::Split<'a, char>),
    Empty,
}

pub(crate) fn numbers<T: FromNumber>(value: &Value) -> Numbers<'_, T> {
    let inner = match value {
        Value::Int(o) => NumbersInner::Ints(o.iter()),
        Value::UInt(o) => NumbersInner::UInts(o.iter()),
        Value::Float(o) => NumbersInner::Floats(o.iter()),
        Value::Str(s) => NumbersInner::Text(s.split('\\')),
        _ => NumbersInner::Empty,
    };
    Numbers { inner, _marker: std::marker::PhantomData }
}

impl<T: FromNumber> Iterator for Numbers<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match &mut self.inner {
            NumbersInner::Ints(it) => it.next().map(|v| T::from_i64(*v)),
            NumbersInner::UInts(it) => it.next().map(|v| T::from_u64(*v)),
            NumbersInner::Floats(it) => it.next().map(|v| T::from_f64(*v)),
            NumbersInner::Text(sp) => {
                for tok in sp.by_ref() {
                    let tok = tok.trim_matches([' ', '\0']);
                    if !tok.is_empty()
                        && let Some(v) = T::parse(tok)
                    {
                        return Some(v);
                    }
                }
                None
            }
            NumbersInner::Empty => None,
        }
    }
}

/// Owning coercion of a [`Value`]'s scalars to a stream of `T`. Unlike
/// [`Numbers`], this carries the [`Value`] itself, so it can be returned from an
/// accessor whose source value was decoded into a temporary (a `Mapped`/`Owned`
/// element). Coercion follows the same rules as [`numbers`].
pub(crate) struct OwnedNumbers<T> {
    value: Value,
    index: usize,
    _marker: std::marker::PhantomData<T>,
}

pub(crate) fn owned_numbers<T: FromNumber>(value: Value) -> OwnedNumbers<T> {
    OwnedNumbers { value, index: 0, _marker: std::marker::PhantomData }
}

impl<T: FromNumber> Iterator for OwnedNumbers<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match &self.value {
            Value::Int(o) => {
                let v = o.iter().nth(self.index)?;
                self.index += 1;
                Some(T::from_i64(*v))
            }
            Value::UInt(o) => {
                let v = o.iter().nth(self.index)?;
                self.index += 1;
                Some(T::from_u64(*v))
            }
            Value::Float(o) => {
                let v = o.iter().nth(self.index)?;
                self.index += 1;
                Some(T::from_f64(*v))
            }
            Value::Str(s) => {
                for tok in s.split('\\').skip(self.index) {
                    self.index += 1;
                    let tok = tok.trim_matches([' ', '\0']);
                    if !tok.is_empty()
                        && let Some(v) = T::parse(tok)
                    {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Encodes a logical [`Value`] to even-length on-wire bytes for `vr`, using the
/// dataset charset for translatable text and `order` (the *target* byte order)
/// for binary numerics, appended to `out`. The inverse of [`decode`]; `Pixels`
/// are written by the serializer.
pub(crate) fn encode(shared: &Shared, little_endian: bool, vr: Vr, value: &Value, out: &mut Vec<u8>) -> Result<()> {
    let start = out.len();
    match vr {
        Vr::DA | Vr::TM | Vr::DT => match value {
            Value::Date(d) => (*d).to_dicom(out),
            Value::Time(t) => (*t).to_dicom(out),
            Value::DateTime(dt) => (*dt).to_dicom(
                out,
                matches!(shared.role(), DatasetRole::QueryRetrieve),
                false,
                Some(shared.effective_tz()),
            )?,
            _ => {}
        },
        Vr::IS => write_numbers_text(out, numbers::<i64>(value)),
        Vr::DS => write_numbers_text(out, numbers::<f64>(value)),
        Vr::AT => {
            if let Value::Tags(o) = value {
                for t in o.iter() {
                    put_u16(out, little_endian, t.key.group());
                    put_u16(out, little_endian, t.key.element());
                }
            }
        }
        _ => match vr.info().kind {
            Kind::Text { translatable, .. } => {
                if let Value::Str(s) = value {
                    if translatable {
                        out.extend_from_slice(&shared.charset().encode(s, &text_context(vr)));
                    } else {
                        out.extend_from_slice(s.as_bytes());
                    }
                }
            }
            // Numbers are coerced from any numeric or string `Value` (e.g. a
            // string `"512"` into US, or a float into SS) by `numbers::<T>`.
            Kind::I16 => {
                for v in numbers::<i16>(value) {
                    put_u16(out, little_endian, v as u16);
                }
            }
            Kind::U16 => {
                for v in numbers::<u16>(value) {
                    put_u16(out, little_endian, v);
                }
            }
            Kind::I32 => {
                for v in numbers::<i32>(value) {
                    put_u32(out, little_endian, v as u32);
                }
            }
            Kind::U32 => {
                for v in numbers::<u32>(value) {
                    put_u32(out, little_endian, v);
                }
            }
            Kind::I64 => {
                for v in numbers::<i64>(value) {
                    put_u64(out, little_endian, v as u64);
                }
            }
            Kind::U64 => {
                for v in numbers::<u64>(value) {
                    put_u64(out, little_endian, v);
                }
            }
            Kind::F32 => {
                for v in numbers::<f32>(value) {
                    put_u32(out, little_endian, v.to_bits());
                }
            }
            Kind::F64 => {
                for v in numbers::<f64>(value) {
                    put_u64(out, little_endian, v.to_bits());
                }
            }
            Kind::Bytes => {
                if let Value::Bytes(b) = value {
                    out.extend_from_slice(b);
                }
            }
            Kind::Items | Kind::Invalid => {}
        },
    }
    if (out.len() - start) % 2 == 1 {
        // DICOM values are even-length: UI pads with NUL, other text with space,
        // binary/odd-byte values with NUL.
        let pad = if matches!(vr.info().kind, Kind::Text { .. }) && vr != Vr::UI { b' ' } else { 0 };
        out.push(pad);
    }
    Ok(())
}

/// Writes `'\'`-separated values formatted with `Display` straight into `out`
/// (no intermediate strings; `Vec<u8>` implements `io::Write`).
fn write_numbers_text<T: std::fmt::Display>(out: &mut Vec<u8>, values: impl Iterator<Item = T>) {
    use std::io::Write;
    for (i, v) in values.enumerate() {
        if i > 0 {
            out.push(b'\\');
        }
        let _ = write!(out, "{v}");
    }
}

fn first_text(s: &str) -> &str {
    s.split('\\').next().unwrap_or(s).trim_end_matches([' ', '\0'])
}

fn split_text(s: &str) -> Vec<String> {
    s.split('\\').map(|p| p.trim_end_matches([' ', '\0']).to_owned()).collect()
}

impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self> {
        match v {
            Value::Str(s) => Ok(first_text(s).to_owned()),
            _ => Err(dicom_err!(InvalidData, "value is not textual")),
        }
    }
    fn from_value_all(v: &Value) -> Result<Vec<Self>> {
        match v {
            Value::Str(s) => Ok(split_text(s)),
            _ => Err(dicom_err!(InvalidData, "value is not textual")),
        }
    }
}

impl FromValue for Bytes {
    fn from_value(v: &Value) -> Result<Self> {
        match v {
            Value::Bytes(b) => Ok(b.clone()),
            _ => Err(dicom_err!(InvalidData, "value is not a byte string")),
        }
    }
    fn from_value_all(v: &Value) -> Result<Vec<Self>> {
        Ok(vec![Self::from_value(v)?])
    }
}

macro_rules! from_integer {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(v: &Value) -> Result<Self> {
                let n: i128 = match v {
                    Value::Int(o) => *o.first() as i128,
                    Value::UInt(o) => *o.first() as i128,
                    _ => return Err(dicom_err!(InvalidData, "value is not an integer")),
                };
                <$t>::try_from(n).map_err(|_| dicom_err!(InvalidData, "integer value out of range"))
            }
            fn from_value_all(v: &Value) -> Result<Vec<Self>> {
                let conv = |n: i128| <$t>::try_from(n).map_err(|_| dicom_err!(InvalidData, "integer value out of range"));
                match v {
                    Value::Int(o) => o.iter().map(|x| conv(*x as i128)).collect(),
                    Value::UInt(o) => o.iter().map(|x| conv(*x as i128)).collect(),
                    _ => Err(dicom_err!(InvalidData, "value is not an integer")),
                }
            }
        }
    )*};
}
from_integer!(i64, i32, i16, u64, u32, u16, u8);

macro_rules! from_float {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(v: &Value) -> Result<Self> {
                match v {
                    Value::Float(o) => Ok(*o.first() as $t),
                    _ => Err(dicom_err!(InvalidData, "value is not a floating-point number")),
                }
            }
            fn from_value_all(v: &Value) -> Result<Vec<Self>> {
                match v {
                    Value::Float(o) => Ok(o.iter().map(|x| *x as $t).collect()),
                    _ => Err(dicom_err!(InvalidData, "value is not a floating-point number")),
                }
            }
        }
    )*};
}
from_float!(f64, f32);

macro_rules! from_datetime {
    ($($t:ty => $arm:ident),*) => {$(
        impl FromValue for $t {
            fn from_value(v: &Value) -> Result<Self> {
                match v {
                    Value::$arm(x) => Ok(*x),
                    _ => Err(dicom_err!(InvalidData, "value is not the expected date/time type")),
                }
            }
            fn from_value_all(v: &Value) -> Result<Vec<Self>> {
                Ok(vec![Self::from_value(v)?])
            }
        }
    )*};
}
from_datetime!(DicomDate => Date, DicomTime => Time, DicomDateTime => DateTime);

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Str(self)
    }
}
impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Str(self.to_owned())
    }
}
impl IntoValue for Bytes {
    fn into_value(self) -> Value {
        Value::Bytes(self)
    }
}
macro_rules! into_signed {
    ($($t:ty),*) => {$(
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::Int(OneOrMany::One(self as i64))
            }
        }
    )*};
}
into_signed!(i64, i32, i16);

macro_rules! into_unsigned {
    ($($t:ty),*) => {$(
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::UInt(OneOrMany::One(self as u64))
            }
        }
    )*};
}
into_unsigned!(u64, u32, u16, u8);

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::Float(OneOrMany::One(self))
    }
}
impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::Float(OneOrMany::One(self as f64))
    }
}

macro_rules! into_datetime {
    ($($t:ty => $arm:ident),*) => {$(
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::$arm(self)
            }
        }
    )*};
}
into_datetime!(DicomDate => Date, DicomTime => Time, DicomDateTime => DateTime);
