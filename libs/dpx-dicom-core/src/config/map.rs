//! Configuration values map.

use super::{Key, Value};
use crate::network::{AssocDescription, Network};
use std::borrow::Cow;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Condition {
    pub is_tls_used: Option<bool>,
    pub is_incoming: Option<bool>,
    pub is_virtual: Option<bool>,
    pub peer_aet: Option<Cow<'static, str>>,
    pub local_aet: Option<Cow<'static, str>>,
    pub peer_network: Option<Network>,
    pub local_network: Option<Network>,
}

pub const DEFAULT_CONDITION: Condition = Condition {
    is_tls_used: None,
    is_incoming: None,
    is_virtual: None,
    peer_aet: None,
    local_aet: None,
    peer_network: None,
    local_network: None,
};

#[derive(Debug, Clone)]
pub struct Conditionals(pub(crate) Vec<(Value, Condition)>);

impl From<(Value, Condition)> for Conditionals {
    fn from((value, condition): (Value, Condition)) -> Self {
        Self(vec![(value, condition)])
    }
}
impl From<Value> for Conditionals {
    fn from(value: Value) -> Self {
        Self(vec![(value, Condition::default())])
    }
}

#[derive(Debug, Clone, Default)]
pub struct Map(pub(crate) crate::HashMap<Key, Conditionals>);

impl Map {
    pub fn new() -> Map {
        Map(crate::HashMap::new())
    }

    /// Adds a new possibly value for `key` possibly with condition attached.
    /// Newer values with a same rank will win over previous values.
    pub fn add(&mut self, key: Key, value: Value, cond: Option<Condition>) {
        let entry = self.0.entry(key);
        match entry {
            std::collections::hash_map::Entry::Occupied(mut existing) => {
                existing.get_mut().0.push((value, cond.unwrap_or(DEFAULT_CONDITION)))
            }
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(Conditionals(vec![(value, cond.unwrap_or(DEFAULT_CONDITION))]));
            }
        }
    }

    pub fn get_ranked(&self, key: &Key, assoc: Option<&AssocDescription>) -> Option<(&Value, u32)> {
        if let Some(assoc) = assoc {
            self.0.get(key).and_then(|cv| {
                cv.0.iter()
                    .filter_map(|(v, condition)| Self::rank_conditional(condition, assoc).map(|score| (v, score)))
                    .max_by(|l, r| l.1.cmp(&r.1))
            })
        } else {
            self.0.get(key).and_then(|cv| {
                cv.0.iter()
                    .filter_map(|(v, condition)| {
                        if *condition == DEFAULT_CONDITION {
                            Some((v, 0))
                        } else {
                            None
                        }
                    })
                    .next_back()
            })
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Conditionals)> {
        self.0.iter()
    }

    fn rank_conditional(cond: &Condition, assoc: &AssocDescription) -> Option<u32> {
        let mut score = 0u32;
        if let Some(is_tls_used) = cond.is_tls_used {
            if is_tls_used != assoc.is_tls_used {
                return None;
            }
            score += 64;
        }
        if let Some(is_incoming) = cond.is_incoming {
            if is_incoming != assoc.is_incoming {
                return None;
            }
            score += 32;
        }
        if let Some(is_virtual) = cond.is_virtual {
            if is_virtual != assoc.is_virtual {
                return None;
            }
            score += 16;
        }

        if let Some(peer_aet) = &cond.peer_aet {
            if Some(peer_aet.as_ref()) != assoc.peer_aet.as_deref() {
                return None;
            }
            score += 8;
        }
        if let Some(local_aet) = &cond.local_aet {
            if Some(local_aet.as_ref()) != assoc.local_aet.as_deref() {
                return None;
            }
            score += 4;
        }
        if let Some(net) = &cond.peer_network {
            if let Some(addr) = &assoc.peer_addr {
                if !addr.sock_addr.is_in_network(net) {
                    return None;
                }
                score += 2;
            } else {
                return None;
            }
        }
        if let Some(net) = &cond.local_network {
            if let Some(addr) = &assoc.local_addr {
                if !addr.sock_addr.is_in_network(net) {
                    return None;
                }
                score += 1;
            } else {
                return None;
            }
        }

        Some(score)
    }
}

