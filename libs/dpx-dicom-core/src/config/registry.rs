use super::{Key, Value, meta::KeyMeta, meta::ValueMeta};
use crate::{Arc, HashMap};

/// A statically-registered batch of key descriptors.
///
/// Modules and applications contribute their keys at link time via
/// `inventory::submit! { StaticRegistry(&[ ... ]) }`.
pub struct StaticRegistry(pub &'static [KeyMeta]);
inventory::collect!(StaticRegistry);

/// The assembled set of known keys with their metadata and default values.
#[derive(Debug, Clone)]
pub struct Registry {
    keys: HashMap<Key, &'static KeyMeta>,
    defaults: HashMap<Key, Option<Value>>,
    objects: HashMap<Key, Arc<Registry>>,
}

impl Registry {
    /// Creates an empty registry with no keys.
    pub fn new_empty() -> Registry {
        Registry {
            keys: HashMap::new(),
            defaults: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    pub fn new_from(meta_list: &'static [KeyMeta]) -> Registry {
        let mut rv = Registry::new_empty();
        rv.insert_multi(meta_list.iter());
        rv
    }

    /// Creates a registry pre-populated from all link-time [`StaticRegistry`] submissions.
    pub fn new() -> Registry {
        let mut rv = Registry::new_empty();
        for entry in inventory::iter::<StaticRegistry> {
            rv.insert_multi(entry.0.iter());
        }
        rv
    }

    pub fn insert(&mut self, key_meta: &'static KeyMeta) {
        self.defaults.insert(key_meta.key, key_meta.default.map(|v| v()));
        self.keys.insert(key_meta.key, key_meta);
        if let ValueMeta::Object { items, .. } = key_meta.value_meta {
            self.objects.insert(key_meta.key, Arc::new(Registry::new_from(items)));
        }
    }

    pub fn insert_multi(&mut self, iter: impl Iterator<Item = &'static KeyMeta>) {
        let (_, upper) = iter.size_hint();
        if let Some(l) = upper {
            self.defaults.reserve(l);
            self.keys.reserve(l);
        }
        iter.for_each(|v| self.insert(v));
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyMeta> {
        self.keys.values().copied()
    }

    pub fn get(&self, key: &Key) -> Option<&'static KeyMeta> {
        self.keys.get(key).copied()
    }

    pub fn get_object_registry(&self, key: &Key) -> Option<&Arc<Registry>> {
        self.objects.get(key)
    }

    pub fn default_value_of(&self, key: &Key) -> Option<&Value> {
        self.defaults.get(key)?.as_ref()
    }

    pub fn remove(&mut self, key: &Key) {
        self.keys.remove(key);
        self.defaults.remove(key);
        self.objects.remove(key);
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
