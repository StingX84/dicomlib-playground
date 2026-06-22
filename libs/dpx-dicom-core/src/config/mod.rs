//! Application configuration system.
//!
//! The configuration system separates these concerns:
//!
//! - **Metadata** ([`meta`]) — descriptors that let any key be validated,
//!   edited in a GUI/TUI and documented without hard-coding it. Applications
//!   extend the surface by submitting [`StaticRegistry`] batches via `inventory`.
//! - **Values** ([`value`]) — the dynamically-typed [`Value`] payloads.
//! - **Values Map** ([`map`]) — loaded data with [`Value`]s mapped to keys, either
//!   conditionally or unconditionally.
//! - **Config** ([`Config`]) — a single configuration layer tying a [`Registry`]
//!   to a [`Map`]. It is the unit of configuration that can be composed into a
//!   [`GlobalConfig`] or installed into a [`Context`](crate::Context).
//! - **GlobalConfig** ([`GlobalConfig`]) — the single source of truth for getting
//!   and setting the global base [`Config`].
//! - **Loader** ([`loader`]) — a [`serde`] deserializer that reads a configuration
//!   file and produces a [`Config`] layer.

pub mod complex;
pub mod global;
#[cfg(feature = "serde")]
pub mod loader;
//#[macro_use]
//pub mod macros;
pub mod map;
pub mod meta;
pub mod registry;
pub mod subst;
pub mod typed;
pub(crate) mod validator;
pub mod value;

use std::borrow::Cow;

pub use complex::{ComplexConfigNode, ComplexType};
pub use global::GlobalConfig;
#[cfg(feature = "serde")]
pub use loader::YamlLoader;
pub use map::{Condition, Map};
pub use registry::{Registry, StaticRegistry};
pub use subst::{AppDir, SubstVars};
pub use value::{Value, ValueFile, ValueRef, millis, mins, secs};

use crate::{Arc, network::AssocDescription};

pub const GLOBAL_LAYER_ID: LayerId = LayerId::Borrowed("<global>");
pub const OBJECT_LAYER_ID: LayerId = LayerId::Borrowed("<object>");

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

/// A read-only view over the configuration values stored in a single [`Config`]
/// or across all layers of a [`Context`](super::Context).
pub trait ConfigValues {
    type Iter<'a>: Iterator<Item = (&'a Key, &'a Value, Option<&'a Condition>, &'a LayerId)>
    where
        Self: 'a;

    /// Returns a list of all values stored.
    ///
    /// Keys are sorted by layer and can duplicate.
    fn config_iter(&self) -> Self::Iter<'_>;

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

/// One configuration layer: a [`Map`] of values resolved against a [`Registry`].
#[derive(Debug, Clone)]
pub struct Config {
    layer_id: LayerId,
    registry: Arc<Registry>,
    map: Map,
}

impl Config {
    pub fn new(layer_id: LayerId, registry: Arc<Registry>, map: Map) -> Self {
        Config {
            layer_id,
            registry,
            map,
        }
    }

    pub fn new_empty(layer_id: LayerId, registry: Arc<Registry>) -> Self {
        Config {
            layer_id,
            registry,
            map: Map::new(),
        }
    }

    pub fn layer_id(&self) -> &LayerId {
        &self.layer_id
    }

    /// The metadata registry this layer resolves keys and defaults against.
    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }

    /// Returns a map of values
    pub fn values(&self) -> &Map {
        &self.map
    }

    /// Returns a mutable map of values
    pub fn values_mut(&mut self) -> &mut Map {
        &mut self.map
    }

    /// Returns the default value for `key`.
    ///
    /// Note: every registered key must have a default value.
    pub fn default_of(&self, key: &Key) -> Option<&Value> {
        self.registry.default_value_of(key)
    }
}

pub struct ConfigIter<'a> {
    config: &'a Config,
    map_iter: std::collections::hash_map::Iter<'a, Key, map::Conditionals>,
    cond_iter: Option<(&'a Key, std::slice::Iter<'a, (Value, Condition)>)>,
}
impl<'a> Iterator for ConfigIter<'a> {
    type Item = (&'a Key, &'a Value, Option<&'a Condition>, &'a LayerId);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, cond_iter)) = &mut self.cond_iter {
            if let Some((val, cond)) = cond_iter.next() {
                return Some((key, val, Some(cond), &self.config.layer_id));
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

impl ConfigValues for Config {
    type Iter<'a>
        = ConfigIter<'a>
    where
        Self: 'a;

    fn config_iter(&self) -> Self::Iter<'_> {
        ConfigIter {
            config: self,
            map_iter: self.map.0.iter(),
            cond_iter: None,
        }
    }

    fn config_default_of(&self, key: &Key) -> Option<&Value> {
        self.default_of(key)
    }

    fn config_get_explicit(&self, key: &Key, assoc: Option<&AssocDescription>) -> Option<&Value> {
        self.map.get_ranked(key, assoc).map(|(v, _)| v)
    }
}
