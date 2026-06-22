//! Runtime representation of a single configuration value.
//!
//! A [`Value`] is the dynamically-typed payload stored against a configuration
//! [`Key`](super::Key). Its concrete shape is described and constrained by the
//! corresponding [`ValueMeta`](super::meta::ValueMeta) in the [`Registry`](super::registry::Registry).

use super::Config;
use crate::{Arc, Map};

/// Compile-time `Duration` of `n` seconds, for use as a `config!` default.
pub const fn secs(n: u64) -> std::time::Duration {
    std::time::Duration::from_secs(n)
}

/// Compile-time `Duration` of `n` milliseconds, for use as a `config!` default.
pub const fn millis(n: u64) -> std::time::Duration {
    std::time::Duration::from_millis(n)
}

/// Compile-time `Duration` of `n` minutes, for use as a `config!` default.
pub const fn mins(n: u64) -> std::time::Duration {
    std::time::Duration::from_secs(n * 60)
}

/// A file-valued configuration entry.
///
/// Some settings reference an external file either by path (optionally watched
/// for changes) or by inline content captured at load time.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValueFile {
    Name { path: String, hot_reload: bool },
    Content(Vec<u8>),
}

/// A dynamically-typed configuration value.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    String(String),
    Int(i64),
    Enum(u32),
    Duration(std::time::Duration),
    Tag(crate::tag::Tag),
    Vr(crate::vr::Vr),
    File(ValueFile),
    Network(crate::network::Network),
    Host(crate::network::Host),
    Object(Config),
    Vec(Vec<Value>),
    Map(Map<String, Value>),
    Complex(Arc<dyn std::any::Any + Send + Sync>),
}

impl Value {
    /// Returns a short, stable name of the value's variant, used in diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Int(_) => "Int",
            Value::Enum(_) => "Enum",
            Value::Duration(_) => "Duration",
            Value::Tag(_) => "Tag",
            Value::Vr(_) => "Vr",
            Value::File(_) => "File",
            Value::Network(_) => "Network",
            Value::Host(_) => "Host",
            Value::Object(_) => "Object",
            Value::Vec(_) => "Vec",
            Value::Map(_) => "Map",
            Value::Complex(_) => "Complex",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Int(i) => write!(f, "{}", i),
            Value::Enum(e) => write!(f, "{}", e),
            Value::Duration(d) => write!(f, "{} ms", d.as_millis()),
            Value::Tag(t) => write!(f, "{}", t),
            Value::Vr(v) => write!(f, "{}", v),
            Value::File(ValueFile::Name { path, .. }) => write!(f, "{}", path),
            Value::File(ValueFile::Content { 0: content }) => write!(f, "{} bytes", content.len()),
            Value::Network(network) => write!(f, "{}", network.definition),
            Value::Host(host) => write!(f, "{}", host.definition),
            Value::Object(_) => write!(f, "Object"),
            Value::Vec(vec) => write!(f, "{:?}", vec),
            Value::Map(map) => write!(f, "{:?}", map),
            Value::Complex(_) => write!(f, "Complex"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Enum(a), Value::Enum(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Tag(a), Value::Tag(b)) => a == b,
            (Value::Vr(a), Value::Vr(b)) => a == b,
            (Value::File(a), Value::File(b)) => a == b,
            (Value::Network(a), Value::Network(b)) => a == b,
            (Value::Host(a), Value::Host(b)) => a == b,
            (Value::Object(_), Value::Object(_)) => false,
            (Value::Vec(a), Value::Vec(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Complex(_), Value::Complex(_)) => false, // No equality for complex types
            _ => false,                                      // Different variants are not equal
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Enum(a), Value::Enum(b)) => a.partial_cmp(b),
            (Value::Duration(a), Value::Duration(b)) => a.partial_cmp(b),
            (Value::Tag(a), Value::Tag(b)) => a.partial_cmp(b),
            (Value::Vr(a), Value::Vr(b)) => a.partial_cmp(b),
            (Value::File(ValueFile::Content { 0: a }), Value::File(ValueFile::Content { 0: b })) => a.partial_cmp(b),
            (Value::File(ValueFile::Name { path: a, .. }), Value::File(ValueFile::Name { path: b, .. })) => {
                a.partial_cmp(b)
            }
            (Value::Network(a), Value::Network(b)) => a.partial_cmp(b),
            (Value::Host(a), Value::Host(b)) => a.partial_cmp(b),
            (Value::Object(_), Value::Object(_)) => None, // No natural ordering for objects
            (Value::Vec(_), Value::Vec(_)) => None,       // No natural ordering for vectors
            (Value::Map(_), Value::Map(_)) => None,       // No natural ordering for maps
            (Value::Complex(_), Value::Complex(_)) => None, // No natural ordering for complex types
            _ => None,                                    // Different variants are not comparable
        }
    }
}

