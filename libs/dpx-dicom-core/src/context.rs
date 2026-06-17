use crate::*;
use arc_swap::ArcSwap;
use std::{
    borrow::Cow, cell::Cell, fmt, future::Future, net::SocketAddr, pin::Pin, ptr::NonNull, sync::LazyLock, task,
};

thread_local! {
    static LOCAL_CTX: Cell<Option<NonNull<Context>>> = const { Cell::new(None) };
}

// The global root: always carries default dictionaries.
// ArcSwap allows lock-free reads and atomic full-Arc replacement.
static GLOBAL_CTX: LazyLock<ArcSwap<Context>> = LazyLock::new(|| {
    ArcSwap::from_pointee(Context {
        assoc: None,
        action: None,
        tag_dict: Some(Arc::new(tag::Dictionary::default())),
        uid_dict: Some(Arc::new(uid::Dictionary::default())),
        config: None,
        prev: None,
    })
});

// ── AssocDescription ──────────────────────────────────────────────────────────

/// Description of a DICOM association carried by [`Context`].
#[derive(Debug, Clone)]
pub struct AssocDescription {
    pub id: u64,
    pub peer_aet: String,
    pub local_aet: String,
    pub peer_addr: Option<SocketAddr>,
    pub local_addr: Option<SocketAddr>,
    pub is_incoming: bool,
    pub is_tls_secured: bool,
}

// ── ActionEntry (private) ─────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct ActionEntry {
    kind: &'static str,
    id: u64,
}

// ── Context ───────────────────────────────────────────────────────────────────

/// A layered operational context carried through sync and async call trees.
///
/// Each node patches zero or more fields on top of its parent chain; accessors
/// walk toward the root to find the nearest value. Build a new layer with
/// [`Context::extend()`] and activate it with [`ContextBuilder::provide`] or
/// [`ContextBuilder::scope`].
///
/// ```
/// use std::sync::Arc;
/// use dpx_dicom_core::context::{Context, AssocDescription};
///
/// let assoc = Arc::new(AssocDescription {
///     id: 1, peer_aet: "PEER".into(), local_aet: "LOCAL".into(),
///     peer_addr: None, local_addr: None,
///     is_incoming: true, is_tls_secured: false,
/// });
///
/// Context::extend().assoc(assoc).provide(|| {
///     Context::with_current(|ctx| {
///         assert!(ctx.assoc().is_some());
///     });
/// });
/// ```
pub struct Context {
    assoc: Option<Arc<AssocDescription>>,
    action: Option<ActionEntry>,
    tag_dict: Option<Arc<tag::Dictionary>>,
    uid_dict: Option<Arc<uid::Dictionary>>,
    config: Option<Arc<config::Config>>,
    prev: Option<Arc<Context>>,
}

