//! The hot-swappable config manager.

use super::Object;
use crate::error::{ErrContext, Result};
use crate::event::{Event, EventObserver};
use crate::{Arc, Context};
use std::sync::LazyLock;

/// The single source of truth for getting and setting the global base [`Object`].
///
/// `GlobalConfig` is a singleton.
/// Read the live configuration with [`current`](Self::current), replace it directly with
/// [`set_forced`](Self::set_forced), or apply a two-phase reload that subscribers can veto with
/// [`set_transactional`](Self::set_transactional). Modules observe reloads by subscribing to the three
/// phase events; all methods are associated functions operating on that
/// singleton.
///
/// Reloads are applied in two phases so a module that owns external resources
/// (open sockets, thread pools, caches) can validate a candidate configuration
/// before it goes live and refuse it without leaving the system half-applied.
///
/// 1. [`on_prepare`](Self::on_prepare) fires for every subscriber. A handler that
///    returns an error vetoes the whole reload.
/// 2. If prepare succeeds, the new config goes live and
///    [`on_commit`](Self::on_commit) fires.
/// 3. If a handler vetoed, [`on_abort`](Self::on_abort) fires so subscribers can
///    release anything they reserved, and the old config stays live.
///
/// All handlers are synchronous and must not block for long; they run on the
/// thread that called [`set_transactional`](Self::set_transactional).
pub struct GlobalConfig {
    on_prepare: Event<Object>,
    on_commit: Event<Object>,
    on_abort: Event<Object>,
}

static MANAGER: LazyLock<GlobalConfig> = LazyLock::new(|| GlobalConfig {
    on_prepare: Event::new(),
    on_commit: Event::new(),
    on_abort: Event::new(),
});

impl GlobalConfig {
    /// Returns the live global configuration. The [`Context`] root always carries
    /// a base configuration, so this never returns `None`. Lock-free.
    pub fn current() -> Arc<Object> {
        Context::global_config()
    }

    /// Publishes `candidate` as the global base configuration using a two-phase commit.
    ///
    /// If a [`on_prepare`](Self::on_prepare) handler vetoes, the
    /// [`on_abort`](Self::on_abort) subscribers are notified, the live
    /// configuration is left unchanged, and the veto error is returned.
    ///
    /// See also: [`set_forced`](Self::set_forced)
    pub fn set_transactional(candidate: Arc<Object>) -> Result {
        if let Err(e) = MANAGER.on_prepare.fire(candidate.as_ref()) {
            let _ = MANAGER.on_abort.fire(candidate.as_ref());
            return Err(e).err_context("configuration reload vetoed");
        }

        Context::publish_global_config(Arc::clone(&candidate));
        let _ = MANAGER.on_commit.fire(candidate.as_ref());
        Ok(())
    }

    /// Atomically replaces the global base configuration without running the
    /// two-phase reload protocol.
    ///
    /// See also: [`set_transactional`](Self::set_transactional)
    pub fn set_forced(config: Arc<Object>) {
        Context::publish_global_config(config);
    }

    /// Subscribe-only handle to the *prepare* phase.
    ///
    /// A handler validates the candidate configuration and reserves whatever the
    /// module needs to switch to it; returning an error vetoes the reload.
    pub fn on_prepare() -> EventObserver<Object> {
        MANAGER.on_prepare.observer()
    }

    /// Subscribe-only handle to the *commit* phase, fired once the candidate has
    /// gone live. A commit handler cannot veto — the configuration is already
    /// published — so any error it returns does not roll the reload back.
    pub fn on_commit() -> EventObserver<Object> {
        MANAGER.on_commit.observer()
    }

