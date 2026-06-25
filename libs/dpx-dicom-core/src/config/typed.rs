//! Typed, compile-time-checked access to configuration values.
//!
//! A [`TypedKey`] is a handle that carries its value's Rust type `T`, its
//! nullability `N` ([`Req`]/[`Opt`]) and whether it is conditional.
//!
//! Return shape is driven by the markers: a [`Req`] key yields `T` directly (it
//! panics only if the ObjectMeta invariant — every key has a default — is broken),
//! an [`Opt`] key yields `Option<T>`. The concrete `T` view is chosen by
//! [`ValueRef`]: `Copy` scalars are returned by value, heap types by reference.

use std::marker::PhantomData;

use super::{ConfigValues, Key, ValueRef};
use crate::network::AssocDescription;

/// Marker for a non-optional key: reads return `T`.
pub enum Req {}
/// Marker for a optional key: reads return `Option<T>`.
pub enum Opt {}

/// Type-level marker over the value type and nullability, carried by
/// [`TypedKey`] without owning either. Uses `fn() -> _` so the key stays
/// `Send + Sync + Copy` regardless of `T`/`N`.
type Marker<T, N> = PhantomData<(fn() -> T, fn() -> N)>;

/// A configuration key carrying its value type `T`, nullability `N`.
/// Construct via [`TypedKey::new`] (or the `declare_config_object!`
/// macro); read via [`get`](TypedKey::get).
#[derive(Copy, Clone)]
pub struct TypedKey<T, N = Req> {
    key: Key,
    _p: Marker<T, N>,
}

impl<T, N> TypedKey<T, N> {
    /// Creates a typed handle for the dotted store `path`.
    pub const fn new(path: &'static str) -> TypedKey<T, N> {
        TypedKey {
            key: Key::new(path),
            _p: PhantomData,
        }
    }

    /// The untyped key identity.
    pub const fn key(&self) -> Key {
        self.key
    }
}

// ── Reading ───────────────────────────────────────────────────────────────────

impl<T: ValueRef> TypedKey<T, Req> {
    /// Reads the value, matching conditionals against `src`.
    pub fn get<'c>(&self, obj: &'c impl ConfigValues, assoc: Option<&'c AssocDescription>) -> <T as ValueRef>::Ref<'c> {
        obj.config_get_as::<T>(&self.key, assoc)
            .unwrap_or_else(|| missing(self.key))
    }
}

impl<T: ValueRef> TypedKey<T, Opt> {
    /// Reads the value, deriving the condition from the specified [`Context`].
    pub fn get<'c>(
        &self,
        ctx: &'c impl ConfigValues,
        assoc: Option<&'c AssocDescription>,
    ) -> Option<<T as ValueRef>::Ref<'c>> {
        ctx.config_get_as::<T>(&self.key, assoc)
    }
}

#[cold]
#[inline(never)]
fn missing(key: Key) -> ! {
    panic!(
        "configuration key {:?} has no value and no default — MetaObject invariant violated",
        key.as_str()
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Object, Value,
        map::{Condition, Conditionals, Map},
        meta::{KeyMeta, KeyMetaBuilder, build},
    };
    use crate::config_object_meta;

    use std::time::Duration;

    const K_TIMEOUT: &str = "timeout";
    const K_MAX: &str = "max";
    const K_LABEL: &str = "label";
    const K_RETRIES: &str = "retries";

    static METAS: [KeyMeta; 4] = [
        KeyMetaBuilder::new(Key::new(K_TIMEOUT), build::Duration::new().build())
            .default(|| Value::Duration(Duration::from_secs(10)))
            .build(),
        KeyMetaBuilder::new(Key::new(K_MAX), build::Int::new().build())
            .conditional()
            .default(|| Value::Int(5))
            .build(),
        // Nullable, no default: an Opt read yields None when unset.
        KeyMetaBuilder::new(Key::new(K_LABEL), build::String::new().optional().build()).build(),
        // Nullable but with a default: an Opt read falls back to Some(default).
        KeyMetaBuilder::new(Key::new(K_RETRIES), build::Int::new().optional().build())
            .default(|| Value::Int(3))
            .build(),
    ];

    config_object_meta!( fn test_object_meta() = &METAS );

    fn timeout() -> TypedKey<Duration, Req> {
        TypedKey::new(K_TIMEOUT)
    }
    fn max_key() -> TypedKey<i64, Req> {
        TypedKey::new(K_MAX)
    }

    fn populated() -> Object {
        let keys = [
            (
                Key::new(K_TIMEOUT),
                Conditionals([(Value::Duration(Duration::from_secs(30)), Condition::default())].into()),
            ),
            (
                Key::new(K_MAX),
                Conditionals(
                    [
                        (
                            Value::Int(99),
                            Condition {
                                peer_aet: Some("PEER".into()),
                                ..Default::default()
                            },
                        ),
                        (Value::Int(7), Condition::default()),
                    ]
                    .into(),
                ),
            ),
        ];
        Object::new(test_object_meta(), Map::from_iter(keys))
    }

    #[test]
    fn plain_reads_explicit_value_by_value() {
        assert_eq!(timeout().get(&populated(), None), Duration::from_secs(30));
    }

    #[test]
    fn plain_falls_back_to_default() {
        let cfg = Object::new_empty(test_object_meta());
        assert_eq!(timeout().get(&cfg, None), Duration::from_secs(10));
    }

    #[test]
    fn conditional_get_for_selects_by_condition() {
        let cfg = populated();
        let peer = AssocDescription {
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        assert_eq!(max_key().get(&cfg, Some(&peer)), 99);
        let other = AssocDescription {
            peer_aet: Some("OTHER".into()),
            ..Default::default()
        };
        assert_eq!(max_key().get(&cfg, Some(&other)), 7);
    }

    #[test]
    fn conditional_falls_back_to_default() {
        let cfg = Object::new_empty(test_object_meta());
        assert_eq!(max_key().get(&cfg, None), 5);
    }

    fn label() -> TypedKey<String, Opt> {
        TypedKey::new(K_LABEL)
    }
    fn retries() -> TypedKey<i64, Opt> {
        TypedKey::new(K_RETRIES)
    }

    #[test]
    fn opt_reads_explicit_value() {
        let cfg = Object::new(
            test_object_meta(),
            Map::from_iter([(Key::new(K_LABEL), Value::String("hi".into()))]),
        );
        assert_eq!(label().get(&cfg, None), Some("hi"));
    }

    #[test]
    fn opt_without_value_or_default_is_none() {
        let cfg = Object::new_empty(test_object_meta());
        assert_eq!(label().get(&cfg, None), None);
    }

    #[test]
    fn opt_falls_back_to_default() {
        let cfg = Object::new_empty(test_object_meta());
        assert_eq!(retries().get(&cfg, None), Some(3));
    }
}
