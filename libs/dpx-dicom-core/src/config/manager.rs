//! The assembled, read-only configuration snapshot and its hot-swappable manager.
//!
//! A [`Config`] is one immutable layer: the unconditional [`Settings`], the
//! association-aware [`ConditionalSettings`] overlay, and the [`Registry`] that
//! supplies metadata and defaults. Layers are stacked through [`Context`](crate::context).
//!
//! [`GlobalConfig`] is the single source of truth for getting and setting the
//! *global* base configuration, which is owned by the [`Context`] root and which
//! every reader reaches through
//! [`Context::config_value`](crate::Context::config_value). `GlobalConfig` does
//! not hold a configuration of its own; it reads from and publishes into the
//! context. Reloads use a two-phase commit driven by three [`Event`]s
//! ([`on_prepare`](GlobalConfig::on_prepare) →
//! [`on_commit`](GlobalConfig::on_commit) /
//! [`on_abort`](GlobalConfig::on_abort)) so a module that cannot accept a new
//! configuration can veto the swap before it goes live.

use super::{Key, Registry, Value, settings::ConditionalSettings, settings::MatchAttributes, settings::Settings};
use crate::error::{ErrContext, Result};
use crate::event::{Event, EventObserver};
use crate::{Arc, Context};
use std::sync::LazyLock;

/// An immutable configuration layer.
///
/// Resolution within a single layer is: the best-matching conditional value,
/// then the unconditional value, then the registry default. Cross-layer
/// resolution is handled by [`Context`].
#[derive(Debug, Clone)]
pub struct Config {
    registry: Arc<Registry>,
    settings: Settings,
    conditional: ConditionalSettings,
    version: u32,
}

impl Config {
    /// Begins building a layer backed by the given metadata `registry`.
    pub fn builder(registry: Arc<Registry>) -> ConfigBuilder {
        ConfigBuilder {
            registry,
            settings: Settings::new(),
            conditional: ConditionalSettings::new(),
            version: 0,
        }
    }

    /// The metadata registry this layer resolves keys and defaults against.
    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }

    /// The schema version this layer was produced for (used by migrations).
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the explicitly-configured value for `key`, ignoring defaults.
    ///
    /// A matching conditional value takes precedence over the unconditional one.
    pub fn get_explicit(&self, key: &Key, attrs: &MatchAttributes) -> Option<&Value> {
        self.conditional.get(key, attrs).or_else(|| self.settings.get(key))
    }

    /// Returns the registry default for `key`, if any.
    pub fn default_of(&self, key: &Key) -> Option<&Value> {
        self.registry.default_value_of(key)
    }

    /// Resolves `key` within this single layer: explicit value, then default.
    pub fn get(&self, key: &Key, attrs: &MatchAttributes) -> Option<&Value> {
        self.get_explicit(key, attrs).or_else(|| self.default_of(key))
    }
}

/// Builder for an immutable [`Config`] layer.
pub struct ConfigBuilder {
    registry: Arc<Registry>,
    settings: Settings,
    conditional: ConditionalSettings,
    version: u32,
}

impl ConfigBuilder {
    pub fn settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    pub fn conditional(mut self, conditional: ConditionalSettings) -> Self {
        self.conditional = conditional;
        self
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn build(self) -> Config {
        Config {
            registry: self.registry,
            settings: self.settings,
            conditional: self.conditional,
            version: self.version,
        }
    }
}

// ── GlobalConfig ──────────────────────────────────────────────────────────────

/// The single source of truth for getting and setting the global base [`Config`].
///
/// `GlobalConfig` is a singleton: there is exactly one event hub per process,
/// matching the single global configuration owned by the [`Context`]. Read the
/// live configuration with [`current`](Self::current), replace it directly with
/// [`set`](Self::set), or apply a two-phase reload that subscribers can veto with
/// [`reload`](Self::reload). Modules observe reloads by subscribing to the three
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
/// thread that called [`reload`](Self::reload).
pub struct GlobalConfig {
    on_prepare: Event<Config>,
    on_commit: Event<Config>,
    on_abort: Event<Config>,
}

static MANAGER: LazyLock<GlobalConfig> = LazyLock::new(|| GlobalConfig {
    on_prepare: Event::new(),
    on_commit: Event::new(),
    on_abort: Event::new(),
});

impl GlobalConfig {
    /// Returns the live global configuration. The [`Context`] root always carries
    /// a base configuration, so this never returns `None`. Lock-free.
    pub fn current() -> Arc<Config> {
        Context::global_config()
    }

    /// Atomically replaces the global base configuration without running the
    /// two-phase reload protocol. Use [`reload`](Self::reload) when subscribers
    /// must be able to validate or veto the change.
    pub fn set(config: Arc<Config>) {
        Context::publish_global_config(config);
    }

