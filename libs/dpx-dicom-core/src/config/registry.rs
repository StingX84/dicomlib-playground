use super::{Key, OBJECT_LAYER_ID, Value, meta::KeyMeta, meta::ValueDefault, meta::ValueMeta};
use crate::{Arc, HashMap};
use std::any::Any;

/// A statically-registered batch of key descriptors.
///
/// Modules and applications contribute their keys at link time via
/// `inventory::submit! { StaticRegistry(&[ ... ]) }`.
pub struct StaticRegistry(pub &'static [KeyMeta]);
inventory::collect!(StaticRegistry);

/// The assembled set of known keys with their metadata and default values.
#[derive(Debug, Clone)]
pub struct Registry {
    /// Keys supported by this registry with precomputed default values
    keys: HashMap<Key, (&'static KeyMeta, Value)>,
    objects: HashMap<Key, Arc<Registry>>,
}

impl Registry {
    /// Creates an empty registry with no keys.
    pub fn new_empty() -> Registry {
        Registry {
            keys: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    pub fn new_from_meta(meta_list: &'static [KeyMeta]) -> Registry {
        let mut rv = Registry::new_empty();
        rv.insert_multi(meta_list.iter());
        rv
    }

    /// Creates a registry pre-populated from all link-time [`StaticRegistry`] submissions.
    pub fn new_from_static() -> Registry {
        let mut rv = Registry::new_empty();
        for entry in inventory::iter::<StaticRegistry> {
            rv.insert_multi(entry.0.iter());
        }
        rv
    }

    pub fn insert(&mut self, key_meta: &'static KeyMeta) {
        if let ValueMeta::Object { meta: items, .. } = key_meta.value_meta {
            self.objects
                .insert(key_meta.key, Arc::new(Registry::new_from_meta(items)));
        }

        let def_value = match (&key_meta.default, &key_meta.value_meta) {
            (ValueDefault::Static(v), _) => v.clone(),
            (ValueDefault::Dynamic(v), _) => v(self),
            (ValueDefault::Default, ValueMeta::Bool { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Bool { .. }) => Value::Bool(bool::default()),
            (ValueDefault::Default, ValueMeta::String { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::String { .. }) => Value::String(String::default()),
            (ValueDefault::Default, ValueMeta::Int { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Int { .. }) => Value::Int(i64::default()),
            (ValueDefault::Default, ValueMeta::Enum { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Enum { .. }) => Value::Enum(u32::default()),
            (ValueDefault::Default, ValueMeta::Duration { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Duration { .. }) => Value::Duration(std::time::Duration::default()),
            (ValueDefault::Default, ValueMeta::Tag { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Tag { .. }) => Value::Tag(crate::Tag::new_standard(0, 0)),
            (ValueDefault::Default, ValueMeta::Vr { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Vr { .. }) => Value::Vr(crate::Vr::Undefined),
            (ValueDefault::Default, ValueMeta::File { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::File { .. }) => Value::File(super::ValueFile::Content(Vec::default())),
            (ValueDefault::Default, ValueMeta::Network { .. }) => Value::Null,
            (ValueDefault::Default, ValueMeta::Host { .. }) => Value::Null,
            (ValueDefault::Default, ValueMeta::Object { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Object { .. }) => Value::Object(super::Config::new_empty(
                OBJECT_LAYER_ID,
                self.objects
                    .get(&key_meta.key)
                    .expect("Registry was constructed at the start of a function")
                    .clone(),
            )),
            (ValueDefault::Default, ValueMeta::Vec { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Vec { .. }) => Value::Vec(Vec::default()),
            (ValueDefault::Default, ValueMeta::Map { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Map { .. }) => Value::Map(crate::Map::default()),
            (ValueDefault::Default, ValueMeta::Complex { nullable, .. }) if *nullable => Value::Null,
            (ValueDefault::Default, ValueMeta::Complex { ty, .. }) => {
                Value::Complex(ty.default().expect("Unable to create default value"))
            }
        };
        
        #[cfg(debug_assertions)]
        assert!(
            Self::fast_acceptance_test(&key_meta.value_meta, &def_value),
            "BUG! Default value {:?} is not compatible with its meta {:?}",
            def_value,
            key_meta.value_meta
        );

        self.keys.insert(key_meta.key, (key_meta, def_value));
    }

    pub fn insert_multi(&mut self, iter: impl Iterator<Item = &'static KeyMeta>) {
        let (_, upper) = iter.size_hint();
        if let Some(l) = upper {
            self.keys.reserve(l);
        }
        iter.for_each(|v| self.insert(v));
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyMeta> {
        self.keys.values().map(|(m, _)| *m)
    }

    pub fn get(&self, key: &Key) -> Option<&'static KeyMeta> {
        self.keys.get(key).map(|(m, _)| *m)
    }

    pub fn get_object_registry(&self, key: &Key) -> Option<&Arc<Registry>> {
        self.objects.get(key)
    }

    pub fn default_value_of(&self, key: &Key) -> Option<&Value> {
        self.keys.get(key).map(|(_, d)| d)
    }

    pub fn remove(&mut self, key: &Key) {
        self.keys.remove(key);
        self.objects.remove(key);
    }

    #[cfg(debug_assertions)]
    fn fast_acceptance_test(value_meta: &ValueMeta, value: &Value) -> bool {
        match (value_meta, value) {
            (_, Value::Null) => true,
            (ValueMeta::Bool { .. }, Value::Bool(_)) => true,
            (ValueMeta::String { .. }, Value::String(_)) => true,
            (ValueMeta::Int { .. }, Value::Int(_)) => true,
            (ValueMeta::Enum { .. }, Value::Enum(_)) => true,
            (ValueMeta::Duration { .. }, Value::Duration(_)) => true,
            (ValueMeta::Tag { .. }, Value::Tag(_)) => true,
            (ValueMeta::Vr { .. }, Value::Vr(_)) => true,
            (ValueMeta::File { .. }, Value::File(_)) => true,
            (ValueMeta::Network { .. }, Value::Network(_)) => true,
            (ValueMeta::Host { .. }, Value::Host(_)) => true,
            (ValueMeta::Object { .. }, Value::Object(_)) => true,
            (ValueMeta::Vec { .. }, Value::Vec(_)) => true,
            (ValueMeta::Map { .. }, Value::Map(_)) => true,
            (ValueMeta::Complex { ty, .. }, Value::Complex(v)) if v.type_id() == ty.type_id() => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::GlobalConfig, config::validator::Validator};

    #[test]
    fn check_if_config_default_values_are_compatible_with_keys() {
        let config = GlobalConfig::current();
        let registry = config.registry();
        for (_, (key_meta, def_value)) in registry.keys.iter() {
            let validator = Validator {
                key_meta,
                value_meta: &key_meta.value_meta,
                vec_index: None,
                map_key: None,
                file: None,
                parent: None,
            };
            validator
                .validate(def_value)
                .unwrap_or_else(|e| panic!("Static configuration failed to validate default value: {}", e));
        }
    }
}