impl Context {
    /// Returns the global root [`Context`] store.
    ///
    /// The global always carries default tag and UID dictionaries.
    /// Swap in a new root to change global defaults:
    /// ```
    /// use dpx_dicom_core::Context;
    /// // Context::global().store(Arc::new(...));
    /// let _ = Context::global();
    /// ```
    pub fn global() -> &'static ArcSwap<Context> {
        &GLOBAL_CTX
    }

    /// Creates a [`ContextBuilder`] extending the currently installed context.
    /// The current context becomes the parent of the new layer.
    pub fn extend() -> ContextBuilder {
        let prev = LOCAL_CTX.with(|cell| {
            cell.get()
                .map(|ptr| {
                    // SAFETY: `ptr` was written by `ContextBuilder::provide` or
                    // `ContextScope::poll`, both of which keep a live `Arc<Context>`
                    // for as long as the pointer remains in the cell.
                    // Incrementing the strong count creates a new owning Arc that
                    // shares the same allocation without invalidating the original.
                    unsafe {
                        Arc::increment_strong_count(ptr.as_ptr());
                        Arc::from_raw(ptr.as_ptr())
                    }
                })
                .unwrap_or_else(|| Arc::clone(&GLOBAL_CTX.load()))
        });
        ContextBuilder {
            ctx: Arc::new(Context {
                assoc: None,
                action: None,
                tag_dict: None,
                uid_dict: None,
                config: None,
                prev: Some(prev),
            }),
        }
    }

    /// Calls `f` with the currently installed [`Context`].
    /// Falls back to the global root context when nothing is installed on this thread.
    pub fn with_current<F, T>(f: F) -> T
    where
        F: FnOnce(&Context) -> T,
    {
        if let Some(ptr) = LOCAL_CTX.with(|cell| cell.get()) {
            return f(unsafe { ptr.as_ref() });
        }
        // Load the global Arc (lock-free), then install it temporarily so
        // nested extend() calls chain from the global correctly.
        let global = Arc::clone(&GLOBAL_CTX.load());
        LOCAL_CTX.with(|cell| {
            // SAFETY: `Arc::as_ptr` is non-null and carries provenance over the
            // whole `ArcInner` allocation (refcounts included), so a nested
            // `extend()` can reconstruct an `Arc` from this pointer. Deriving it
            // through `&Context` would narrow provenance to the data range and
            // make that reconstruction UB.
            let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&global) as *mut Context) };
            let old = cell.replace(Some(ptr));
            let rv = f(unsafe { ptr.as_ref() });
            cell.set(old);
            rv
        })
        // global Arc drops here, after thread-local is restored
    }

    // ── Chain accessors ───────────────────────────────────────────────────────

    /// Returns the nearest [`AssocDescription`] in the context chain, if any.
    pub fn assoc(&self) -> Option<&Arc<AssocDescription>> {
        self.find(|n| n.assoc.as_ref())
    }

    /// Returns the effective tag dictionary, walking the chain toward the
    /// global root which always carries a default.
    pub fn tag_dict(&self) -> &tag::Dictionary {
        self.find(|n| n.tag_dict.as_deref())
            .expect("Context chain missing tag dictionary — global root must have one")
    }

    /// Returns the effective UID dictionary, walking the chain toward the
    /// global root which always carries a default.
    pub fn uid_dict(&self) -> &uid::Dictionary {
        self.find(|n| n.uid_dict.as_deref())
            .expect("Context chain missing UID dictionary — global root must have one")
    }

    /// Returns the nearest configuration layer in the context chain, if any.
    pub fn config(&self) -> Option<&Arc<config::Config>> {
        self.find(|n| n.config.as_ref())
    }

    /// Resolves a configuration value for `key`, honouring the layered context.
    ///
    /// Layers are searched from innermost to outermost: the first layer that
    /// carries an *explicit* value for `key` (a matching conditional value, then
    /// the unconditional one) wins. Conditional matching uses the association of
    /// the active context (see [`Context::assoc`]). If no layer has an explicit
    /// value, the registry default of the innermost layer is returned.
    pub fn config_value(&self, key: &config::Key) -> Option<&config::Value> {
        let attrs = self.match_attributes();

        let mut innermost: Option<&config::Config> = None;
        let mut node = Some(self);
        while let Some(n) = node {
            if let Some(cfg) = n.config.as_deref() {
                if innermost.is_none() {
                    innermost = Some(cfg);
                }
                if let Some(value) = cfg.get_explicit(key, &attrs) {
                    return Some(value);
                }
            }
            node = n.prev.as_deref();
        }

        innermost?.default_of(key)
    }

    /// Derives [`MatchAttributes`] from the active association, used to select
    /// conditional configuration values.
    fn match_attributes(&self) -> config::MatchAttributes {
        match self.assoc() {
            Some(a) => config::MatchAttributes {
                peer_aet: Some(Cow::Owned(a.peer_aet.clone())),
                local_aet: Some(Cow::Owned(a.local_aet.clone())),
                peer_ip: a.peer_addr.map(|s| s.ip()),
                local_ip: a.local_addr.map(|s| s.ip()),
                local_port: a.local_addr.map(|s| s.port()),
            },
            None => config::MatchAttributes::default(),
        }
    }

    /// Iterates over all active actions from innermost to outermost.
    pub fn actions(&self) -> impl Iterator<Item = (&'static str, u64)> + '_ {
        ContextActionsIter { node: Some(self) }
    }

    /// Returns `true` if any action with the given `kind` is currently active.
    pub fn has_action(&self, kind: &'static str) -> bool {
        self.actions().any(|(k, _)| k == kind)
    }

    /// Returns `true` if an action with both the given `kind` and `id` is active.
    pub fn has_action_with_id(&self, kind: &'static str, id: u64) -> bool {
        self.actions().any(|(k, i)| k == kind && i == id)
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn find<'a, T>(&'a self, f: impl Fn(&'a Context) -> Option<T>) -> Option<T> {
        let mut node = self;
        loop {
            if let Some(v) = f(node) {
                return Some(v);
            }
            match &node.prev {
                Some(prev) => node = prev,
                None => return None,
            }
        }
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("assoc", &self.assoc)
            .field("action", &self.action.map(|a| (a.kind, a.id)))
            .field("has_tag_dict", &self.tag_dict.is_some())
            .field("has_uid_dict", &self.uid_dict.is_some())
            .field("has_config", &self.config.is_some())
            .field("prev", &self.prev.as_ref().map(|_| ".."))
            .finish()
    }
}

// ── ContextActionsIter ────────────────────────────────────────────────────────

struct ContextActionsIter<'a> {
    node: Option<&'a Context>,
}