impl IntoIterator for Map {
    type Item = (Key, Conditionals);
    type IntoIter = <crate::HashMap<Key, Conditionals> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(Key, Conditionals)> for Map {
    fn from_iter<T: IntoIterator<Item = (Key, Conditionals)>>(values: T) -> Self {
        Map(crate::HashMap::from_iter(values))
    }
}

impl FromIterator<(Key, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (Key, Value)>>(values: T) -> Self {
        Map(crate::HashMap::from_iter(
            values
                .into_iter()
                .map(|(k, v)| (k, Conditionals(vec![(v, DEFAULT_CONDITION)]))),
        ))
    }
}

impl FromIterator<(Key, Value, Condition)> for Map {
    fn from_iter<T: IntoIterator<Item = (Key, Value, Condition)>>(iter: T) -> Self {
        let mut map = Map::new();
        for (k, v, c) in iter {
            map.add(k, v, Some(c));
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Arc;
    use crate::network::{HostDefinition, NetworkDefinition, PeerAddress, PeerSocketAddr};
    use std::assert_matches;

    fn net(cidr: &str) -> Network {
        cidr.parse::<NetworkDefinition>().unwrap().resolve_sync().unwrap()
    }

    fn peer(sock: &str) -> Arc<PeerAddress> {
        let sa: std::net::SocketAddr = sock.parse().unwrap();
        Arc::new(PeerAddress {
            definition: HostDefinition::Ip {
                addr: sa.ip(),
                port: Some(sa.port()),
            },
            sock_addr: PeerSocketAddr::Ip(sa),
        })
    }

    #[test]
    fn network_conditions_rank_and_fall_back() {
        let key = Key::new("k");
        let mut cs = Map::new();
        cs.add(key, Value::Int(0), None);
        cs.add(
            key,
            Value::Int(1),
            Some(Condition {
                local_network: Some(net("172.16.0.0/12")),
                ..Default::default()
            }),
        );
        cs.add(
            key,
            Value::Int(2),
            Some(Condition {
                peer_network: Some(net("10.0.0.0/8")),
                ..Default::default()
            }),
        );

        // Both networks match — peer_network outranks local_network.
        let both = AssocDescription {
            peer_addr: Some(peer("10.1.1.1:104")),
            local_addr: Some(peer("172.16.1.1:104")),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&both)), Some((Value::Int(2), ..)));

        // Only the local network matches.
        let local_only = AssocDescription {
            peer_addr: Some(peer("8.8.8.8:104")),
            local_addr: Some(peer("172.16.1.1:104")),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&local_only)), Some((Value::Int(1), ..)));

        // Neither network matches — fall back to the unconditional value.
        let neither = AssocDescription {
            peer_addr: Some(peer("8.8.8.8:104")),
            local_addr: Some(peer("8.8.4.4:104")),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&neither)), Some((Value::Int(0), ..)));

        // A network condition with no corresponding address can never match.
        let no_addr = AssocDescription::default();
        assert_matches!(cs.get_ranked(&key, Some(&no_addr)), Some((Value::Int(0), ..)));
    }

    #[test]
    fn equal_rank_prefers_newer_value() {
        let key = Key::new("k");
        let mut cs = Map::new();
        let cond = || Condition {
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        cs.add(key, Value::Int(1), Some(cond()));
        cs.add(key, Value::Int(2), Some(cond()));

        let assoc = AssocDescription {
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&assoc)), Some((Value::Int(2), ..)));
    }

    #[test]
    fn unconditional_lookup_ignores_conditional_entries() {
        let key = Key::new("k");
        let mut cs = Map::new();
        cs.add(
            key,
            Value::Int(1),
            Some(Condition {
                peer_aet: Some("PEER".into()),
                ..Default::default()
            }),
        );
        // No unconditional value exists, so a no-assoc lookup finds nothing.
        assert!(cs.get_ranked(&key, None).is_none());
        // A missing key yields nothing in either mode.
        assert!(cs.get_ranked(&Key::new("absent"), None).is_none());
        assert!(
            cs.get_ranked(&Key::new("absent"), Some(&AssocDescription::default()))
                .is_none()
        );
    }

    #[test]
    fn conditional_lookup_prefers_most_specific() {
        let key = Key::new("test.lookup");
        let mut cs = Map::new();

        // Generic fallback (unconditional).
        cs.add(key, Value::Int(0), None);
        // More specific: matches a particular peer AET.
        cs.add(
            key,
            Value::Int(1),
            Some(Condition {
                peer_aet: Some("PEER".into()),
                ..Default::default()
            }),
        );
        // Most specific
        cs.add(
            key,
            Value::Int(2),
            Some(Condition {
                peer_aet: Some("PEER".into()),
                local_aet: Some("LOCAL".into()),
                ..Default::default()
            }),
        );

        // Most specific match
        let assoc = AssocDescription {
            peer_aet: Some("PEER".into()),
            local_aet: Some("LOCAL".into()),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&assoc)), Some((Value::Int(2), ..)));

        // Less specific match
        let assoc = AssocDescription {
            peer_aet: Some("PEER".into()),
            local_aet: Some("LOCAL_OTHER".into()),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&assoc)), Some((Value::Int(1), ..)));

        // Non-specific match
        let assoc = AssocDescription {
            peer_aet: Some("PEER_OTHER".into()),
            local_aet: Some("LOCAL_OTHER".into()),
            ..Default::default()
        };
        assert_matches!(cs.get_ranked(&key, Some(&assoc)), Some((Value::Int(0), ..)));

        let assoc = AssocDescription { ..Default::default() };
        assert_matches!(cs.get_ranked(&key, Some(&assoc)), Some((Value::Int(0), ..)));
        assert_matches!(cs.get_ranked(&key, None), Some((Value::Int(0), ..)));
    }
}
