use crate::{Arc, Cow, HashMap, Map};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValueFile {
    Name { path: String, auto_reload: bool },
    Content(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum MaybeGenerated<T>
where
    T: 'static,
{
    Static(&'static [T]),
    Dynamic(fn() -> Box<dyn Iterator<Item = T>>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    String(String),
    Int(i64),
    Enum(u32),
    Duration(std::time::Duration),
    Tag(crate::tag::Tag<'static>),
    Vr(crate::vr::Vr),
    File(ValueFile),
    Vec(Vec<Value>),
    Map(Map<Value, Value>),
    Complex(Arc<dyn std::any::Any>),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key {
    pub module: &'static str,
    pub code: u32,
}
impl Key {
    #[inline]
    pub const fn new(module: &'static str, code: u32) -> Key {
        Key { module, code }
    }
}

#[derive(Debug, Clone)]
pub struct Concept {
    pub name: &'static str,
    pub display_name: &'static str,
    pub help_string: Option<&'static str>,
}
impl Concept {
    pub const fn new(
        name: &'static str,
        display_name: &'static str,
        help_string: Option<&'static str>,
    ) -> Concept {
        Self {
            name,
            display_name,
            help_string,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueMeta {
    Bool,
    String {
        regexp: Option<&'static str>,
        min_length: Option<u32>,
        max_length: Option<u32>,
    },
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },
    Enum {
        values: MaybeGenerated<(u32, Concept)>,
    },
    Duration {
        min: Option<i64>,
        max: Option<i64>,
    },
    Tag {
        filter_by_levels: Option<&'static [&'static crate::tag::Level<'static>]>,
        filter_by_vr: Option<&'static [crate::vr::Vr]>,
        one_of: Option<MaybeGenerated<crate::tag::Tag<'static>>>,
    },
    Vr {
        one_of: Option<MaybeGenerated<crate::vr::Vr>>,
    },
    File {
        name_only: bool,
        content_only: bool,
    },
    Vec {
        items: &'static ValueMeta,
        min: Option<u32>,
        max: Option<u32>,
    },
    Map {
        keys: &'static ValueMeta,
        values: &'static ValueMeta,
        min: Option<u32>,
        max: Option<u32>,
    },
    Complex {
        kind: &'static str,
        limits: HashMap<&'static str, &'static str>,
    },
}

#[derive(Debug, Clone)]
pub struct KeyMeta {
    pub key: Key,
    pub is_advanced: bool,
    pub display_section: &'static str,
    pub concept: Concept,
    pub value_meta: ValueMeta,
    pub make_default: fn() -> Option<Value>,
}

pub struct StaticRegistry(pub &'static [KeyMeta]);
inventory::collect!(StaticRegistry);

#[derive(Debug, Clone)]
pub struct Registry {
    keys: HashMap<Key, &'static KeyMeta>,
    defaults: HashMap<Key, Option<Value>>,
}

impl Registry {
    pub(crate) fn new_empty() -> Registry {
        Registry {
            keys: HashMap::new(),
            defaults: HashMap::new(),
        }
    }
    pub(crate) fn new() -> Registry {
        let mut rv = Registry {
            keys: HashMap::new(),
            defaults: HashMap::new(),
        };
        for entry in inventory::iter::<StaticRegistry> {
            rv.insert_multi(entry.0.iter());
        }
        rv
    }
    pub fn insert(&mut self, key_meta: &'static KeyMeta) {
        self.defaults
            .insert(key_meta.key, (key_meta.make_default)());
        self.keys.insert(key_meta.key, key_meta);
    }
    pub fn insert_multi(&mut self, iter: impl Iterator<Item = &'static KeyMeta>) {
        if let (_, Some(l)) = iter.size_hint() {
            self.defaults.reserve(l);
            self.keys.reserve(l);
        }
        iter.for_each(|v| {
            self.insert(v);
        });
    }
    pub fn iter(&self) -> impl Iterator<Item = &KeyMeta> {
        self.keys.values().copied()
    }
    pub fn get(&self, key: &'_ Key) -> Option<&'static KeyMeta> {
        self.keys.get(key).copied()
    }
    pub fn default_value_of<'a>(&'a self, key: &'_ Key) -> Option<&'a Value> {
        self.defaults.get(key)?.as_ref()
    }
    pub fn remove(&mut self, key: &Key) {
        self.keys.remove(key);
        self.defaults.remove(key);
    }
}

#[derive(Debug, Clone)]
pub struct Settings(HashMap<Key, Value>);

impl Settings {
    pub fn new() -> Settings {
        Settings(HashMap::new())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.0.iter()
    }
    pub fn set_many(&mut self, values: impl Iterator<Item = (Key, Value)>) {
        values.for_each(|(k, v)| {
            self.0.insert(k, v);
        });
    }
    pub fn set(&mut self, k: Key, v: Value) {
        self.0.insert(k, v);
    }
    pub fn get<'a>(&'a self, k: &'_ Key) -> Option<&'a Value> {
        self.0.get(k)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(Key, Value)> for Settings {
    fn from_iter<T: IntoIterator<Item = (Key, Value)>>(values: T) -> Self {
        Settings(HashMap::from_iter(values))
    }
}

impl IntoIterator for Settings {
    type Item = (Key, Value);
    type IntoIter = <HashMap<Key, Value> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ConditionalKey {
    key: Key,
    peer_aet: Option<Cow<'static, str>>,
    local_aet: Option<Cow<'static, str>>,
    peer_ip: Option<std::net::IpAddr>,
    local_ip: Option<std::net::IpAddr>,
    local_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct ConditionalSettings(Vec<(ConditionalKey, Value)>);

impl ConditionalSettings {
    pub fn new() -> ConditionalSettings {
        ConditionalSettings(Vec::new())
    }
    pub fn iter(&self) -> impl Iterator<Item = &(ConditionalKey, Value)> {
        self.0.iter()
    }
    pub fn add_many(&mut self, values: impl Iterator<Item = (ConditionalKey, Value)>) {
        values.for_each(|(k, v)| {
            self.0.push((k, v));
        });
    }
    pub fn add(&mut self, k: ConditionalKey, v: Value) {
        self.0.push((k, v));
    }
    pub fn get<'a>(&'a self, k: &'_ ConditionalKey) -> Option<&'a Value> {
        self.0
            .iter()
            .filter_map(|v| {
                if v.0.key != k.key {
                    return None;
                }
                let mut score = 0;
                if v.0.peer_aet.is_some() {
                    if k.peer_aet.is_none() || v.0.peer_aet != k.peer_aet {
                        return None;
                    }
                    score += 16;
                }
                if v.0.local_aet.is_some() {
                    if k.local_aet.is_none() || v.0.local_aet != k.local_aet {
                        return None;
                    }
                    score += 8;
                }
                if v.0.peer_ip.is_some() {
                    if k.peer_ip.is_none() || v.0.peer_ip != k.peer_ip {
                        return None;
                    }
                    score += 4;
                }
                if v.0.local_ip.is_some() {
                    if k.local_ip.is_none() || v.0.local_ip != k.local_ip {
                        return None;
                    }
                    score += 2;
                }
                if v.0.local_port.is_some() {
                    if k.local_port.is_none() || v.0.local_port != k.local_port {
                        return None;
                    }
                    score += 1;
                }
                Some((score, &v.0, &v.1))
            })
            .max_by(|l, r| l.0.cmp(&r.0))
            .map(|t| t.2)
    }
}

impl Default for ConditionalSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(ConditionalKey, Value)> for ConditionalSettings {
    fn from_iter<T: IntoIterator<Item = (ConditionalKey, Value)>>(values: T) -> Self {
        ConditionalSettings(Vec::from_iter(values))
    }
}

impl IntoIterator for ConditionalSettings {
    type Item = (ConditionalKey, Value);
    type IntoIter = <Vec<(ConditionalKey, Value)> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
