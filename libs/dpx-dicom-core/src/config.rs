//! Application configuration.
//!
//! Declare your settings once — with their types, defaults, constraints and
//! documentation — then read them back type-safely, whether a value comes from
//! a config file, a runtime override, or its built-in default.
//!
//! # Using it
//!
//! 1. **Declare** settings with [`declare_config_objects!`](crate::declare_config_objects)
//!    (and [`declare_config_enums!`](crate::declare_config_enums) for enum-valued
//!    keys). Every leaf key becomes a typed [`Key`] constant grouped in a module;
//!    objects marked `#[root]` are wired into the process-wide configuration
//!    automatically.
//! 2. **Read** values through the [`ConfigValues`] trait. Call it on the current
//!    [`Context`](crate::Context) to honour the active configuration layers and
//!    association, or on any [`Object`] directly.
//!    [`value`](ConfigValues::value) returns the value or its default;
//!    [`some_value`](ConfigValues::some_value) is the non-panicking variant.
//! 3. **Load and publish** files with [`YamlLoader`] and [`GlobalConfig`], the
//!    single source of truth for the application-wide configuration.
//!
//! A required key (`Key<T>`) reads back as `T`; an optional key
//! (`Key<Option<T>>`) reads back as `Option<T>`. Nested objects
//! ([`Value::Object`]) are read by chaining: `cfg.value(parent).value(child)`.
//! Application-defined value types are supported via
//! [`CustomType`](custom::CustomType)/[`Serde`](custom::Serde).
//!
//! ```
//! # use dpx_dicom_core::{declare_config_objects, config::{ConfigValues, Object}};
//! declare_config_objects! {
//!     pub settings {
//!         /// Maximum PDU size, in bytes.
//!         max_pdu: Int = 16384,
//!         /// Optional human-readable label.
//!         label: String(optional),
//!     }
//! }
//!
//! let cfg = Object::new_empty(settings::object_meta());
//! assert_eq!(cfg.value(settings::max_pdu), 16384); // built-in default
//! assert_eq!(cfg.value(settings::label), None);    // optional, unset
//! ```
//!
//! # Module map
//!
//! - [`macros`] — the [`declare_config_objects!`](crate::declare_config_objects)
//!   and [`declare_config_enums!`](crate::declare_config_enums) declaration macros.
//! - [`meta`] — descriptors ([`ObjectMeta`](meta::ObjectMeta) /
//!   [`KeyMeta`](meta::KeyMeta)) that let any key be validated, edited in a
//!   GUI/TUI and documented without hard-coding it.
//! - [`value`] — the dynamically-typed [`Value`] payloads and their projection
//!   into Rust types ([`ValueRef`]).
//! - [`Object`] — one configuration unit (an [`ObjectMeta`](meta::ObjectMeta)
//!   plus its values); composable into a [`GlobalConfig`], installable into a
//!   [`Context`](crate::Context), or nested as a [`Value::Object`].
//! - [`global`] — [`GlobalConfig`], the process-wide source of truth.
//! - [`loader`] — a [`serde`] deserializer that reads a config file into an [`Object`].
//! - [`subst`] — `${...}` variable substitution applied while reading values.

#[cfg(feature = "serde")]
pub mod custom;
pub mod global;
#[cfg(feature = "serde")]
pub mod loader;
pub mod macros;
pub(crate) mod map;
pub mod meta;
pub mod subst;
pub mod validator;
pub mod value;

use std::borrow::Cow;

// Hoist the types used across many call sites: reading and writing values,
// loading config files, and swapping the global configuration. Specialized,
// rarely-named pieces stay reachable through their submodule path — custom value
// adapters in [`custom`], the [`ValueRef`](value::ValueRef) projection in
// [`value`], and the whole descriptor surface in [`meta`].
pub use global::GlobalConfig;
#[cfg(feature = "serde")]
pub use loader::YamlLoader;
pub use map::Condition;
pub(crate) use map::ValueStore;
pub use meta::Value;
pub use subst::{AppDir, SubstVars};
pub use value::{File, millis, mins, secs};

pub(crate) use value::ValueRef;

use crate::network::AssocDescription;
use std::marker::PhantomData;

pub const GLOBAL_LAYER_ID: LayerId = LayerId::Borrowed("<global>");

