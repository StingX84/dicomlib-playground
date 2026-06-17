//! The assembled, read-only configuration snapshot.
//!
//! A [`Config`] is one immutable layer: the unconditional [`Settings`], the
//! association-aware [`ConditionalSettings`] overlay, and the [`Registry`] that
//! supplies metadata and defaults. Layers are stacked through [`Context`](crate::context);
//! the hot-swappable global manager that owns the base layer is added in a later phase.

use super::{ConditionalSettings, Key, MatchAttributes, Registry, Settings, Value};
use crate::Arc;

/// An immutable configuration layer.
///
/// Resolution within a single layer is: the best-matching conditional value,
/// then the unconditional value, then the registry default. Cross-layer
/// resolution is handled by [`Context`](crate::Context).
#[derive(Debug, Clone)]
pub struct Config {
    registry: Arc<Registry>,
    settings: Settings,
    conditional: ConditionalSettings,
    version: u32,
}

impl Config {
    /// Begins building a layer backed by the given metadata `registry`.
    pub fn builder(registry: Arc<Registry>) -> ConfigBuilder {
        ConfigBuilder {
            registry,
            settings: Settings::new(),
            conditional: ConditionalSettings::new(),
            version: 0,
        }
    }

    /// The metadata registry this layer resolves keys and defaults against.
    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }

    /// The schema version this layer was produced for (used by migrations).
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the explicitly-configured value for `key`, ignoring defaults.
    ///
    /// A matching conditional value takes precedence over the unconditional one.
    pub fn get_explicit(&self, key: &Key, attrs: &MatchAttributes) -> Option<&Value> {
        self.conditional.get(key, attrs).or_else(|| self.settings.get(key))
    }

    /// Returns the registry default for `key`, if any.
    pub fn default_of(&self, key: &Key) -> Option<&Value> {
        self.registry.default_value_of(key)
    }

    /// Resolves `key` within this single layer: explicit value, then default.
    pub fn get(&self, key: &Key, attrs: &MatchAttributes) -> Option<&Value> {
        self.get_explicit(key, attrs).or_else(|| self.default_of(key))
    }
}

/// Builder for an immutable [`Config`] layer.
pub struct ConfigBuilder {
    registry: Arc<Registry>,
    settings: Settings,
    conditional: ConditionalSettings,
    version: u32,
}

impl ConfigBuilder {
    pub fn settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    pub fn conditional(mut self, conditional: ConditionalSettings) -> Self {
        self.conditional = conditional;
        self
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn build(self) -> Config {
        Config {
            registry: self.registry,
            settings: self.settings,
            conditional: self.conditional,
            version: self.version,
        }
    }
}