    /// Subscribe-only handle to the *prepare* phase.
    ///
    /// A handler validates the candidate configuration and reserves whatever the
    /// module needs to switch to it; returning an error vetoes the reload.
    pub fn on_prepare() -> EventObserver<Config> {
        MANAGER.on_prepare.observer()
    }

    /// Subscribe-only handle to the *commit* phase, fired once the candidate has
    /// gone live. A commit handler cannot veto — the configuration is already
    /// published — so any error it returns does not roll the reload back.
    pub fn on_commit() -> EventObserver<Config> {
        MANAGER.on_commit.observer()
    }

    /// Subscribe-only handle to the *abort* phase, fired when a reload was vetoed
    /// during prepare. Unlike commit, this notifies every abort subscriber, so a
    /// handler must tolerate being called even when its own prepare did not run.
    pub fn on_abort() -> EventObserver<Config> {
        MANAGER.on_abort.observer()
    }

    /// Publishes `candidate` as the global base configuration using a two-phase
    /// commit, making it visible to every reader through the [`Context`].
    ///
    /// If a [`on_prepare`](Self::on_prepare) handler vetoes, the
    /// [`on_abort`](Self::on_abort) subscribers are notified, the live
    /// configuration is left unchanged, and the veto error is returned.
    pub fn reload(candidate: Config) -> Result<()> {
        let candidate = Arc::new(candidate);

        if let Err(e) = MANAGER.on_prepare.fire(candidate.as_ref()) {
            let _ = MANAGER.on_abort.fire(candidate.as_ref());
            return Err(e).err_context("configuration reload vetoed");
        }

        Context::publish_global_config(Arc::clone(&candidate));
        let _ = MANAGER.on_commit.fire(candidate.as_ref());
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom_err;
    use crate::event::Subscription;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, PoisonError};

    // The base configuration lives in the process-global `Context`, so these
    // tests share that state and must run serially.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        m.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn empty_config(version: u32) -> Config {
        Config::builder(Arc::new(Registry::new_empty()))
            .version(version)
            .build()
    }

    fn current_version() -> u32 {
        GlobalConfig::current().version()
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
        fn subscribe(self: &Arc<Self>) -> [Subscription<Config>; 3] {
            let p = Arc::clone(self);
            let prepare = GlobalConfig::on_prepare().subscribe(move |_c: &Config| {
                p.prepared.fetch_add(1, Ordering::SeqCst);
                if p.veto {
                    Err(dicom_err!(Configuration, "vetoed"))
                } else {
                    Ok(())
                }
            });
            let c = Arc::clone(self);
            let commit = GlobalConfig::on_commit().subscribe(move |_c: &Config| {
                c.committed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
            let a = Arc::clone(self);
            let abort = GlobalConfig::on_abort().subscribe(move |_c: &Config| {
                a.aborted.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
            [prepare, commit, abort]
        }
    }

    #[test]
    fn successful_reload_prepares_commits_and_publishes() {
        let _guard = lock(&SERIAL);
        GlobalConfig::set(Arc::new(empty_config(1)));

        let sub = Arc::new(Recorder::default());
        let _subs = sub.subscribe();

        GlobalConfig::reload(empty_config(2)).expect("reload");

        assert_eq!(sub.prepared.load(Ordering::SeqCst), 1);
        assert_eq!(sub.committed.load(Ordering::SeqCst), 1);
        assert_eq!(sub.aborted.load(Ordering::SeqCst), 0);
        // The new config is now visible through the global context.
        assert_eq!(current_version(), 2);
    }

    #[test]
    fn veto_aborts_subscribers_and_keeps_old_config() {
        let _guard = lock(&SERIAL);
        GlobalConfig::set(Arc::new(empty_config(1)));

        let ok = Arc::new(Recorder::default());
        let bad = Arc::new(Recorder {
            veto: true,
            ..Default::default()
        });
        let _ok_subs = ok.subscribe();
        let _bad_subs = bad.subscribe();

        let err = GlobalConfig::reload(empty_config(2)).unwrap_err();
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
        let _guard = lock(&SERIAL);
        GlobalConfig::set(Arc::new(empty_config(1)));

        let sub = Arc::new(Recorder::default());
        drop(sub.subscribe());

        GlobalConfig::reload(empty_config(2)).expect("reload");
        assert_eq!(current_version(), 2);
        // Dropping the handles cancelled every subscription.
        assert_eq!(sub.prepared.load(Ordering::SeqCst), 0);
        assert_eq!(sub.committed.load(Ordering::SeqCst), 0);
        assert!(!GlobalConfig::on_prepare().has_subscribers());
    }
}
