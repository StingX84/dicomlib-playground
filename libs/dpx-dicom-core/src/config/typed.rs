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
use crate::{context::Context, network::AssocDescription};

/// Marker for a non-nullable key: reads return `T`.
pub enum Req {}
/// Marker for a nullable key: reads return `Option<T>`.
pub enum Opt {}

/// Type-level marker over the value type and nullability, carried by
/// [`TypedKey`] without owning either. Uses `fn() -> _` so the key stays
/// `Send + Sync + Copy` regardless of `T`/`N`.
type Marker<T, N> = PhantomData<(fn() -> T, fn() -> N)>;

/// A configuration key carrying its value type `T`, nullability `N` and whether
/// it is association-matched. Construct via [`TypedKey::new`] (or the `config!`
/// macro); read via [`get`](TypedKey::get) / [`get_for`](TypedKey::get_for).
#[derive(Copy, Clone)]
pub struct TypedKey<T, N = Req> {
    key: Key,
    conditional: bool,
    _p: Marker<T, N>,
}

impl<T, N> TypedKey<T, N> {
    /// Creates a typed handle for the dotted store `path`. `conditional` selects
    /// the association-matched store on read.
    pub const fn new(path: &'static str, conditional: bool) -> TypedKey<T, N> {
        TypedKey {
            key: Key::new(path),
            conditional,
            _p: PhantomData,
        }
    }

    /// The untyped key identity.
    pub const fn key(&self) -> Key {
        self.key
    }

    /// Whether this key resolves against the association-matched store.
    pub const fn conditional(&self) -> bool {
        self.conditional
    }
}

// ── Reading ───────────────────────────────────────────────────────────────────

impl<T: ValueRef, N> TypedKey<T, N> {
    /// Resolves against a single layer using an explicit condition.
    fn resolve<'c>(
        &self,
        conf: &'c impl ConfigValues,
        assoc: Option<&AssocDescription>,
    ) -> Option<<T as ValueRef>::Ref<'c>> {
        if self.conditional {
            conf.config_get_as::<T>(&self.key, assoc)
        } else {
            conf.config_get_as::<T>(&self.key, None)
        }
    }

    /// Resolves against a single layer, deriving the condition from the current
    /// [`Context`]. Unconditional keys never touch the context.
    fn resolve_current<'c>(&self, conf: &'c impl ConfigValues) -> Option<<T as ValueRef>::Ref<'c>> {
        let assoc = if self.conditional {
            Context::with_current(|ctx| ctx.assoc().cloned())
        } else {
            None
        };
        conf.config_get_as::<T>(&self.key, assoc.as_deref())
    }
}

impl<T: ValueRef> TypedKey<T, Req> {
    /// Reads the value, deriving the condition from the current [`Context`].
    pub fn get<'c>(&self, conf: &'c impl ConfigValues) -> <T as ValueRef>::Ref<'c> {
        self.resolve_current(conf).unwrap_or_else(|| missing(self.key))
    }

    /// Reads the value, matching conditionals against `src`.
    pub fn get_for<'c>(
        &self,
        conf: &'c impl ConfigValues,
        src: impl Into<Option<&'c AssocDescription>>,
    ) -> <T as ValueRef>::Ref<'c> {
        self.resolve(conf, src.into()).unwrap_or_else(|| missing(self.key))
    }
}

impl<T: ValueRef> TypedKey<T, Opt> {
    /// Reads the value, deriving the condition from the current [`Context`].
    pub fn get<'c>(&self, conf: &'c impl ConfigValues) -> Option<<T as ValueRef>::Ref<'c>> {
        self.resolve_current(conf)
    }

    /// Reads the value, matching conditionals against `src`.
    pub fn get_for<'c>(
        &self,
        conf: &'c impl ConfigValues,
        src: impl Into<Option<&'c AssocDescription>>,
    ) -> Option<<T as ValueRef>::Ref<'c>> {
        self.resolve(conf, src.into())
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
        GLOBAL_LAYER_ID, Object, Value,
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
        KeyMetaBuilder::new(Key::new(K_LABEL), build::String::new().nullable().build()).build(),
        // Nullable but with a default: an Opt read falls back to Some(default).
        KeyMetaBuilder::new(Key::new(K_RETRIES), build::Int::new().nullable().build())
            .default(|| Value::Int(3))
            .build(),
    ];

    config_object_meta!( fn test_object_meta() = &METAS );

    fn timeout() -> TypedKey<Duration, Req> {
        TypedKey::new(K_TIMEOUT, false)
    }
    fn max_key() -> TypedKey<i64, Req> {
        TypedKey::new(K_MAX, true)
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
        Object::new(GLOBAL_LAYER_ID.clone(), test_object_meta(), Map::from_iter(keys))
    }

    #[test]
    fn plain_reads_explicit_value_by_value() {
        assert_eq!(timeout().get(&populated()), Duration::from_secs(30));
    }

    #[test]
    fn plain_falls_back_to_default() {
        let cfg = Object::new_empty(GLOBAL_LAYER_ID.clone(), test_object_meta());
        assert_eq!(timeout().get(&cfg), Duration::from_secs(10));
    }

    #[test]
    fn conditional_get_for_selects_by_condition() {
        let cfg = populated();
        let peer = AssocDescription {
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        assert_eq!(max_key().get_for(&cfg, Some(&peer)), 99);
        let other = AssocDescription {
            peer_aet: Some("OTHER".into()),
            ..Default::default()
        };
        assert_eq!(max_key().get_for(&cfg, Some(&other)), 7);
    }

    #[test]
    fn conditional_falls_back_to_default() {
        let cfg = Object::new_empty(GLOBAL_LAYER_ID, test_object_meta());
        assert_eq!(max_key().get_for(&cfg, None), 5);
    }

    fn label() -> TypedKey<String, Opt> {
        TypedKey::new(K_LABEL, false)
    }
    fn retries() -> TypedKey<i64, Opt> {
        TypedKey::new(K_RETRIES, false)
    }

    #[test]
    fn opt_reads_explicit_value() {
        let cfg = Object::new(
            GLOBAL_LAYER_ID.clone(),
            test_object_meta(),
            Map::from_iter([(Key::new(K_LABEL), Value::String("hi".into()))]),
        );
        assert_eq!(label().get(&cfg), Some("hi"));
    }

    #[test]
    fn opt_without_value_or_default_is_none() {
        let cfg = Object::new_empty(GLOBAL_LAYER_ID.clone(), test_object_meta());
        assert_eq!(label().get(&cfg), None);
        assert_eq!(label().get_for(&cfg, None), None);
    }

    #[test]
    fn opt_falls_back_to_default() {
        let cfg = Object::new_empty(GLOBAL_LAYER_ID.clone(), test_object_meta());
        assert_eq!(retries().get(&cfg), Some(3));
    }
}
