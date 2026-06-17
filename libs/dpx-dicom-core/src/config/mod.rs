//! Application configuration system.
//!
//! The configuration system separates three concerns:
//!
//! - **Metadata** ([`meta`]) — descriptors that let any key be validated,
//!   edited in a GUI/TUI and documented without hard-coding it. Applications
//!   extend the surface by submitting [`StaticRegistry`] batches via `inventory`.
//! - **Values** ([`value`]) — the dynamically-typed [`Value`] payloads.
//! - **Settings** ([`settings`]) — loaded data, both unconditional
//!   ([`Settings`]) and association-aware ([`ConditionalSettings`]).
//!
//! Validation happens in two phases: phase one checks each value against its
//! [`ValueMeta`] descriptor (see [`ValueMeta::validate`]); phase two (added
//! later) checks cross-key consistency.

pub mod complex;
pub mod manager;
pub mod meta;
pub mod settings;
pub mod value;

pub use complex::{ComplexType, ConfigNode};
pub use manager::{Config, ConfigBuilder};
pub use meta::{Concept, Key, KeyMeta, MaybeGenerated, Registry, StaticRegistry, ValueMeta};
pub use settings::{ConditionalKey, ConditionalSettings, MatchAttributes, Settings};
pub use value::{Value, ValueFile};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_validation_respects_length_and_pattern() {
        let meta = ValueMeta::String {
            regexp: Some(r"^[A-Z]+$"),
            min_length: Some(2),
            max_length: Some(4),
        };
        assert!(meta.validate(&Value::String("ABC".into())).is_ok());
        // too short
        assert!(meta.validate(&Value::String("A".into())).is_err());
        // too long
        assert!(meta.validate(&Value::String("ABCDE".into())).is_err());
        // pattern mismatch
        assert!(meta.validate(&Value::String("abc".into())).is_err());
    }

    #[test]
    fn int_range_is_enforced() {
        let meta = ValueMeta::Int {
            min: Some(0),
            max: Some(10),
        };
        assert!(meta.validate(&Value::Int(5)).is_ok());
        assert!(meta.validate(&Value::Int(-1)).is_err());
        assert!(meta.validate(&Value::Int(11)).is_err());
    }

    #[test]
    fn enum_membership_is_checked() {
        static CHOICES: [(u32, Concept); 2] = [(1, Concept::new("a", "A", None)), (2, Concept::new("b", "B", None))];
        let meta = ValueMeta::Enum {
            values: MaybeGenerated::Static(&CHOICES),
        };
        assert!(meta.validate(&Value::Enum(1)).is_ok());
        assert!(meta.validate(&Value::Enum(3)).is_err());
    }

    #[test]
    fn type_mismatch_is_rejected() {
        let meta = ValueMeta::Bool;
        let err = meta.validate(&Value::Int(1)).unwrap_err();
        assert_eq!(err.kind, crate::ErrorKind::Internal);
    }

    #[test]
    fn vec_validates_each_element() {
        static ITEM: ValueMeta = ValueMeta::Int {
            min: Some(0),
            max: None,
        };
        let meta = ValueMeta::Vec {
            items: &ITEM,
            min_length: Some(1),
            max_length: Some(3),
            stride: None,
        };
        assert!(meta.validate(&Value::Vec(vec![Value::Int(1), Value::Int(2)])).is_ok());
        // element out of range
        assert!(meta.validate(&Value::Vec(vec![Value::Int(-1)])).is_err());
        // too many items
        let many4 = Value::Vec(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]);
        assert!(meta.validate(&many4).is_err());
        // stride is one
        let meta = ValueMeta::Vec {
            items: &ITEM,
            min_length: None,
            max_length: None,
            stride: Some(1),
        };
        assert!(meta.validate(&many4).is_ok());
        let many3 = Value::Vec(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert!(meta.validate(&many3).is_ok());
        let empty = Value::Vec(Vec::new());
        assert!(meta.validate(&empty).is_ok());
        // stride is even
        let meta = ValueMeta::Vec {
            items: &ITEM,
            min_length: None,
            max_length: None,
            stride: Some(2),
        };
        assert!(meta.validate(&many4).is_ok());
        assert!(meta.validate(&many3).is_err());
        assert!(meta.validate(&empty).is_ok());
    }

    #[test]
    fn conditional_lookup_prefers_most_specific() {
        let key = Key::new("test", 1);
        let mut cs = ConditionalSettings::new();

        // Generic fallback (unconditional).
        cs.add(ConditionalKey::unconditional(key), Value::Int(0));
        // More specific: matches a particular peer AET.
        cs.add(
            ConditionalKey {
                key,
                peer_aet: Some("PEER".into()),
                ..ConditionalKey::unconditional(key)
            },
            Value::Int(1),
        );

        let attrs = MatchAttributes {
            peer_aet: Some("PEER"),
            ..Default::default()
        };
        let got = cs.get(&key, &attrs).unwrap();
        assert!(matches!(got, Value::Int(1)));

        // A different peer falls back to the unconditional entry.
        let other = MatchAttributes {
            peer_aet: Some("OTHER"),
            ..Default::default()
        };
        assert!(matches!(cs.get(&key, &other).unwrap(), Value::Int(0)));
    }

    #[test]
    fn conditional_scoring_respects_attribute_priority() {
        use std::net::{IpAddr, Ipv4Addr};

        const KEY: Key = Key::new("test", 9);
        let peer_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let local_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        // An association that matches every dimension, so candidate selection
        // is decided purely by which attributes each candidate constrains.
        let attrs = MatchAttributes {
            peer_aet: Some("PEER"),
            local_aet: Some("LOCAL"),
            peer_ip: Some(peer_ip),
            local_ip: Some(local_ip),
            local_port: Some(104),
        };

        // A candidate constraining only `peer_aet` (score 16) must outrank a
        // candidate constraining every *lower* dimension at once
        // (local_aet + peer_ip + local_ip + local_port = 8+4+2+1 = 15). This is
        // the property that makes matching a strict priority, not additive.
        let only_peer_aet = ConditionalKey {
            key: KEY,
            peer_aet: Some("PEER".into()),
            ..ConditionalKey::unconditional(KEY)
        };
        let all_lower = ConditionalKey {
            key: KEY,
            local_aet: Some("LOCAL".into()),
            peer_ip: Some(peer_ip),
            local_ip: Some(local_ip),
            local_port: Some(104),
            ..ConditionalKey::unconditional(KEY)
        };

        let mut cs = ConditionalSettings::new();
        cs.add(all_lower.clone(), Value::Int(15));
        cs.add(only_peer_aet.clone(), Value::Int(16));
        assert!(
            matches!(cs.get(&KEY, &attrs), Some(Value::Int(16))),
            "peer_aet must outrank all lower dimensions combined"
        );

        // Adding more matching dimensions on top of the same highest one raises
        // the score: peer_aet + local_aet (24) beats peer_aet alone (16).
        let peer_and_local_aet = ConditionalKey {
            key: KEY,
            peer_aet: Some("PEER".into()),
            local_aet: Some("LOCAL".into()),
            ..ConditionalKey::unconditional(KEY)
        };
        let mut cs = ConditionalSettings::new();
        cs.add(only_peer_aet.clone(), Value::Int(16));
        cs.add(peer_and_local_aet, Value::Int(24));
        assert!(
            matches!(cs.get(&KEY, &attrs), Some(Value::Int(24))),
            "more matching dimensions must win within the same top priority"
        );
    }

    #[test]
    fn conditional_excludes_absent_or_mismatched_attributes() {
        const KEY: Key = Key::new("test", 10);

        let specific = ConditionalKey {
            key: KEY,
            peer_aet: Some("PEER".into()),
            ..ConditionalKey::unconditional(KEY)
        };
        let mut cs = ConditionalSettings::new();
        cs.add(ConditionalKey::unconditional(KEY), Value::Int(0));
        cs.add(specific, Value::Int(1));

        // Attribute the candidate constrains is absent from the association:
        // the candidate is excluded, the unconditional entry remains.
        let absent = MatchAttributes::default();
        assert!(matches!(cs.get(&KEY, &absent), Some(Value::Int(0))));

        // Attribute present but unequal: also excluded.
        let mismatch = MatchAttributes {
            peer_aet: Some("OTHER"),
            ..Default::default()
        };
        assert!(matches!(cs.get(&KEY, &mismatch), Some(Value::Int(0))));

        // Attribute present and equal: the specific candidate wins.
        let matching = MatchAttributes {
            peer_aet: Some("PEER"),
            ..Default::default()
        };
        assert!(matches!(cs.get(&KEY, &matching), Some(Value::Int(1))));
    }

    // ── Complex application-defined types ─────────────────────────────────────

    use crate::Arc;
    use std::any::Any;

    #[derive(Debug, PartialEq)]
    struct Port(u16);

    struct PortType;
    impl ComplexType for PortType {
        fn name(&self) -> &'static str {
            "port"
        }
        fn decode(&self, node: &ConfigNode) -> crate::error::Result<Arc<dyn Any + Send + Sync>> {
            let n = node
                .as_int()
                .ok_or_else(|| crate::dicom_err!(InvalidData, "port expects an integer"))?;
            Ok(Arc::new(Port(u16::try_from(n).map_err(|_| {
                crate::dicom_err!(InvalidData, "port out of range")
            })?)))
        }
        fn encode(&self, value: &dyn Any) -> crate::error::Result<ConfigNode> {
            let p = value
                .downcast_ref::<Port>()
                .ok_or_else(|| crate::dicom_err!(Internal, "port got wrong value type"))?;
            Ok(ConfigNode::Int(p.0 as i64))
        }
        fn validate(&self, value: &dyn Any) -> crate::error::Result<()> {
            let p = value
                .downcast_ref::<Port>()
                .ok_or_else(|| crate::dicom_err!(Internal, "port got wrong value type"))?;
            if p.0 == 0 {
                return Err(crate::dicom_err!(InvalidData, "port must not be zero"));
            }
            Ok(())
        }
    }

    static PORT_TYPE: PortType = PortType;

    #[test]
    fn complex_type_round_trips_through_config_node() {
        let ty: &'static dyn ComplexType = &PORT_TYPE;
        let decoded = ty.decode(&ConfigNode::Int(104)).unwrap();
        assert_eq!(decoded.downcast_ref::<Port>(), Some(&Port(104)));
        assert_eq!(ty.encode(decoded.as_ref()).unwrap(), ConfigNode::Int(104));
    }

    #[test]
    fn complex_value_meta_delegates_validation_to_type() {
        let meta = ValueMeta::Complex {
            ty: &PORT_TYPE,
            limits: &[],
        };
        let good: Arc<dyn Any + Send + Sync> = Arc::new(Port(104));
        assert!(meta.validate(&Value::Complex(good)).is_ok());

        let bad: Arc<dyn Any + Send + Sync> = Arc::new(Port(0));
        assert!(meta.validate(&Value::Complex(bad)).is_err());
    }
}