/// Uniquely identifies a configuration key.
///
/// The wrapped string is the key's dotted store path (e.g.
/// `"dicom.association.artim_timeout"`). For
/// [`runtime`](meta::KeyMeta::runtime) keys it is purely an in-memory identity
/// that never appears in a file; for a field of an
/// [`Object`](meta::ValueMeta::Object) it is the local field name.
/// Within one registry scope the string must be unique.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct KeyId<'a>(pub &'a str);

impl<'a> KeyId<'a> {
    #[inline]
    pub const fn new(name: &'a str) -> KeyId<'a> {
        KeyId(name)
    }

    /// The key's path/identity string.
    #[inline]
    pub const fn as_str(&self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for KeyId<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl<'a> std::fmt::Display for KeyId<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// This fixes a lifetime issue with the HashMap in ValueStore.
// The KeyId is 'static, but the Value and Condition are not
// necessarily 'static. By wrapping them in a Vec, we can store them without
// needing to specify a lifetime for the entire ValueStore.
impl std::borrow::Borrow<str> for KeyId<'_> {
    fn borrow(&self) -> &str {
        self.0
    }
}

/// Maps a key's *declared* type to how a read projects and what it returns.
pub trait TypedKey: Clone + std::fmt::Debug + Send + Sync {
    type Out<'a>;
    type SomeOut<'a>;
    fn extract<'a, C: ConfigValues>(&self, values: &'a C, assoc: Option<&AssocDescription>) -> Self::Out<'a>;
    fn extract_some<'a, C: ConfigValues>(&self, values: &'a C, assoc: Option<&AssocDescription>) -> Self::SomeOut<'a>;
}

/// A typed handle to a single configuration key.
///
/// Pairs a [`KeyId`] with the key's declared Rust type `T`, which is
/// phantom (carried only at compile time). The type parameter drives how
/// reads project and what they return:
///
/// - `Key<T>` is a **required** key: [`value`](ConfigValues::value) yields
///   the bare `T` and panics if no value or default is present.
/// - `Key<Option<T>>` is an **optional** key: the same read yields
///   `Option<T>`, with `None` standing in for a missing value.
///
/// These constants are normally generated by the config-declaration macros,
/// one per leaf key. See [`TypedKey`] for the projection rules.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key<'a, T> {
    pub id: KeyId<'a>,
    _p: PhantomData<fn() -> T>,
}
impl<'a, T> Key<'a, T> {
    pub const fn new(id: KeyId<'a>) -> Self {
        Key { id, _p: PhantomData }
    }
}
impl<'a, T> std::fmt::Display for Key<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T: ValueRef> TypedKey for Key<'_, Option<T>> {
    type Out<'a> = Option<<T as ValueRef>::Ref<'a>>;
    type SomeOut<'a> = Option<<T as ValueRef>::Ref<'a>>;
    fn extract<'a, C: ConfigValues>(&self, v: &'a C, assoc: Option<&AssocDescription>) -> Self::Out<'a> {
        v.value_for_id(self.id, assoc)
            .or_else(|| v.value_default_for_id(self.id))
            .and_then(<T as ValueRef>::project)
    }
    fn extract_some<'a, C: ConfigValues>(&self, values: &'a C, assoc: Option<&AssocDescription>) -> Self::SomeOut<'a> {
        self.extract(values, assoc)
    }
}
impl<T: ValueRef> TypedKey for Key<'_, T> {
    type Out<'a> = <T as ValueRef>::Ref<'a>;
    type SomeOut<'a> = Option<<T as ValueRef>::Ref<'a>>;
    fn extract<'a, C: ConfigValues>(&self, v: &'a C, assoc: Option<&AssocDescription>) -> Self::Out<'a> {
        v.value_for_id(self.id, assoc)
            .or_else(|| v.value_default_for_id(self.id))
            .and_then(<T as ValueRef>::project)
            .expect("Key value is missing or invalid type")
    }
    fn extract_some<'a, C: ConfigValues>(&self, values: &'a C, assoc: Option<&AssocDescription>) -> Self::SomeOut<'a> {
        values
            .value_for_id(self.id, assoc)
            .or_else(|| values.value_default_for_id(self.id))
            .and_then(<T as ValueRef>::project)
    }
}