impl Iterator for ContextActionsIter<'_> {
    type Item = (&'static str, u64);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.node?;
            self.node = node.prev.as_deref();
            if let Some(a) = node.action {
                return Some((a.kind, a.id));
            }
        }
    }
}

// ── ContextBuilder ────────────────────────────────────────────────────────────

/// A [`Context`] layer under construction. Obtained from [`Context::extend()`].
pub struct ContextBuilder {
    ctx: Arc<Context>,
}

impl ContextBuilder {
    fn ctx_mut(&mut self) -> &mut Context {
        Arc::get_mut(&mut self.ctx).expect("ContextBuilder: Arc unexpectedly shared")
    }

    /// Sets the association description for this context layer.
    pub fn assoc(mut self, assoc: Arc<AssocDescription>) -> Self {
        self.ctx_mut().assoc = Some(assoc);
        self
    }

    /// Adds an action entry to this context layer.
    pub fn action(mut self, kind: &'static str, id: u64) -> Self {
        self.ctx_mut().action = Some(ActionEntry { kind, id });
        self
    }

    /// Overrides the tag dictionary for this context layer and descendants.
    pub fn tag_dict(mut self, d: Arc<tag::Dictionary>) -> Self {
        self.ctx_mut().tag_dict = Some(d);
        self
    }

    /// Overrides the UID dictionary for this context layer and descendants.
    pub fn uid_dict(mut self, d: Arc<uid::Dictionary>) -> Self {
        self.ctx_mut().uid_dict = Some(d);
        self
    }

    /// Installs a configuration layer for this context layer and descendants.
    ///
    /// Values not found here fall through to lower layers (see
    /// [`Context::config_value`]).
    pub fn config(mut self, c: Arc<config::Config>) -> Self {
        self.ctx_mut().config = Some(c);
        self
    }

    /// Replaces the global root context with this layer and returns the
    /// previous global root. Use the returned `Arc` to restore it afterwards.
    pub fn install_global(self) -> Arc<Context> {
        GLOBAL_CTX.swap(self.ctx)
    }

    /// Installs this context layer for the duration of `f`, then restores
    /// whatever context was active before.
    pub fn provide<F: FnOnce() -> T, T>(self, f: F) -> T {
        // SAFETY: `Arc::as_ptr` is non-null and preserves provenance over the
        // whole `ArcInner` allocation, so a nested `extend()` can reconstruct an
        // `Arc` from this pointer (see `extend`). Going through `&Context` would
        // narrow provenance to the data range and make that reconstruction UB.
        let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&self.ctx) as *mut Context) };
        LOCAL_CTX.with(|cell| {
            let old = cell.replace(Some(ptr));
            let rv = f();
            cell.set(old);
            rv
        })
        // self.ctx (Arc) drops here, after the thread-local is restored
    }

    /// Wraps `future` so that this context layer is installed before every
    /// `poll()` and restored afterwards.
    pub fn scope<F: Future>(self, future: F) -> ContextScope<F> {
        ContextScope {
            ctx: self.ctx,
            inner: future,
        }
    }
}

