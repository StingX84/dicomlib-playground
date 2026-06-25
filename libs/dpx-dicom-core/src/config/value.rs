//! Runtime representation of a single configuration value.
//!
//! A [`Value`] is the dynamically-typed payload stored against a configuration
//! [`Key`](super::Key). Its concrete shape is described and constrained by the
//! corresponding [`ValueMeta`](super::meta::ValueMeta) in the [`ObjectMeta`](super::meta::ObjectMeta).

use super::{Object, meta::ConfigEnum, meta::Value};

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
pub enum File {
    Name { path: String, hot_reload: bool },
    Content(Vec<u8>),
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
            #[cfg(feature = "uuid")]
            Value::Uuid(v) => write!(f, "{}", v),
            Value::File(File::Name { path, .. }) => write!(f, "{}", path),
            Value::File(File::Content { 0: content }) => write!(f, "{} bytes", content.len()),
            Value::Network(network) => write!(f, "{}", network.definition),
            Value::Host(host) => write!(f, "{}", host.definition),
            Value::Object(_) => write!(f, "Object"),
            Value::Vec(vec) => write!(f, "{:?}", vec),
            Value::Map(map) => write!(f, "{:?}", map),
            #[cfg(feature = "serde")]
            Value::Custom(_) => write!(f, "Custom"),
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
            #[cfg(feature = "uuid")]
            (Value::Uuid(a), Value::Uuid(b)) => a == b,
            (Value::File(a), Value::File(b)) => a == b,
            (Value::Network(a), Value::Network(b)) => a == b,
            (Value::Host(a), Value::Host(b)) => a == b,
            (Value::Object(_), Value::Object(_)) => false,
            (Value::Vec(a), Value::Vec(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            #[cfg(feature = "serde")]
            (Value::Custom(_), Value::Custom(_)) => false, // No equality for custom types
            _ => false, // Different variants are not equal
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
            #[cfg(feature = "uuid")]
            (Value::Uuid(a), Value::Uuid(b)) => a.partial_cmp(b),
            (Value::File(File::Content { 0: a }), Value::File(File::Content { 0: b })) => a.partial_cmp(b),
            (Value::File(File::Name { path: a, .. }), Value::File(File::Name { path: b, .. })) => a.partial_cmp(b),
            (Value::Network(a), Value::Network(b)) => a.partial_cmp(b),
            (Value::Host(a), Value::Host(b)) => a.partial_cmp(b),
            (Value::Object(_), Value::Object(_)) => None, // No natural ordering for objects
            (Value::Vec(_), Value::Vec(_)) => None,       // No natural ordering for vectors
            (Value::Map(_), Value::Map(_)) => None,       // No natural ordering for maps
            #[cfg(feature = "serde")]
            (Value::Custom(_), Value::Custom(_)) => None, // No natural ordering for custom types
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

// ── Value::Enum ──────────────────────────────────────────

impl<T: ConfigEnum> From<&T> for Value {
    fn from(value: &T) -> Self {
        Value::Enum(value.as_u32())
    }
}

impl<T: ConfigEnum> ValueRef for T {
    type Ref<'a> = T;
    fn project(v: &Value) -> Option<T> {
        T::from_value(v)
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

// ── Value::Uuid ──────────────────────────────────────────

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for Value {
    fn from(value: uuid::Uuid) -> Self {
        Value::Uuid(value)
    }
}

#[cfg(feature = "uuid")]
impl ValueRef for uuid::Uuid {
    type Ref<'a> = uuid::Uuid;
    fn project(v: &Value) -> Option<uuid::Uuid> {
        if let Value::Uuid(d) = v { Some(*d) } else { None }
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

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        Value::Object(value)
    }
}

impl ValueRef for Object {
    type Ref<'a> = &'a Object;
    fn project(v: &Value) -> Option<&Object> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConfigValues, Key, Map, meta};
    use crate::{config_object_meta, declare_config_enums};

    declare_config_enums!(
        pub enum TestEnum {
            A,
            B,
            C,
        }
    );

    static ENUM_KEY: Key = Key::new("my_enum");
    static OBJECT_META: &[meta::KeyMeta] =
        &[
            meta::KeyMetaBuilder::new(ENUM_KEY, meta::build::Enum::new(TestEnum::CHOICES).build())
                .runtime()
                .build(),
        ];

    config_object_meta!( fn object_meta() = &OBJECT_META );

    #[test]
    fn can_set_and_get_enums() {
        let values = Map::from_iter([(ENUM_KEY, TestEnum::C.as_value())]);
        let mut object = Object::new(object_meta(), values);
        let value = object
            .config_get_as::<TestEnum>(&ENUM_KEY, None)
            .expect("should get enum value");
        assert!(value == TestEnum::C);

        object.values_mut().add(ENUM_KEY, TestEnum::B.as_value(), None);
        let value = object
            .config_get_as::<TestEnum>(&ENUM_KEY, None)
            .expect("should get enum value");
        assert!(value == TestEnum::B);

        object.values_mut().add(ENUM_KEY, 666.into(), None);
        assert!(
            object.config_get_as::<TestEnum>(&ENUM_KEY, None).is_none(),
            "should not get enum value for invalid int"
        );

        object.values_mut().add(ENUM_KEY, Value::Null, None);
        assert!(
            object.config_get_as::<TestEnum>(&ENUM_KEY, None).is_none(),
            "should not get enum value for null"
        );
    }

    #[cfg(all(test, feature = "uuid"))]
    #[test]
    fn uuid_values_compare_by_inner_value() {
        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        assert_eq!(Value::Uuid(a), Value::Uuid(a));
        assert_ne!(Value::Uuid(a), Value::Uuid(b));
        assert!(Value::Uuid(a) < Value::Uuid(b));
    }
}
