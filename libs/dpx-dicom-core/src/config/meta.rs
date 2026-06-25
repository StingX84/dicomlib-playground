//! Configuration metadata: keys, value descriptors
//!
//! This is the vocabulary an application uses to *describe* its configuration
//! surface so it can be validated, edited in a GUI/TUI and documented without
//! hard-coding any particular key. The runtime payload is a
//! [`Value`]; the descriptor here is [`ValueMeta`].

#![allow(clippy::new_without_default)]

use super::Key;
use std::{any::Any, sync::OnceLock};

// cspell:ignore rfield
macro_rules! declare_value_meta {
    ($(
        $(#[doc = $doc:tt])*
        $(#[cfg($cfg:meta)])*
        $name:ident { $(required: { $($rfield:ident : $rty:ty),* } $(,)?)? $(flags: { $($flag:ident),* } $(,)?)? $(limits: { $($field:ident : $ty:ty),* })? } = $native:ty),* $(,)
    ?) => {::place_macro::place! {
        /// Describes the type, constraints and flags of a [`Value`].
        #[derive(Debug, Clone)]
        pub enum ValueMeta {$(
            #[doc = __str__("Native: `" $native "`")]
            $(#[doc = $doc])*
            $(#[cfg($cfg)])*
            $name {
                $($($rfield: $rty,)*)?
                $($($flag: bool,)*)?
                $($($field: Option<$ty>,)*)?
                optional: bool
            },
        )*}
        impl ValueMeta {
            pub const fn kind_name(&self) -> &'static str {
                match self {
                    $($(#[cfg($cfg)])* Self::__ident__($name) { .. } => stringify!($name),)*
                }
            }
            pub const fn is_optional(&self) -> bool {
                match self {
                    $($(#[cfg($cfg)])* Self::__ident__($name) { optional, .. } => *optional,)*
                }
            }
            pub const fn is_support_subst(&self) -> bool {
                declare_value_meta!(@subst_match (self) () $( $(#[cfg($cfg)])* __ident__($name) [ $($($flag)*)? ] )* )
            }
        }
        #[derive(Debug, Clone)]
        pub enum Value {
            Null,
            $(
                $(#[doc = $doc])*
                $(#[cfg($cfg)])*
                __ident__($name)($native),
            )*
        }
        impl Value {
            pub const fn kind_name(&self) -> &'static str {
                match self {
                    Self::Null => "Null",
                    $($(#[cfg($cfg)])* Self::__ident__($name) { .. } => stringify!($name),)*
                }
            }
        }
        pub mod build {
            #![allow(unused_imports)]
            use super::*;
            pub trait Native {
               type T;
            }
            $(
                declare_value_meta!{@doc_selector,
                    concat!("Builder for [`ValueMeta::", stringify!($name), "`]"),
                    $(#[cfg($cfg)])*,
                pub struct __ident__($name) {
                    $($($rfield: $rty,)*)?
                    $($($flag: bool,)*)?
                    $($($field: Option<$ty>,)*)?
                    optional: bool,
                }
            }
            $(#[cfg($cfg)])*
            impl __ident__($name) {
                pub const fn new($($($rfield : $rty),*)?) -> Self {
                    Self {
                        $($($rfield,)*)?
                        $($($flag: false,)*)?
                        $($($field: None,)*)?
                        optional: false
                    }
                }
                $($(pub const fn $field(mut self, value: $ty) -> Self {
                    self.$field = Some(value);
                    self
                })*)?
                $($(pub const fn $flag(mut self) -> Self {
                    self.$flag = true;
                    self
                })*)?
                pub const fn optional(mut self) -> Self {
                    self.optional = true;
                    self
                }
                pub const fn build(self) -> super::ValueMeta {
                    super::ValueMeta::$name {
                        $($($rfield: self.$rfield,)*)?
                        $($($flag: self.$flag,)*)?
                        $($($field: self.$field,)*)?
                        optional: self.optional,
                    }
                }
            }
            $(#[cfg($cfg)])*
            impl Native for __ident__($name) { type T = $native; }
        )*}
    }};
    // Build the `is_support_subst` match by recursing over variants and their
    // flags, emitting a real arm only for variants that carry a `subst` flag.
    (@subst_match ($s:expr) ($($arm:tt)*)) => {
        match $s { $($arm)* #[allow(unreachable_patterns)] _ => false }
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)*) $($rest)*)
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ subst $($f:ident)* ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)* $(#[cfg($cfg)])* Self::$name { subst, .. } => *subst,) $($rest)*)
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ $other:ident $($f:ident)* ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)*) $(#[cfg($cfg)])* $name [ $($f)* ] $($rest)*)
    };

    (@doc_selector,$def_doc:expr,$(#[$inner:meta])+,$($c:tt)+) => { $(#[$inner])+ $($c)+ };
    (@doc_selector,$def_doc:expr,,$($c:tt)+) => { #[doc=$def_doc] $($c)+ };
}

declare_value_meta!(
    /// Boolean value, `true` or `false`.
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    Bool { } = bool,

    /// String value, UTF-8 text. See [`Value::String`].
    ///
    /// Flags:
    /// - `subst`: the string may contain `${...}` substitutions that are resolved during config read.
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `regexp`: the string must match this regex.
    /// - `min`: the string must be at least this many characters long.
    /// - `max`: the string must be at most this many characters long.
    String { flags: { subst }, limits: { regexp: &'static str, min: usize, max: usize } } = std::string::String,

    /// Integer value, 64-bit signed. See [`Value::Int`].
    ///
    /// Flags:
    /// - `subst`: the integer may be specified as a string with `${...}` substitutions that are resolved during config read.
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `min`: the integer must be at least this value.
    /// - `max`: the integer must be at most this value.
    Int { flags: { subst }, limits: { min: i64, max: i64 } } = i64,

    /// Enumeration value, one of a fixed set of choices. See [`Value::Enum`].
    ///
    /// Required:
    /// - `one_of`: the list of valid choices tuples:
    ///   - 0 : the integer value stored in the config file
    ///   - 1 : programmatic identity of the choice, used in code
    ///   - 2 : optional human-facing name of the choice, used in the GUI/TUI
    ///
    /// Flags:
    /// - `subst`: the enum may be specified as a string with `${...}` substitutions
    /// - `optional`: the value may be [`Value::Null`].
    Enum { required: { one_of: Choices<(u32, &'static str, Option<EnumVisual>)> }, flags: { subst } } = u32,

    /// Duration value, a time interval. See [`Value::Duration`].
    ///
    /// Flags:
    /// - `subst`: the duration may be specified as a string with `${...}` substitutions
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `min`: the duration must be at least this value.
    /// - `max`: the duration must be at most this value.
    Duration { flags: { subst }, limits: { min: std::time::Duration, max: std::time::Duration } } = std::time::Duration,

    /// DICOM Tag value with special formatting. See [`Value::Tag`].
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    Tag { limits: { one_of: Choices<crate::Tag> } } = crate::Tag,

    /// DICOM VR value with special formatting. See [`Value::Vr`].
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    Vr { limits: { one_of: Choices<crate::Vr> } } = crate::Vr,

    /// Universally unique identifier (UUID) value with special formatting. See [`Value::Uuid`].
    ///
    /// Flags:
    /// - `non_zero`: the UUID must not be the nil/zero UUID.
    /// - `subst`: the UUID may be specified as a string with `${...}` substitutions
    /// - `optional`: the value may be [`Value::Null`].
    #[cfg(feature = "uuid")]
    Uuid { flags: { non_zero, subst } } = uuid::Uuid,

    /// File path or content value. See [`Value::File`].
    ///
    /// Flags:
    /// - `allow_content`: the value may be specified as a file content blob instead of a path.
    /// - `allow_dir`: the file may be a directory.
    /// - `allow_file`: the file may be a regular file.
    /// - `allow_glob`: the file path may contain glob patterns.
    /// - `hot_reload`: the file is watched for changes and reloaded automatically.
    /// - `should_exist`: the file must exist when the config is loaded.
    /// - `should_not_exist`: the file must not exist when the config is loaded.
    /// - `subst`: the file path may contain `${...}` substitutions that are resolved during config read.
    /// - `optional`: the value may be [`Value::Null`].
    File { flags: { allow_content, allow_dir, allow_file, allow_glob, hot_reload, should_exist, should_not_exist, subst } } = crate::config::File,

    /// Network address with special formatting. See [`Value::Network`].
    ///
    /// Flags:
    /// - `domain`: the address may be specified as a domain name.
    /// - `unix`: the address may be specified as a Unix socket path.
    /// - `ipv4`: the address may be specified as an IPv4 address.
    /// - `ipv6`: the address may be specified as an IPv6 address.
    /// - `subst`: the address may be specified as a string with `${...}` substitutions
    /// - `optional`: the value may be [`Value::Null`].
    Network { flags: { domain, unix, ipv4, ipv6, subst } } = crate::network::Network,

    /// Hostname or IP address with optional port with special formatting. See [`Value::Host`].
    ///
    /// Flags:
    /// - `domain`: the host may be specified as a domain name.
    /// - `unix`: the host may be specified as a Unix socket path.
    /// - `ipv4`: the host may be specified as an IPv4 address.
    /// - `ipv6`: the host may be specified as an IPv6 address.
    /// - `subst`: the host may be specified as a string with `${...}` substitutions
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `default_port`: the default port to use if none is specified in the host string.
    Host { flags: { domain, unix, ipv4, ipv6, subst }, limits: { default_port: u16 } } = crate::network::Host,

    /// Object value, a nested configuration structure. See [`Value::Object`].
    ///
    /// Required:
    /// - `meta`: the metadata of the nested object.
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    Object { required: { meta: fn() -> &'static ObjectMeta } } = crate::config::Object,

    /// Vector value, a list of values of the same type. See [`Value::Vec`].
    ///
    /// Required:
    /// - `meta`: the metadata of the vector element type.
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `min`: the vector must have at least this many elements.
    /// - `max`: the vector must have at most this many elements.
    /// - `stride`: the vector must have a number of elements that is a multiple of
    Vec { required: { meta: &'static ValueMeta }, limits: { min: usize, max: usize, stride: usize } } = crate::Vec<crate::config::Value>,

    /// Map value, a dictionary of values of the same type. See [`Value::Map`].
    ///
    /// Required:
    /// - `meta`: the metadata of the map value type.
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    ///
    /// Limits:
    /// - `min`: the map must have at least this many entries.
    /// - `max`: the map must have at most this many entries.
    Map { required: { meta: &'static ValueMeta }, limits: { min: usize, max: usize } } = crate::Map<String, crate::config::Value>,
    /// Application-defined value, a type-erased value with a known type identity. See [`Value::Custom`].
    ///
    /// Required:
    /// - `ty`: the type identity of the custom value.
    ///
    /// Flags:
    /// - `optional`: the value may be [`Value::Null`].
    #[cfg(feature = "serde")]
    Custom { required: { ty: &'static dyn crate::config::CustomType} } = crate::Arc<dyn std::any::Any + Send + Sync>
);

/// A statically- or dynamically-sourced list of choices.
///
/// Enumerations and "one of" constraints are often known at compile time, but
/// some (e.g. the set of supported character sets) are assembled at runtime.
#[derive(Debug, Clone)]
pub enum Choices<T>
where
    T: 'static,
{
    Static(&'static [T]),
    Dynamic(fn() -> Box<dyn Iterator<Item = T>>),
}

impl<T: Clone> Choices<T> {
    /// Iterates over the choices, materializing the dynamic variant on demand.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        match self {
            Choices::Static(s) => Box::new(s.iter().cloned()),
            Choices::Dynamic(f) => f(),
        }
    }
}

pub trait ConfigEnum:
    Clone + Copy + PartialEq + Eq + std::fmt::Debug + std::fmt::Display + Into<Value> + Send + Sync + 'static
{
    const CHOICES: Choices<(u32, &'static str, Option<EnumVisual>)>;

    fn name(&self) -> &'static str;
    fn from_name(name: &str) -> Option<Self>;
    fn as_u32(&self) -> u32;
    fn from_u32(v: u32) -> Option<Self>;
    fn as_value(&self) -> crate::config::Value {
        crate::config::Value::Enum(self.as_u32())
    }
    fn from_value(v: &crate::config::Value) -> Option<Self> {
        match v {
            crate::config::Value::Enum(u) => Self::from_u32(*u),
            _ => None,
        }
    }
}

/// Human-facing identity of a key or an enum choice.
#[derive(Debug, Default, Clone)]
pub struct KeyVisual {
    /// Slash-separated section path in the configuration UI. Example: "DICOM/Network".
    pub section: Option<&'static str>,
    /// Short name. Example: "Listen Address"
    pub display_name: &'static str,
    /// One line brief. Example: "Accepts IPv4/IPv6 address or domain name with optional port"
    pub brief: Option<&'static str>,
    /// Long multiline help.
    pub help: Option<&'static str>,
    /// Hide this setting in the GUI by default, unless the user enables "Show Advanced Options".
    pub is_advanced: bool,
    /// Show the setting as read-only; the GUI must not let the user change it.
    pub read_only: bool,
}

#[derive(Debug, Default, Clone)]
pub struct EnumVisual {
    /// Short name. Example: "Lossless Compression Algorithm"
    pub display_name: &'static str,
    /// Long multiline help.
    pub help: Option<&'static str>,
}

/// Full metadata for one configuration key.
#[derive(Debug)]
pub struct KeyMeta {
    pub key: Key,
    pub visual: Option<KeyVisual>,
    /// When `true`, the value is association-matched: it is stored in YAML as a
    /// `when`-filtered list entry and resolved against the active
    /// [`Condition`](super::Condition). When `false`, it is a plain
    /// value at the dotted path.
    pub conditional: bool,
    /// When `true`, the value lives only in memory
    pub runtime: bool,
    pub default: Option<fn() -> Value>,
    pub value_meta: ValueMeta,
}

impl std::fmt::Display for KeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key.0)
    }
}

/// Metadata for a nested object value, which is a collection of keys with their own metadata.
///
/// Provides precomputed default values for all keys to allow for referenced retrieval of values
/// without needing to allocate or compute them on the fly.
#[derive(Debug)]
pub enum ObjectMeta {
    Single {
        meta: &'static [KeyMeta],
        defaults: Vec<(Key, Value)>,
    },
    Combined(Vec<&'static ObjectMeta>),
}

impl ObjectMeta {
    /// Creates a new `ObjectMeta` from a static slice of `KeyMeta`, precomputing default values for each key.
    ///
    /// Used to construct a global static entry.
    ///
    /// Example:
    /// ```rust
    /// # use dpx_dicom_core::{config_object_meta, config::Key, config::meta::{KeyMeta, KeyMetaBuilder, build}};
    /// static KEYS: &[KeyMeta] = &[
    ///     KeyMetaBuilder::new(Key::new("some_string"), build::String::new().build()).build(),
    ///     KeyMetaBuilder::new(Key::new("some_int"), build::Int::new().build()).build(),
    /// ];
    /// config_object_meta!( fn meta() = KEYS );
    /// ```
    pub fn new(meta: &'static [KeyMeta]) -> Self {
        let mut defaults = Vec::new();
        defaults.reserve_exact(meta.len());
        for key_meta in meta {
            let def_value = key_meta.default.map_or(Value::Null, |default_fn| default_fn());

            #[cfg(debug_assertions)]
            assert!(
                Self::fast_acceptance_test(&key_meta.value_meta, &def_value),
                "BUG! Default value {:?} is not compatible with its meta {:?}",
                def_value,
                key_meta.value_meta
            );

            defaults.push((key_meta.key, def_value));
        }
        Self::Single { meta, defaults }
    }

    /// Internal helper to create a combined `ObjectMeta` from all registered `ObjectMetaProvider` entries.
    pub(crate) fn new_collected() -> Self {
        let meta_vec = inventory::iter::<ObjectMetaProvider>
            .into_iter()
            .map(|r| r.0())
            .collect::<Vec<_>>();
        Self::Combined(meta_vec)
    }

    /// Returns the `KeyMeta` for a given key, if it exists in this `ObjectMeta`.
    pub fn key_meta(&self, key: &Key) -> Option<&KeyMeta> {
        match self {
            Self::Single { meta, .. } => meta.iter().find(|m| m.key == *key),
            Self::Combined(metas) => metas.iter().find_map(|m| m.key_meta(key)),
        }
    }

    /// Returns the `KeyMeta` for a given key, if it exists in this `ObjectMeta`.
    pub fn key_meta_str(&self, key: &str) -> Option<&KeyMeta> {
        match self {
            Self::Single { meta, .. } => meta.iter().find(|m| m.key.0 == key),
            Self::Combined(metas) => metas.iter().find_map(|m| m.key_meta_str(key)),
        }
    }

    pub fn iter(&self) -> ObjectMetaKeyIter<'_> {
        match self {
            Self::Single { meta, .. } => ObjectMetaKeyIter::Single(meta.iter()),
            Self::Combined(metas) => ObjectMetaKeyIter::Combined {
                obj_iters: metas.iter(),
                inner_iters: None,
            },
        }
    }

    /// Returns the default value for a given key, if it exists in this `ObjectMeta`.
    pub fn default_of(&self, key: &Key) -> Option<&Value> {
        match self {
            Self::Single { defaults, .. } => defaults.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            Self::Combined(metas) => metas.iter().find_map(|m| m.default_of(key)),
        }
    }

    /// Internal helper to check if default values are compatible with their metadata.
    /// Used only in debug assertion.
    #[cfg(debug_assertions)]
    fn fast_acceptance_test(value_meta: &ValueMeta, value: &Value) -> bool {
        match (value_meta, value) {
            (_, Value::Null) => true, // does not care about "optional" here, that is checked elsewhere
            (ValueMeta::Bool { .. }, Value::Bool(_)) => true,
            (ValueMeta::String { .. }, Value::String(_)) => true,
            (ValueMeta::Int { .. }, Value::Int(_)) => true,
            (ValueMeta::Enum { .. }, Value::Enum(_)) => true,
            (ValueMeta::Duration { .. }, Value::Duration(_)) => true,
            (ValueMeta::Tag { .. }, Value::Tag(_)) => true,
            (ValueMeta::Vr { .. }, Value::Vr(_)) => true,
            #[cfg(feature = "uuid")]
            (ValueMeta::Uuid { .. }, Value::Uuid(_)) => true,
            (ValueMeta::File { .. }, Value::File(_)) => true,
            (ValueMeta::Network { .. }, Value::Network(_)) => true,
            (ValueMeta::Host { .. }, Value::Host(_)) => true,
            (ValueMeta::Object { .. }, Value::Object(_)) => true,
            (ValueMeta::Vec { .. }, Value::Vec(_)) => true,
            (ValueMeta::Map { .. }, Value::Map(_)) => true,
            #[cfg(feature = "serde")]
            (ValueMeta::Custom { ty, .. }, Value::Custom(v)) => v.type_id() == ty.type_id(),
            #[cfg(feature = "serde")]
            (ValueMeta::Custom { .. }, _) => true, // Allow custom readers to produce any type
            _ => false,
        }
    }
}

pub enum ObjectMetaKeyIter<'a> {
    Single(std::slice::Iter<'a, KeyMeta>),
    Combined {
        obj_iters: std::slice::Iter<'a, &'a ObjectMeta>,
        inner_iters: Option<Box<ObjectMetaKeyIter<'a>>>,
    },
}
impl<'a> Iterator for ObjectMetaKeyIter<'a> {
    type Item = &'a KeyMeta;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(iter) => iter.next(),
            Self::Combined { obj_iters, inner_iters } => {
                if let Some(inner) = inner_iters.as_mut().and_then(|it| it.next()) {
                    Some(inner)
                } else {
                    *inner_iters = None;
                    for obj_meta in obj_iters.by_ref() {
                        let mut inner_iter = obj_meta.iter();
                        if let Some(inner) = inner_iter.next() {
                            *inner_iters = Some(Box::new(inner_iter));
                            return Some(inner);
                        }
                    }
                    None
                }
            }
        }
    }
}

/// A provider of `ObjectMeta` for global static registration.
/// Every submitted `ObjectMetaProvider` is collected into a global [`ObjectMeta`] by [`collected_global_meta()`].
///
/// Example:
/// ```rust
/// # use dpx_dicom_core::{config_object_meta, config::Key, config::meta::{KeyMeta, KeyMetaBuilder, build, ObjectMetaProvider}};
/// static APP_CONF_KEYS: &[KeyMeta] = &[
///     KeyMetaBuilder::new(Key::new("some_string"), build::String::new().build()).build(),
///     KeyMetaBuilder::new(Key::new("some_int"), build::Int::new().build()).build(),
/// ];
/// config_object_meta!( fn app_conf_meta() = APP_CONF_KEYS );
///
/// inventory::submit!( ObjectMetaProvider(app_conf_meta) );
/// ```
pub struct ObjectMetaProvider(pub fn() -> &'static ObjectMeta);
inventory::collect!(ObjectMetaProvider);

static GLOBAL_META: OnceLock<ObjectMeta> = OnceLock::new();
/// Provides a combined `ObjectMeta` from all registered `ObjectMetaProvider` entries.
///
/// This is a base for global configuration layer.
pub fn collected_global_meta() -> &'static ObjectMeta {
    GLOBAL_META.get_or_init(ObjectMeta::new_collected)
}

pub struct KeyMetaBuilder {
    key: Key,
    section: Option<&'static str>,
    display_name: Option<&'static str>,
    brief: Option<&'static str>,
    help: Option<&'static str>,
    is_advanced: bool,
    read_only: bool,
    conditional: bool,
    runtime: bool,
    default: Option<fn() -> Value>,
    value_meta: ValueMeta,
}
impl KeyMetaBuilder {
    pub const fn new(key: Key, value_meta: ValueMeta) -> Self {
        Self {
            key,
            section: None,
            display_name: None,
            brief: None,
            help: None,
            is_advanced: false,
            read_only: false,
            conditional: false,
            runtime: false,
            default: None,
            value_meta,
        }
    }
    pub const fn section(mut self, section: &'static str) -> Self {
        self.section = Some(section);
        self
    }
    pub const fn display_name(mut self, display_name: &'static str) -> Self {
        self.display_name = Some(display_name);
        self
    }
    pub const fn brief(mut self, brief: &'static str) -> Self {
        self.brief = Some(brief);
        self
    }
    pub const fn help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
    pub const fn is_advanced(mut self) -> Self {
        self.is_advanced = true;
        self
    }
    pub const fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
    pub const fn conditional(mut self) -> Self {
        self.conditional = true;
        self
    }
    pub const fn runtime(mut self) -> Self {
        self.runtime = true;
        self
    }
    pub const fn default(mut self, default: fn() -> Value) -> Self {
        self.default = Some(default);
        self
    }
    pub const fn build(self) -> KeyMeta {
        let visual = if let Some(display_name) = self.display_name {
            Some(KeyVisual {
                section: self.section,
                display_name,
                brief: self.brief,
                help: self.help,
                is_advanced: self.is_advanced,
                read_only: self.read_only,
            })
        } else {
            None
        };

        KeyMeta {
            key: self.key,
            visual,
            conditional: self.conditional,
            runtime: self.runtime,
            default: self.default,
            value_meta: self.value_meta,
        }
    }
}

pub struct EnumVisualBuilder {
    display_name: Option<&'static str>,
    help: Option<&'static str>,
}
impl EnumVisualBuilder {
    pub const fn new() -> Self {
        Self {
            display_name: None,
            help: None,
        }
    }
    pub const fn display_name(mut self, display_name: &'static str) -> Self {
        self.display_name = Some(display_name);
        self
    }
    pub const fn help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
    pub const fn build(self) -> Option<EnumVisual> {
        if let Some(display_name) = self.display_name {
            Some(EnumVisual {
                display_name,
                help: self.help,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_object_meta;

    static A_KEYS: &[KeyMeta] = &[
        KeyMetaBuilder::new(Key::new("a1"), build::Int::new().build())
            .default(|| Value::Int(1))
            .build(),
        KeyMetaBuilder::new(Key::new("a2"), build::Int::new().build())
            .default(|| Value::Int(2))
            .build(),
    ];
    static B_KEYS: &[KeyMeta] = &[KeyMetaBuilder::new(Key::new("b1"), build::Int::new().build())
        .default(|| Value::Int(3))
        .build()];
    static EMPTY_KEYS: &[KeyMeta] = &[];
    config_object_meta!( fn a_meta() = A_KEYS );
    config_object_meta!( fn b_meta() = B_KEYS );
    config_object_meta!( fn empty_meta() = EMPTY_KEYS );

    #[test]
    fn combined_meta_flattens_and_resolves_across_parts() {
        // Empty parts interleaved to exercise the iterator's skip-empty path.
        let combined = ObjectMeta::Combined(vec![empty_meta(), a_meta(), empty_meta(), b_meta()]);

        let keys: Vec<Key> = combined.iter().map(|km| km.key).collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&Key::new("a1")));
        assert!(keys.contains(&Key::new("a2")));
        assert!(keys.contains(&Key::new("b1")));

        // Defaults resolve from either part; unknown keys yield nothing.
        assert_eq!(combined.default_of(&Key::new("a1")), Some(&Value::Int(1)));
        assert_eq!(combined.default_of(&Key::new("b1")), Some(&Value::Int(3)));
        assert_eq!(combined.default_of(&Key::new("nope")), None);

        // Lookups search across every part.
        assert!(combined.key_meta(&Key::new("b1")).is_some());
        assert!(combined.key_meta_str("a2").is_some());
        assert!(combined.key_meta(&Key::new("nope")).is_none());
    }

    #[test]
    fn test_if_no_duplicate_keys_in_combined_meta() {
        fn check_value_meta(value_meta: &ValueMeta) {
            match &value_meta {
                ValueMeta::Object { meta, .. } => {
                    check_object_meta(meta());
                }
                ValueMeta::Vec { meta, .. } => {
                    check_value_meta(meta);
                }
                ValueMeta::Map { meta, .. } => {
                    check_value_meta(meta);
                }
                _ => {}
            }
        }

        fn check_object_meta(meta: &ObjectMeta) {
            fn check_single_meta(meta: &ObjectMeta, seen_keys: &mut std::collections::HashSet<Key>) {
                match meta {
                    ObjectMeta::Single {
                        meta: key_meta_list, ..
                    } => {
                        for key_meta in *key_meta_list {
                            assert!(
                                seen_keys.insert(key_meta.key),
                                "Duplicate key found: {:?}",
                                key_meta.key
                            );
                            check_value_meta(&key_meta.value_meta);
                        }
                    }
                    ObjectMeta::Combined(metas) => {
                        for m in metas {
                            check_single_meta(m, seen_keys);
                        }
                    }
                }
            }

            let mut seen_keys = std::collections::HashSet::new();
            check_single_meta(meta, &mut seen_keys);
        }

        check_object_meta(collected_global_meta());
    }

    #[test]
    fn test_conflicting_flags() {
        fn check_value_meta(key_meta: &KeyMeta, value_meta: &ValueMeta) {
            match &value_meta {
                ValueMeta::String {
                    min: Some(min),
                    max: Some(max),
                    ..
                } => {
                    assert!(
                        min <= max,
                        "String min > max: {} > {} in {}",
                        min,
                        max,
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::Int {
                    min: Some(min),
                    max: Some(max),
                    ..
                } => {
                    assert!(
                        min <= max,
                        "Int min > max: {} > {} in {}",
                        min,
                        max,
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::Duration {
                    min: Some(min),
                    max: Some(max),
                    ..
                } => {
                    assert!(
                        min <= max,
                        "Duration min > max: {:?} > {:?} in {}",
                        min,
                        max,
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::File {
                    allow_content,
                    allow_dir,
                    allow_file,
                    allow_glob,
                    should_exist,
                    should_not_exist,
                    ..
                } => {
                    assert!(
                        *allow_content || *allow_dir || *allow_file,
                        "File must have at least one of allow_content/allow_dir/allow_file in {}",
                        key_meta.key.as_str()
                    );
                    assert!(
                        !*should_exist || *allow_dir || *allow_file,
                        "File with should_exist must have at least one of allow_dir/allow_file in {}",
                        key_meta.key.as_str()
                    );
                    assert!(
                        !*should_not_exist || *allow_dir || *allow_file,
                        "File with should_not_exist must have at least one of allow_dir/allow_file in {}",
                        key_meta.key.as_str()
                    );
                    assert!(
                        !*allow_glob || *allow_file || *allow_dir,
                        "File with allow_glob must have at least one of allow_file or allow_dir in {}",
                        key_meta.key.as_str()
                    );
                    assert!(
                        !*should_exist || !*should_not_exist,
                        "File cannot have both should_exist and should_not_exist in {}",
                        key_meta.key.as_str()
                    );
                    assert!(
                        !*allow_glob || !*should_not_exist,
                        "File with allow_glob cannot have should_not_exist in {}",
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::Network {
                    domain,
                    unix,
                    ipv4,
                    ipv6,
                    ..
                } => {
                    assert!(
                        *domain || *unix || *ipv4 || *ipv6,
                        "Network must have at least one of domain/unix/ipv4/ipv6 in {}",
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::Host {
                    domain,
                    unix,
                    ipv4,
                    ipv6,
                    ..
                } => {
                    assert!(
                        *domain || *unix || *ipv4 || *ipv6,
                        "Host must have at least one of domain/unix/ipv4/ipv6 in {}",
                        key_meta.key.as_str()
                    );
                }
                ValueMeta::Object { meta, .. } => check_object_meta(meta()),
                ValueMeta::Vec {
                    meta,
                    min,
                    max,
                    stride: stripe,
                    ..
                } => {
                    if let Some(min) = min
                        && let Some(max) = max
                        && min > max
                    {
                        panic!("Vec min > max: {} > {} in {}", min, max, key_meta.key.as_str());
                    }
                    if let Some(stripe) = stripe
                        && *stripe == 0
                    {
                        panic!("Vec stripe must be > 0 in {}", key_meta.key.as_str());
                    }
                    if let Some(stripe) = stripe
                        && let Some(max) = max
                        && *stripe > *max
                    {
                        panic!("Vec stripe > max: {} > {} in {}", stripe, max, key_meta.key.as_str());
                    }
                    // stripe = 5, min = 8, max = 9  - impossible
                    if let Some(stripe) = stripe
                        && let Some(min) = min
                        && let Some(max) = max
                    {
                        let min_stripe = min.div_ceil(*stripe) * stripe;
                        if min_stripe > *max {
                            panic!(
                                "Vec impossible constraints: min {} rounded to stripe {} > max {} in {}",
                                min,
                                min_stripe,
                                max,
                                key_meta.key.as_str()
                            );
                        }
                    }
                    check_value_meta(key_meta, meta);
                }
                ValueMeta::Map { meta, min, max, .. } => {
                    if let Some(min) = min
                        && let Some(max) = max
                        && min > max
                    {
                        panic!("Map min > max: {} > {} in {}", min, max, key_meta.key.as_str());
                    }
                    check_value_meta(key_meta, meta);
                }
                _ => {}
            }
        }
        fn check_object_meta(meta: &ObjectMeta) {
            match meta {
                ObjectMeta::Single { meta, .. } => {
                    for key_meta in *meta {
                        check_value_meta(key_meta, &key_meta.value_meta);
                    }
                }
                ObjectMeta::Combined(metas) => {
                    for m in metas {
                        check_object_meta(m);
                    }
                }
            }
        }

        check_object_meta(collected_global_meta());
    }
}