/// Names the source layer a config value came from.
///
/// Layers are stacked (e.g. built-in defaults, a config file, runtime
/// overrides) and a later layer shadows an earlier one for the same key. The
/// id is the layer's human-readable label, such as the
/// [`CONFIG_LAYER_ID`](loader::CONFIG_LAYER_ID) `"file"` layer. `Cow` so a
/// fixed layer can use a `'static` literal while a dynamic one owns its name.
pub type LayerId = Cow<'static, str>;

/// A config value type backed by a fixed set of named choices.
///
/// Implemented by the enums declared via
/// [`declare_config_enums`](crate::declare_config_enums). Each variant maps
/// to a stable `u32` discriminant and a string name, so an enum value can
/// round-trip through a [`Value::Enum`] in storage and a human-readable name
/// in a config file. [`CHOICES`](Self::CHOICES) lists every variant (with
/// optional display metadata) for validation and UI.
pub trait ConfigEnum:
    Clone + Copy + PartialEq + Eq + std::fmt::Debug + std::fmt::Display + Into<Value> + Send + Sync + 'static
{
    const CHOICES: meta::Choices<(u32, &'static str, Option<meta::EnumVisual>)>;

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

/// A read-only view over the configuration values stored in a single [`Object`]
/// or across all layers of a [`Context`](super::Context).
pub trait ConfigValues {
    type Iter<'a>: Iterator<Item = (KeyId<'a>, &'a Value, Option<&'a Condition>, &'a LayerId)>
    where
        Self: 'a;

    /// Returns a list of all values stored.
    ///
    /// Keys are sorted by layer and can duplicate.
    fn values_iter<'a>(&'a self, parent_layer_id: Option<&'a LayerId>) -> Self::Iter<'a>
    where
        Self: 'a;

    /// Returns a default value for `key` from the registry.
    fn value_default_for_id(&self, key: KeyId) -> Option<&Value>;

    /// Returns a configured value for `key` without applying default.
    fn value_for_id(&self, key: KeyId, assoc: Option<&AssocDescription>) -> Option<&Value>;

    /// Returns a configured value with default applied for `key`
    ///
    /// Association is taken into account only by [`Context`](super::Context).
    /// Regular [`Object`] ignores association and returns unconditional values.
    ///
    /// Use [`value_conditional`](Self::value_conditional) if you want to always
    /// take association into account and/or optimize performance.
    ///
    /// # Panics
    /// For a required key (`Key<T>`) this panics if neither a value nor a default
    /// is present, or if the stored value does not match the key's type `T`. An
    /// optional key (`Key<Option<T>>`) returns `None` in those cases instead.
    fn value<T: TypedKey>(&self, key: T) -> T::Out<'_>;

    /// Non-panicking counterpart of [`value`](Self::value): always returns an
    /// `Option`, yielding `None` when the key has no value and no default, or
    /// when the stored value does not match the key's type.
    ///
    /// For a required key (`Key<T>`) this is `Option<T>` instead of the bare `T`
    /// that [`value`](Self::value) would panic to produce; for an optional key
    /// (`Key<Option<T>>`) it is identical to [`value`](Self::value).
    ///
    /// Association is taken into account only by [`Context`](super::Context);
    /// use [`some_value_conditional`](Self::some_value_conditional) to pass one
    /// explicitly.
    fn some_value<T: TypedKey>(&self, key: T) -> T::SomeOut<'_>;

    /// Returns a configured value for `key` with default applied,
    /// taking supplied association into account.
    ///
    /// # Panics
    /// For a required key (`Key<T>`) this panics if neither a value nor a default
    /// is present, or if the stored value does not match the key's type `T`. An
    /// optional key (`Key<Option<T>>`) returns `None` in those cases instead.
    fn value_conditional<T: TypedKey>(&self, key: T, assoc: Option<&AssocDescription>) -> T::Out<'_>
    where
        Self: Sized,
    {
        key.extract(self, assoc)
    }

    /// Non-panicking counterpart of [`value_conditional`](Self::value_conditional),
    /// taking the supplied association into account: always returns an `Option`,
    /// yielding `None` when the key has no value and no default, or when the
    /// stored value does not match the key's type.
    ///
    /// For a required key (`Key<T>`) this is `Option<T>` instead of the bare `T`
    /// that [`value_conditional`](Self::value_conditional) would panic to
    /// produce; for an optional key (`Key<Option<T>>`) it is identical to
    /// [`value_conditional`](Self::value_conditional).
    fn some_value_conditional<T: TypedKey>(&self, key: T, assoc: Option<&AssocDescription>) -> T::SomeOut<'_>
    where
        Self: Sized,
    {
        key.extract_some(self, assoc)
    }
}

/// One configuration unit: a set of values described by a static schema.
///
/// Pairs an [`ObjectMeta`](meta::ObjectMeta) (the schema — which keys exist,
/// their types and defaults) with the [`ValueStore`] holding the actual
/// values. Reads go through the [`ConfigValues`] trait. An `Object` can stand
/// alone (e.g. global settings, a [`Context`](crate::Context)) or be nested
/// inside another as a [`Value::Object`].
///
/// An optional [`layer_id`](Self::layer_id) records which source layer the
/// values came from; see [`LayerId`].
#[derive(Debug, Clone)]
pub struct Object {
    layer_id: Option<LayerId>,
    object_meta: &'static meta::ObjectMeta,
    values: ValueStore,
}

impl Object {
    pub fn new(object_meta: &'static meta::ObjectMeta, values: ValueStore) -> Self {
        Object {
            layer_id: None,
            object_meta,
            values,
        }
    }

    pub fn new_with_layer_id(object_meta: &'static meta::ObjectMeta, values: ValueStore, layer_id: LayerId) -> Self {
        Object {
            layer_id: Some(layer_id),
            object_meta,
            values,
        }
    }

    pub fn new_empty(object_meta: &'static meta::ObjectMeta) -> Self {
        Object {
            layer_id: None,
            object_meta,
            values: ValueStore::new(),
        }
    }

    pub fn layer_id(&self) -> Option<&LayerId> {
        self.layer_id.as_ref()
    }

    pub fn set_layer_id(&mut self, layer_id: LayerId) {
        self.layer_id = Some(layer_id);
    }

    /// The metadata registry this layer resolves keys and defaults against.
    pub fn object_meta(&self) -> &'static meta::ObjectMeta {
        self.object_meta
    }

    /// Returns the value store of this object.
    pub fn values(&self) -> &ValueStore {
        &self.values
    }

    /// Returns the mutable value store of this object.
    pub fn values_mut(&mut self) -> &mut ValueStore {
        &mut self.values
    }
}

pub struct ConfigIter<'a> {
    layer_id: &'a LayerId,
    map_iter: std::collections::hash_map::Iter<'a, KeyId<'a>, map::Conditionals>,
    cond_iter: Option<(KeyId<'a>, std::slice::Iter<'a, (Value, Condition)>)>,
}
impl<'a> Iterator for ConfigIter<'a> {
    type Item = (KeyId<'a>, &'a Value, Option<&'a Condition>, &'a LayerId);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, cond_iter)) = &mut self.cond_iter {
            if let Some((val, cond)) = cond_iter.next() {
                return Some((*key, val, Some(cond), self.layer_id));
            } else {
                self.cond_iter = None;
            }
        }

        if let Some((key, map_value)) = self.map_iter.next() {
            self.cond_iter = Some((*key, map_value.0.iter()));
            self.next()
        } else {
            None
        }
    }
}

impl ConfigValues for Object {
    type Iter<'a>
        = ConfigIter<'a>
    where
        Self: 'a;

    fn values_iter<'a>(&'a self, parent_layer_id: Option<&'a LayerId>) -> Self::Iter<'a>
    where
        Self: 'a,
    {
        ConfigIter {
            layer_id: self.layer_id.as_ref().or(parent_layer_id).unwrap_or(&GLOBAL_LAYER_ID),
            map_iter: self.values.0.iter(),
            cond_iter: None,
        }
    }

    fn value_default_for_id(&self, key: KeyId) -> Option<&Value> {
        self.object_meta.default_of(key)
    }

    fn value_for_id<'a>(&'a self, key: KeyId<'_>, assoc: Option<&AssocDescription>) -> Option<&'a Value> {
        self.values.get_ranked(key, assoc).map(|(v, _)| v)
    }

    fn value<T: TypedKey>(&self, key: T) -> T::Out<'_> {
        key.extract(self, None)
    }

    fn some_value<T: TypedKey>(&self, key: T) -> T::SomeOut<'_> {
        key.extract_some(self, None)
    }
}
