//! Configuration metadata: keys, value descriptors
//!
//! This is the vocabulary an application uses to *describe* its configuration
//! surface so it can be validated, edited in a GUI/TUI and documented without
//! hard-coding any particular key. The runtime payload is a
//! [`Value`]; the descriptor here is [`ValueMeta`].

use super::{Key, Value};

/// A statically- or dynamically-sourced list of choices.
///
/// Enumerations and "one of" constraints are often known at compile time, but
/// some (e.g. the set of supported character sets) are assembled at runtime.
#[derive(Debug, Clone)]
pub enum MaybeGenerated<T>
where
    T: 'static,
{
    Static(&'static [T]),
    Dynamic(fn() -> Box<dyn Iterator<Item = T>>),
}

impl<T: Clone> MaybeGenerated<T> {
    /// Iterates over the choices, materializing the dynamic variant on demand.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        match self {
            MaybeGenerated::Static(s) => Box::new(s.iter().cloned()),
            MaybeGenerated::Dynamic(f) => f(),
        }
    }
}

/// Human-facing identity of a key or an enum choice.
#[derive(Debug, Clone, Default)]
pub struct EditConcept {
    pub section: &'static str,
    pub is_advanced: bool,
    pub read_only: bool,
    pub name: EditName,
}

#[derive(Debug, Clone, Default)]
pub struct EditName {
    pub display_name: &'static str,
    pub brief: Option<&'static str>,
    pub help: Option<&'static str>,
}

/// Configuration file identity of a key or an enum choice.
#[derive(Debug, Clone, Default)]
pub struct StoreConcept {
    pub name: &'static str,
    pub conditional: bool,
}

#[derive(Debug, Clone)]
pub enum FileType {
    ExistingFilePath,
    ExistingDirPath,
    FilePath,
    DirPath,
    GlobPattern,
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: &'static str,
    pub edit: Option<EditName>,
    pub nullable: bool,
    pub default: Option<fn() -> Value>,
    pub value_meta: ValueMeta,
}

/// Describes the admissible shape and constraints of a [`Value`].
#[derive(Debug, Clone)]
pub enum ValueMeta {
    Bool,
    String {
        regexp: Option<&'static str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
        support_subst: bool, // Supports shell-like substitutions
    },
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },
    Enum {
        one_of: MaybeGenerated<(u32, &'static str, EditName)>,
    },
    Duration {
        min: Option<std::time::Duration>,
        max: Option<std::time::Duration>,
    },
    Tag {
        filter_by_vr: Option<&'static [crate::vr::Vr]>,
        one_of: Option<MaybeGenerated<crate::tag::Tag>>,
    },
    Vr {
        one_of: Option<MaybeGenerated<crate::vr::Vr>>,
    },
    File {
        ty: FileType,
        allow_relative: bool, // Allow relative path names
        allow_content: bool,  // Allow file content instead of file name
        allow_reload: bool,   // Allow user to request hot reload
    },
    Object {
        items: &'static [KeyMeta],
        validate: fn(&super::Config) -> crate::Result<()>,
    },
    Vec {
        items: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
        stride: Option<usize>,
    },
    Map {
        keys: &'static ValueMeta,
        values: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    Complex {
        ty: &'static dyn super::ComplexType,
        limits: &'static [(&'static str, &'static str)],
    },
}

impl ValueMeta {
    /// Returns a short, stable name of the value's variant, used in diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            ValueMeta::Bool => "Bool",
            ValueMeta::String { .. } => "String",
            ValueMeta::Int { .. } => "Int",
            ValueMeta::Enum { .. } => "Enum",
            ValueMeta::Duration { .. } => "Duration",
            ValueMeta::Tag { .. } => "Tag",
            ValueMeta::Vr { .. } => "Vr",
            ValueMeta::File { .. } => "File",
            ValueMeta::Object { .. } => "Object",
            ValueMeta::Vec { .. } => "Vec",
            ValueMeta::Map { .. } => "Map",
            ValueMeta::Complex { .. } => "Complex",
        }
    }
}

/// Full metadata for one configuration key.
#[derive(Debug, Clone)]
pub struct KeyMeta {
    pub key: Key,
    pub edit: Option<EditConcept>,
    pub store: Option<StoreConcept>,
    pub nullable: bool,
    pub default: Option<fn() -> Value>,
    pub value_meta: ValueMeta,
}

impl std::fmt::Display for KeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.store {
            write!(f, "{}", s.name)
        } else {
            write!(f, "{}-{}", self.key.module, self.key.code)
        }
    }
}
