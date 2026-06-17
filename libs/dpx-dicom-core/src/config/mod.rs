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

pub mod manager;
pub mod meta;
pub mod settings;
pub mod value;

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
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        let got = cs.get(&key, &attrs).unwrap();
        assert!(matches!(got, Value::Int(1)));

        // A different peer falls back to the unconditional entry.
        let other = MatchAttributes {
            peer_aet: Some("OTHER".into()),
            ..Default::default()
        };
        assert!(matches!(cs.get(&key, &other).unwrap(), Value::Int(0)));
    }
}