/// Projects a borrowed [`Value`] into its Rust view type.
///
/// The associated [`Ref`](ValueRef::Ref) is the type a read yields: a `Copy`
/// scalar projects to itself, a heap value to a borrow.
pub trait ValueRef {
    type Ref<'a>;
    fn project(v: &Value) -> Option<Self::Ref<'_>>;
}

// ── Value::Null ──────────────────────────────────────────

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        match value {
            None => Value::Null,
            Some(v) => v.into(),
        }
    }
}

// ── Value::Bool ──────────────────────────────────────────

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl ValueRef for bool {
    type Ref<'a> = bool;
    fn project(v: &Value) -> Option<bool> {
        if let Value::Bool(b) = v { Some(*b) } else { None }
    }
}

// ── Value::Int ──────────────────────────────────────────

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl ValueRef for i64 {
    type Ref<'a> = i64;
    fn project(v: &Value) -> Option<i64> {
        if let Value::Int(n) = v { Some(*n) } else { None }
    }
}

// ── Value::String ──────────────────────────────────────────

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.into())
    }
}

impl ValueRef for String {
    type Ref<'a> = &'a str;
    fn project(v: &Value) -> Option<&str> {
        if let Value::String(s) = v {
            Some(s.as_str())
        } else {
            None
        }
    }
}

// ── Value::Duration ──────────────────────────────────────────

impl From<std::time::Duration> for Value {
    fn from(value: std::time::Duration) -> Self {
        Value::Duration(value)
    }
}

impl ValueRef for std::time::Duration {
    type Ref<'a> = std::time::Duration;
    fn project(v: &Value) -> Option<std::time::Duration> {
        if let Value::Duration(d) = v { Some(*d) } else { None }
    }
}

// ── Value::Tag ──────────────────────────────────────────

impl From<crate::Tag> for Value {
    fn from(value: crate::Tag) -> Self {
        Value::Tag(value)
    }
}

impl ValueRef for crate::Tag {
    type Ref<'a> = &'a crate::Tag;
    fn project(v: &Value) -> Option<&crate::Tag> {
        if let Value::Tag(d) = v { Some(d) } else { None }
    }
}

// ── Value::Vr ──────────────────────────────────────────

impl From<crate::Vr> for Value {
    fn from(value: crate::Vr) -> Self {
        Value::Vr(value)
    }
}

impl ValueRef for crate::Vr {
    type Ref<'a> = crate::Vr;
    fn project(v: &Value) -> Option<crate::Vr> {
        if let Value::Vr(d) = v { Some(*d) } else { None }
    }
}

// ── Value::Network ──────────────────────────────────────────

impl From<crate::network::Network> for Value {
    fn from(value: crate::network::Network) -> Self {
        Value::Network(value)
    }
}

impl ValueRef for crate::network::Network {
    type Ref<'a> = &'a crate::network::Network;
    fn project(v: &Value) -> Option<&crate::network::Network> {
        if let Value::Network(d) = v { Some(d) } else { None }
    }
}

// ── Value::Host ──────────────────────────────────────────

impl From<crate::network::Host> for Value {
    fn from(value: crate::network::Host) -> Self {
        Value::Host(value)
    }
}

impl ValueRef for crate::network::Host {
    type Ref<'a> = &'a crate::network::Host;
    fn project(v: &Value) -> Option<&crate::network::Host> {
        if let Value::Host(d) = v { Some(d) } else { None }
    }
}

// ── Value::Object ──────────────────────────────────────────

impl From<Config> for Value {
    fn from(value: Config) -> Self {
        Value::Object(value)
    }
}

impl ValueRef for Config {
    type Ref<'a> = &'a Config;
    fn project(v: &Value) -> Option<&Config> {
        if let Value::Object(c) = v { Some(c) } else { None }
    }
}

// ── Value::Vec ──────────────────────────────────────────

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::Vec(value.into_iter().map(|e| e.into()).collect())
    }
}

impl<const N: usize, T: Into<Value>> From<[T; N]> for Value {
    fn from(value: [T; N]) -> Self {
        Value::Vec(value.into_iter().map(|e| e.into()).collect())
    }
}

// impl<'a, T> From<&'a [T]> for Value where &'a T: Into<Value> {
//     fn from(value: &'a [T]) -> Self {
//         Value::Vec(value.iter().map(|e| e.into()).collect())
//     }
// }

impl<X: 'static> ValueRef for Vec<X> {
    type Ref<'a> = &'a [Value];
    fn project(v: &Value) -> Option<&[Value]> {
        if let Value::Vec(items) = v {
            Some(items.as_slice())
        } else {
            None
        }
    }
}
