//! Parsed configuration values: the unconditional [`Settings`] map and the
//! association-aware [`ConditionalSettings`] overlay.
//!
//! These hold the data that a loaded configuration layer contributes. Metadata
//! and defaults live separately in the [`Registry`](super::Registry).

use super::{Key, Value};
use std::borrow::Cow;

/// A flat map of unconditional configuration values.
#[derive(Debug, Clone, Default)]
pub struct Settings(crate::HashMap<Key, Value>);

impl Settings {
    pub fn new() -> Settings {
        Settings(crate::HashMap::new())
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

    pub fn get(&self, k: &Key) -> Option<&Value> {
        self.0.get(k)
    }
}

impl FromIterator<(Key, Value)> for Settings {
    fn from_iter<T: IntoIterator<Item = (Key, Value)>>(values: T) -> Self {
        Settings(crate::HashMap::from_iter(values))
    }
}

impl IntoIterator for Settings {
    type Item = (Key, Value);
    type IntoIter = <crate::HashMap<Key, Value> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The association attributes a [`ConditionalKey`] may be matched against.
///
/// All fields are optional; an absent field is a wildcard for that dimension.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchAttributes {
    pub peer_aet: Option<Cow<'static, str>>,
    pub local_aet: Option<Cow<'static, str>>,
    pub peer_ip: Option<std::net::IpAddr>,
    pub local_ip: Option<std::net::IpAddr>,
    pub local_port: Option<u16>,
}

/// A configuration key qualified by association-matching attributes.
///
/// A value attached to a `ConditionalKey` only applies when every attribute it
/// specifies matches the active association; unspecified attributes match anything.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ConditionalKey {
    pub key: Key,
    pub peer_aet: Option<Cow<'static, str>>,
    pub local_aet: Option<Cow<'static, str>>,
    pub peer_ip: Option<std::net::IpAddr>,
    pub local_ip: Option<std::net::IpAddr>,
    pub local_port: Option<u16>,
}

impl ConditionalKey {
    /// Creates an unconditional key (matches any association).
    pub fn unconditional(key: Key) -> ConditionalKey {
        ConditionalKey {
            key,
            peer_aet: None,
            local_aet: None,
            peer_ip: None,
            local_ip: None,
            local_port: None,
        }
    }
}

/// An overlay of values selected by association attributes.
///
/// On lookup, the most specific matching entry wins; specificity is scored so
/// that `peer_aet` outranks `local_aet`, which outranks `peer_ip`, and so on,
/// matching the precedence an operator intuitively expects.
#[derive(Debug, Clone, Default)]
pub struct ConditionalSettings(Vec<(ConditionalKey, Value)>);

impl ConditionalSettings {
    pub fn new() -> ConditionalSettings {
        ConditionalSettings(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &(ConditionalKey, Value)> {
        self.0.iter()
    }

    pub fn add_many(&mut self, values: impl Iterator<Item = (ConditionalKey, Value)>) {
        values.for_each(|(k, v)| self.0.push((k, v)));
    }

    pub fn add(&mut self, k: ConditionalKey, v: Value) {
        self.0.push((k, v));
    }

    /// Returns the value for `key` whose conditions best match `attrs`.
    ///
    /// An entry is a candidate only if every attribute it constrains is present
    /// in `attrs` and equal. Among candidates, the highest specificity score wins.
    pub fn get(&self, key: &Key, attrs: &MatchAttributes) -> Option<&Value> {
        self.0
            .iter()
            .filter_map(|(ck, v)| {
                if ck.key != *key {
                    return None;
                }
                let mut score = 0u32;
                if ck.peer_aet.is_some() {
                    if attrs.peer_aet.is_none() || ck.peer_aet != attrs.peer_aet {
                        return None;
                    }
                    score += 16;
                }
                if ck.local_aet.is_some() {
                    if attrs.local_aet.is_none() || ck.local_aet != attrs.local_aet {
                        return None;
                    }
                    score += 8;
                }
                if ck.peer_ip.is_some() {
                    if attrs.peer_ip.is_none() || ck.peer_ip != attrs.peer_ip {
                        return None;
                    }
                    score += 4;
                }
                if ck.local_ip.is_some() {
                    if attrs.local_ip.is_none() || ck.local_ip != attrs.local_ip {
                        return None;
                    }
                    score += 2;
                }
                if ck.local_port.is_some() {
                    if attrs.local_port.is_none() || ck.local_port != attrs.local_port {
                        return None;
                    }
                    score += 1;
                }
                Some((score, v))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, v)| v)
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
