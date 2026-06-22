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

#[derive(Debug, Clone)]
pub enum ValueDefault {
    /// Uses the value type's zero/empty value as the default. Not checked
    /// against the meta's limits, so it may produce an out-of-range value.
    Default,
    /// Compile-time constant default value for the [`Key`].
    Static(Value),
    /// Computes the default during [`Registry`](super::Registry) construction;
    /// must return a value suitable for the [`Key`].
    Dynamic(fn(&super::Registry) -> Value),
}

/// Human-facing identity of a key or an enum choice.
#[derive(Debug, Clone, Default)]
pub struct EditConcept {
    /// Slash-separated section path in the configuration UI. Example: "DICOM/Network".
    pub section: Option<&'static str>,
    /// Hide this setting in the GUI by default, unless the user enables "Show Advanced Options".
    pub is_advanced: bool,
    /// Show the setting as read-only; the GUI must not let the user change it.
    pub read_only: bool,
    /// Setting name for display.
    pub name: EditName,
}

#[derive(Debug, Clone, Default)]
pub struct EditName {
    /// Short name. Example: "Listen Address"
    pub display_name: &'static str,
    /// One line brief. Example: "Accepts IPv4/IPv6 address or domain name with optional port"
    pub brief: Option<&'static str>,
    /// Long multiline help.
    pub help: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum FileType {
    ExistingFilePath,
    ExistingDirPath,
    FilePath,
    DirPath,
    GlobPattern,
}

/// Describes the admissible shape and constraints of a [`Value`].
#[derive(Debug, Clone)]
pub enum ValueMeta {
    Bool {
        nullable: bool,
    },
    String {
        regexp: Option<&'static str>,
        min: Option<usize>,
        max: Option<usize>,
        subst: bool, // Supports shell-like substitutions
        nullable: bool,
    },
    Int {
        min: Option<i64>,
        max: Option<i64>,
        subst: bool,
        nullable: bool,
    },
    Enum {
        one_of: MaybeGenerated<(u32, &'static str, EditName)>,
        subst: bool,
        nullable: bool,
    },
    Duration {
        min: Option<std::time::Duration>,
        max: Option<std::time::Duration>,
        subst: bool,
        nullable: bool,
    },
    Tag {
        filter_by_vr: Option<&'static [crate::vr::Vr]>,
        one_of: Option<MaybeGenerated<crate::tag::Tag>>,
        subst: bool,
        nullable: bool,
    },
    Vr {
        one_of: Option<MaybeGenerated<crate::vr::Vr>>,
        subst: bool,
        nullable: bool,
    },
    File {
        ty: FileType,
        allow_content: bool,  // Allow file content instead of file name
        hot_reload: bool,   // Allow user to request hot reload
        subst: bool,
        nullable: bool,
    },
    Network {
        domain: bool,
        unix: bool,
        v4: bool,
        v6: bool,
        subst: bool,
        nullable: bool,
    },
    Host {
        domain: bool,
        unix: bool,
        v4: bool,
        v6: bool,
        default_port: Option<u16>,
        subst: bool,
        nullable: bool,
    },
    Object {
        meta: &'static [KeyMeta],
        validate: fn(&super::Config) -> crate::Result<()>,
        nullable: bool,
    },
    Vec {
        items: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
        stride: Option<usize>,
        nullable: bool,
    },
    Map {
        values: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
        nullable: bool,
    },
    Complex {
        ty: &'static dyn super::ComplexType,
        limits: &'static [(&'static str, &'static str)],
        nullable: bool,
    },
}

impl ValueMeta {
    /// Returns a short, stable name of the value's variant, used in diagnostics.
    pub const fn kind_name(&self) -> &'static str {
        match self {
            ValueMeta::Bool { .. } => "Bool",
            ValueMeta::String { .. } => "String",
            ValueMeta::Int { .. } => "Int",
            ValueMeta::Enum { .. } => "Enum",
            ValueMeta::Duration { .. } => "Duration",
            ValueMeta::Tag { .. } => "Tag",
            ValueMeta::Vr { .. } => "Vr",
            ValueMeta::File { .. } => "File",
            ValueMeta::Network { .. } => "Network",
            ValueMeta::Host { .. } => "Host",
            ValueMeta::Object { .. } => "Object",
            ValueMeta::Vec { .. } => "Vec",
            ValueMeta::Map { .. } => "Map",
            ValueMeta::Complex { .. } => "Complex",
        }
    }

    pub const fn is_nullable(&self) -> bool {
        match self {
            ValueMeta::Bool { nullable } => *nullable,
            ValueMeta::String { nullable, .. } => *nullable,
            ValueMeta::Int { nullable, .. } => *nullable,
            ValueMeta::Enum { nullable, .. } => *nullable,
            ValueMeta::Duration { nullable, .. } => *nullable,
            ValueMeta::Tag { nullable, .. } => *nullable,
            ValueMeta::Vr { nullable, .. } => *nullable,
            ValueMeta::File { nullable, .. } => *nullable,
            ValueMeta::Network { nullable, .. } => *nullable,
            ValueMeta::Host { nullable, .. } => *nullable,
            ValueMeta::Object { nullable, .. } => *nullable,
            ValueMeta::Vec { nullable, .. } => *nullable,
            ValueMeta::Map { nullable, .. } => *nullable,
            ValueMeta::Complex { nullable, .. } => *nullable,
        }
    }

    pub const fn is_support_subst(&self) -> bool {
        match self {
            ValueMeta::Bool { .. } => false,
            ValueMeta::String { subst, .. } => *subst,
            ValueMeta::Int { subst, .. } => *subst,
            ValueMeta::Enum { subst, .. } => *subst,
            ValueMeta::Duration { subst, .. } => *subst,
            ValueMeta::Tag { subst, .. } => *subst,
            ValueMeta::Vr { subst, .. } => *subst,
            ValueMeta::File { subst, .. } => *subst,
            ValueMeta::Network { subst, .. } => *subst,
            ValueMeta::Host { subst, .. } => *subst,
            ValueMeta::Object { .. } => false,
            ValueMeta::Vec { .. } => false,
            ValueMeta::Map { .. } => false,
            ValueMeta::Complex { .. } => false,
        }
    }
}

/// Full metadata for one configuration key.
#[derive(Debug, Clone)]
pub struct KeyMeta {
    pub key: Key,
    pub edit: Option<EditConcept>,
    /// When `true`, the value is association-matched: it is stored as a
    /// `when`-filtered list entry and resolved against the active
    /// [`Condition`](super::Condition). When `false`, it is a plain
    /// value at the dotted path.
    pub conditional: bool,
    /// When `true`, the value lives only in memory (computed or set through the
    /// API) and the loader never reads or writes it. When `false`, it is
    /// persisted in the configuration file.
    pub runtime: bool,
    /// Produces the fallback value when no layer carries one. Mandatory: every
    /// registered key has a default, so a non-nullable key always resolves to a
    /// value. A nullable key with no meaningful default uses `ValueDefault::Default`,
    /// which yields `Value::Null`.
    pub default: ValueDefault,
    pub value_meta: ValueMeta,
}

impl std::fmt::Display for KeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key.0)
    }
}