    /// Subscribe-only handle to the *abort* phase, fired when a reload was vetoed
    /// during prepare. Unlike commit, this notifies every abort subscriber, so a
    /// handler must tolerate being called even when its own prepare did not run.
    pub fn on_abort() -> EventObserver<Object> {
        MANAGER.on_abort.observer()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::{ConfigValues, GLOBAL_LAYER_ID, Key, Map, Value, meta::*};
    use super::*;
    use crate::{ensure, config_object_meta};
    use crate::event::Subscription;
    use super::super::subst::lock_global_for_test;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static VERSION_KEY: Key = Key::new("version");
    static VERSION_METAS: &[KeyMeta] =
        &[KeyMetaBuilder::new(VERSION_KEY, build::Int::new().build()).runtime().build()];

    config_object_meta!( fn object_meta() = &VERSION_METAS );

    fn config_object(version: i64) -> Arc<Object> {
        Arc::new(Object::new(
            GLOBAL_LAYER_ID.clone(),
            object_meta(),
            Map::from_iter([(VERSION_KEY, Value::Int(version))]),
        ))
    }

    fn current_version() -> i64 {
        GlobalConfig::current()
            .config_get_as::<i64>(&VERSION_KEY, None)
            .expect("version should be int")
    }

    #[derive(Default)]
    struct Recorder {
        prepared: AtomicUsize,
        committed: AtomicUsize,
        aborted: AtomicUsize,
        veto: bool,
    }

    impl Recorder {
        /// Subscribes this recorder to all three phase events. The returned
        /// handles must be kept alive for the subscriptions to stay active.
        fn subscribe(self: &Arc<Self>) -> [Subscription<Object>; 3] {
            let p = Arc::clone(self);
            let prepare = GlobalConfig::on_prepare().subscribe(move |_c: &Object| {
                p.prepared.fetch_add(1, Ordering::SeqCst);
                ensure!(!p.veto, Configuration, "vetoed");
                Ok(())
            });
            let c = Arc::clone(self);
            let commit = GlobalConfig::on_commit().subscribe(move |_c: &Object| {
                c.committed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
            let a = Arc::clone(self);
            let abort = GlobalConfig::on_abort().subscribe(move |_c: &Object| {
                a.aborted.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
            [prepare, commit, abort]
        }
    }

    #[test]
    fn successful_reload_prepares_commits_and_publishes() {
        let _guard = lock_global_for_test();
        GlobalConfig::set_forced(config_object(1));

        let sub = Arc::new(Recorder::default());
        let _subs = sub.subscribe();

        GlobalConfig::set_transactional(config_object(2)).expect("reload");

        assert_eq!(sub.prepared.load(Ordering::SeqCst), 1);
        assert_eq!(sub.committed.load(Ordering::SeqCst), 1);
        assert_eq!(sub.aborted.load(Ordering::SeqCst), 0);
        // The new config is now visible through the global context.
        assert_eq!(current_version(), 2);
    }

    #[test]
    fn veto_aborts_subscribers_and_keeps_old_config() {
        let _guard = lock_global_for_test();
        GlobalConfig::set_forced(config_object(1));

        let ok = Arc::new(Recorder::default());
        let bad = Arc::new(Recorder {
            veto: true,
            ..Default::default()
        });
        let _ok_subs = ok.subscribe();
        let _bad_subs = bad.subscribe();

        let err = GlobalConfig::set_transactional(config_object(2)).unwrap_err();
        assert_eq!(err.kind, crate::ErrorKind::Configuration);

        // The first subscriber prepared, the reload was vetoed, nothing committed.
        assert_eq!(ok.prepared.load(Ordering::SeqCst), 1);
        assert_eq!(ok.aborted.load(Ordering::SeqCst), 1);
        assert_eq!(ok.committed.load(Ordering::SeqCst), 0);
        // The vetoing subscriber prepared (and failed); abort fires for every
        // abort subscriber, so it is notified too.
        assert_eq!(bad.prepared.load(Ordering::SeqCst), 1);
        assert_eq!(bad.aborted.load(Ordering::SeqCst), 1);
        assert_eq!(bad.committed.load(Ordering::SeqCst), 0);
        // The global configuration is unchanged.
        assert_eq!(current_version(), 1);
    }

    #[test]
    fn dropped_subscriptions_are_inactive() {
        let _guard = lock_global_for_test();
        GlobalConfig::set_forced(config_object(1));

        let sub = Arc::new(Recorder::default());
        drop(sub.subscribe());

        GlobalConfig::set_transactional(config_object(2)).expect("reload");
        assert_eq!(current_version(), 2);
        // Dropping the handles cancelled every subscription.
        assert_eq!(sub.prepared.load(Ordering::SeqCst), 0);
        assert_eq!(sub.committed.load(Ordering::SeqCst), 0);
        assert!(!GlobalConfig::on_prepare().has_subscribers());
    }
}