// ── ContextScope ──────────────────────────────────────────────────────────────

/// Wraps a [`Future`] so the associated [`Context`] layer is installed before
/// every `poll()`. Created by [`ContextBuilder::scope`].
pub struct ContextScope<F: Future> {
    ctx: Arc<Context>,
    inner: F,
}

impl<F: Future> Future for ContextScope<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<F::Output> {
        // SAFETY:
        // - `ctx: Arc<Context>` is always Unpin; only a raw pointer is derived
        //   from it, nothing is moved out of the struct.
        // - `inner: F` is structurally pinned: re-pinned via `Pin::new_unchecked`
        //   without moving, which is valid because `self` is already pinned.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: `Arc::as_ptr` is non-null and preserves provenance over the
        // whole `ArcInner` allocation, so a nested `extend()` can reconstruct an
        // `Arc` from this pointer (see `extend`). Going through `&Context` would
        // narrow provenance to the data range and make that reconstruction UB.
        let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&this.ctx) as *mut Context) };
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };

        LOCAL_CTX.with(|cell| {
            let old = cell.replace(Some(ptr));
            let result = inner.poll(cx);
            cell.set(old);
            result
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_assoc(id: u64, peer: &str, local: &str) -> Arc<AssocDescription> {
        Arc::new(AssocDescription {
            id,
            peer_aet: peer.into(),
            local_aet: local.into(),
            peer_addr: None,
            local_addr: None,
            is_incoming: true,
            is_tls_secured: false,
        })
    }

    #[test]
    fn empty_context_has_no_assoc() {
        Context::with_current(|ctx| assert!(ctx.assoc().is_none()));
    }

    #[test]
    fn assoc_visible_within_provide() {
        let assoc = make_assoc(1, "PEER", "LOCAL");
        Context::extend().assoc(Arc::clone(&assoc)).provide(|| {
            Context::with_current(|ctx| {
                assert_eq!(ctx.assoc().unwrap().id, 1);
            });
        });
    }

    #[test]
    fn assoc_restored_after_provide() {
        let assoc = make_assoc(1, "PEER", "LOCAL");
        Context::extend().assoc(assoc).provide(|| {});
        Context::with_current(|ctx| assert!(ctx.assoc().is_none()));
    }

    #[test]
    fn inner_assoc_shadows_outer() {
        let outer = make_assoc(1, "OUTER", "LOCAL");
        let inner = make_assoc(2, "INNER", "LOCAL");
        Context::extend().assoc(outer).provide(|| {
            Context::extend().assoc(inner).provide(|| {
                Context::with_current(|ctx| {
                    assert_eq!(ctx.assoc().unwrap().id, 2);
                });
            });
            Context::with_current(|ctx| {
                assert_eq!(ctx.assoc().unwrap().id, 1);
            });
        });
    }

    #[test]
    fn inner_action_visible_with_outer_assoc() {
        let assoc = make_assoc(1, "PEER", "LOCAL");
        Context::extend().assoc(assoc).provide(|| {
            Context::extend().action("c_store_rq", 42).provide(|| {
                Context::with_current(|ctx| {
                    // Action is visible
                    assert!(ctx.has_action("c_store_rq"));
                    assert!(ctx.has_action_with_id("c_store_rq", 42));
                    // Assoc from outer layer is still visible
                    assert_eq!(ctx.assoc().unwrap().id, 1);
                });
            });
        });
    }

    #[test]
    fn nested_actions_all_visible() {
        Context::extend().action("c_store_rq", 1).provide(|| {
            Context::extend().action("db_request", 7).provide(|| {
                Context::with_current(|ctx| {
                    let actions: Vec<_> = ctx.actions().collect();
                    assert_eq!(actions.len(), 2);
                    assert_eq!(actions[0], ("db_request", 7));
                    assert_eq!(actions[1], ("c_store_rq", 1));
                    assert!(ctx.has_action("c_store_rq"));
                    assert!(ctx.has_action("db_request"));
                    assert!(ctx.has_action_with_id("db_request", 7));
                    assert!(!ctx.has_action_with_id("db_request", 99));
                });
            });
        });
    }

    #[test]
    fn actions_cleared_after_provide() {
        Context::extend().action("c_store_rq", 1).provide(|| {});
        Context::with_current(|ctx| assert!(!ctx.has_action("c_store_rq")));
    }

    // ── Configuration layering ────────────────────────────────────────────────

    use crate::config::{ConditionalKey, ConditionalSettings, Config, Key, Registry, Settings, Value};

    const KEY: Key = Key::new("test", 1);

    fn config_with(settings: Settings, conditional: ConditionalSettings) -> Arc<config::Config> {
        Arc::new(
            Config::builder(Arc::new(Registry::new_empty()))
                .settings(settings)
                .conditional(conditional)
                .build(),
        )
    }

    #[test]
    fn config_value_reads_unconditional_layer() {
        let mut s = Settings::new();
        s.set(KEY, Value::Int(7));
        let cfg = config_with(s, ConditionalSettings::new());

        Context::extend().config(cfg).provide(|| {
            Context::with_current(|ctx| {
                assert!(matches!(ctx.config_value(&KEY), Some(Value::Int(7))));
            });
        });
    }

    #[test]
    fn config_value_missing_key_is_none() {
        let cfg = config_with(Settings::new(), ConditionalSettings::new());
        Context::extend().config(cfg).provide(|| {
            Context::with_current(|ctx| assert!(ctx.config_value(&KEY).is_none()));
        });
    }

    #[test]
    fn config_value_selects_conditional_by_association() {
        let mut cs = ConditionalSettings::new();
        cs.add(ConditionalKey::unconditional(KEY), Value::Int(0));
        cs.add(
            ConditionalKey {
                key: KEY,
                peer_aet: Some("PEER".into()),
                ..ConditionalKey::unconditional(KEY)
            },
            Value::Int(1),
        );
        let cfg = config_with(Settings::new(), cs);
        let assoc = make_assoc(1, "PEER", "LOCAL");

        Context::extend().assoc(assoc).config(cfg).provide(|| {
            Context::with_current(|ctx| {
                assert!(matches!(ctx.config_value(&KEY), Some(Value::Int(1))));
            });
        });
    }

    #[test]
    fn inner_layer_overrides_outer_else_falls_through() {
        let mut outer_s = Settings::new();
        outer_s.set(KEY, Value::Int(1));
        let outer = config_with(outer_s, ConditionalSettings::new());

        // Inner layer has no value for KEY: lookup must fall through to outer.
        let inner = config_with(Settings::new(), ConditionalSettings::new());

        Context::extend().config(outer).provide(|| {
            Context::extend().config(inner).provide(|| {
                Context::with_current(|ctx| {
                    assert!(matches!(ctx.config_value(&KEY), Some(Value::Int(1))));
                });
            });
        });
    }

    #[test]
    fn config_value_falls_back_to_registry_default() {
        static METAS: [crate::config::KeyMeta; 1] = [crate::config::KeyMeta {
            key: KEY,
            is_advanced: false,
            display_section: "Test",
            concept: crate::config::Concept::new("k", "K", None),
            value_meta: crate::config::ValueMeta::Int { min: None, max: None },
            make_default: || Some(Value::Int(42)),
        }];

        let mut registry = Registry::new_empty();
        registry.insert(&METAS[0]);
        let cfg = Arc::new(Config::builder(Arc::new(registry)).build());

        Context::extend().config(cfg).provide(|| {
            Context::with_current(|ctx| {
                assert!(matches!(ctx.config_value(&KEY), Some(Value::Int(42))));
            });
        });
    }
}
