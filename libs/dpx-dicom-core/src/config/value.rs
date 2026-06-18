//! Runtime representation of a single configuration value.
//!
//! A [`Value`] is the dynamically-typed payload stored against a configuration
//! [`Key`](super::Key). Its concrete shape is described and constrained by the
//! corresponding [`ValueMeta`](super::meta::ValueMeta) in the [`Registry`](super::registry::Registry).

use crate::{Arc, Map};

/// A file-valued configuration entry.
///
/// Some settings reference an external file either by path (optionally watched
/// for changes) or by inline content captured at load time.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValueFile {
    Name { path: String, auto_reload: bool },
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
    Object(super::Config),
    Vec(Vec<Value>),
    Map(Map<Value, Value>),
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
            Value::Object(_) => "Object",
            Value::Vec(_) => "Vec",
            Value::Map(_) => "Map",
            Value::Complex(_) => "Complex",
        }
    }
}
