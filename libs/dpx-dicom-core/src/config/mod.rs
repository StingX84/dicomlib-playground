//! Application configuration system.
//!
//! The configuration system separates these concerns:
//!
//! - **Metadata** ([`meta`]) — descriptors that let any key be validated,
//!   edited in a GUI/TUI and documented without hard-coding it. Applications
//!   extend the surface by submitting [`ObjectMetaProvider`](meta::ObjectMetaProvider) batches via `inventory`.
//! - **Values** ([`value`]) — the dynamically-typed [`Value`] payloads.
//! - **Values Map** ([`map`]) — loaded data with [`Value`]s mapped to keys, either
//!   conditionally or unconditionally.
//! - **Object** ([`Object`]) — a single object storing [`ObjectMeta`](meta::ObjectMeta)
//!   and a [`Map`]. It is the unit of configuration that can be composed into a
//!   [`GlobalConfig`], installed into a [`Context`](crate::Context) or be a part of [`Value::Object`].
//! - **GlobalConfig** ([`GlobalConfig`]) — the single source of truth for getting
//!   and setting the global base [`Object`].
//! - **Loader** ([`loader`]) — a [`serde`] deserializer that reads a configuration
//!   file and produces a [`Object`].

#[cfg(feature = "serde")]
pub mod custom;
pub mod global;
#[cfg(feature = "serde")]
pub mod loader;
pub mod macros;
pub mod map;
pub mod meta;
pub mod subst;
pub mod typed;
pub(crate) mod validator;
pub mod value;

use std::borrow::Cow;

#[cfg(feature = "serde")]
pub use custom::{CustomType, Serde};
pub use global::GlobalConfig;
#[cfg(feature = "serde")]
pub use loader::YamlLoader;
pub use map::{Condition, Map};
pub use meta::Value;
pub use subst::{AppDir, SubstVars};
pub use value::{File, ValueRef, millis, mins, secs};

use crate::network::AssocDescription;

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
pub struct Key(pub &'static str);

impl Key {
    #[inline]
    pub const fn new(name: &'static str) -> Key {
        Key(name)
    }

    /// The key's path/identity string.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl From<&'static str> for Key {
    fn from(value: &'static str) -> Self {
        Key::new(value)
    }
}

pub type LayerId = Cow<'static, str>;

/// A read-only view over the configuration values stored in a single [`Object`]
/// or across all layers of a [`Context`](super::Context).
pub trait ConfigValues {
    type Iter<'a>: Iterator<Item = (&'a Key, &'a Value, Option<&'a Condition>, &'a LayerId)>
    where
        Self: 'a;

    /// Returns a list of all values stored.
    ///
    /// Keys are sorted by layer and can duplicate.
    fn config_iter<'a>(&'a self, parent_layer_id: Option<&'a LayerId>) -> Self::Iter<'a>
    where
        Self: 'a;

    /// Returns a default value for `key` from the registry.
    fn config_default_of(&self, key: &Key) -> Option<&Value>;

    /// Returns a configured value for `key` without applying default.
    fn config_get_explicit(&self, key: &Key, assoc: Option<&AssocDescription>) -> Option<&Value>;

    /// Returns a configured value for `key` or default.
    fn config_get(&self, key: &Key, assoc: Option<&AssocDescription>) -> Option<&Value> {
        self.config_get_explicit(key, assoc)
            .or_else(|| self.config_default_of(key))
    }

    /// Returns a configured value for `key` casted to native type T
    ///
    /// See [`ValueRef`] for supported types.
    fn config_get_as<T: ValueRef>(
        &self,
        key: &Key,
        assoc: Option<&AssocDescription>,
    ) -> Option<<T as ValueRef>::Ref<'_>> {
        self.config_get(key, assoc).and_then(<T as ValueRef>::project)
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    layer_id: Option<LayerId>,
    object_meta: &'static meta::ObjectMeta,
    values: Map,
}

impl Object {
    pub fn new(object_meta: &'static meta::ObjectMeta, values: Map) -> Self {
        Object {
            layer_id: None,
            object_meta,
            values,
        }
    }

    pub fn new_with_layer_id(object_meta: &'static meta::ObjectMeta, values: Map, layer_id: LayerId) -> Self {
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
            values: Map::new(),
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

    /// Returns a map of values
    pub fn values(&self) -> &Map {
        &self.values
    }

    /// Returns a mutable map of values
    pub fn values_mut(&mut self) -> &mut Map {
        &mut self.values
    }

    /// Returns the default value for `key`.
    ///
    /// Note: every registered key must have a default value.
    pub fn default_of(&self, key: &Key) -> Option<&Value> {
        self.object_meta.default_of(key)
    }
}

pub struct ConfigIter<'a> {
    layer_id: &'a LayerId,
    map_iter: std::collections::hash_map::Iter<'a, Key, map::Conditionals>,
    cond_iter: Option<(&'a Key, std::slice::Iter<'a, (Value, Condition)>)>,
}
impl<'a> Iterator for ConfigIter<'a> {
    type Item = (&'a Key, &'a Value, Option<&'a Condition>, &'a LayerId);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, cond_iter)) = &mut self.cond_iter {
            if let Some((val, cond)) = cond_iter.next() {
                return Some((key, val, Some(cond), self.layer_id));
            } else {
                self.cond_iter = None;
            }
        }

        if let Some((key, map_value)) = self.map_iter.next() {
            self.cond_iter = Some((key, map_value.0.iter()));
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

    fn config_iter<'a>(&'a self, parent_layer_id: Option<&'a LayerId>) -> Self::Iter<'a>
    where
        Self: 'a,
    {
        ConfigIter {
            layer_id: self.layer_id.as_ref().or(parent_layer_id).unwrap_or(&GLOBAL_LAYER_ID),
            map_iter: self.values.0.iter(),
            cond_iter: None,
        }
    }

    fn config_default_of(&self, key: &Key) -> Option<&Value> {
        self.default_of(key)
    }

    fn config_get_explicit(&self, key: &Key, assoc: Option<&AssocDescription>) -> Option<&Value> {
        self.values.get_ranked(key, assoc).map(|(v, _)| v)
    }
}
