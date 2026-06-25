use crate::network::AssocDescription;
use crate::{config::ConfigIter, config::ConfigValues, *};
use arc_swap::ArcSwap;
use std::{cell::Cell, fmt, future::Future, pin::Pin, ptr::NonNull, sync::LazyLock, task};

thread_local! {
    static LOCAL_CTX: Cell<Option<NonNull<Context>>> = const { Cell::new(None) };
}

// The global root: always carries default dictionaries and a base configuration.
// ArcSwap allows lock-free reads and atomic full-Arc replacement.
static GLOBAL_CTX: LazyLock<ArcSwap<Context>> = LazyLock::new(|| {
    ArcSwap::from_pointee(Context {
        assoc: None,
        action: None,
        tag_dict: Some(Arc::new(tag::Dictionary::default())),
        uid_dict: Some(Arc::new(uid::Dictionary::default())),
        config: Some(Arc::new(config::Object::new_empty(
            config::meta::collected_global_meta(),
        ))),
        prev: None,
    })
});

// Restores the previous thread-local context pointer on drop, so a panic
// unwinding through `provide`/`with_current`/`poll` cannot leave a dangling
// pointer (to a freed `Arc<Context>`) installed in `LOCAL_CTX`.
struct CtxGuard(Option<NonNull<Context>>);

impl Drop for CtxGuard {
    fn drop(&mut self) {
        LOCAL_CTX.with(|cell| cell.set(self.0));
    }
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
/// use dpx_dicom_core::{context::Context, network::AssocDescription};
///
/// let assoc = Arc::new(AssocDescription {
///     id: 1, is_tls_used: false, is_incoming: false, is_virtual: true,
///     peer_aet: Some("PEER".into()), local_aet: Some("LOCAL".into()),
///     peer_addr: None, local_addr: None,
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
    config: Option<Arc<config::Object>>,
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

    /// Returns the base configuration carried by the global root.
    ///
    /// The global root always carries a base configuration, so this never
    /// returns `None`. Override layers installed with [`ContextBuilder::config`]
    /// shadow it for the duration of a call tree.
    ///
    /// This is a crate-internal accessor; the public single source of truth for
    /// the application-wide configuration is
    /// [`GlobalConfig`](crate::config::GlobalConfig).
    pub(crate) fn global_config() -> Arc<config::Object> {
        GLOBAL_CTX
            .load()
            .config
            .clone()
            .expect("global root must always carry a base configuration")
    }

    /// Atomically publishes `config` as the global base configuration.
    ///
    /// Only the root node's `config` field is replaced; the default dictionaries
    /// and any other root state are preserved, and no context layer is added, so
    /// repeated hot-reloads do not grow the chain. Lock-free for readers.
    pub(crate) fn publish_global_config(config: Arc<config::Object>) {
        GLOBAL_CTX.rcu(|old| {
            Arc::new(Context {
                assoc: old.assoc.clone(),
                action: old.action,
                tag_dict: old.tag_dict.clone(),
                uid_dict: old.uid_dict.clone(),
                config: Some(Arc::clone(&config)),
                prev: old.prev.clone(),
            })
        });
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
        // SAFETY: `Arc::as_ptr` is non-null and carries provenance over the
        // whole `ArcInner` allocation (refcounts included), so a nested
        // `extend()` can reconstruct an `Arc` from this pointer. Deriving it
        // through `&Context` would narrow provenance to the data range and
        // make that reconstruction UB.
        let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&global) as *mut Context) };
        // `_guard` is dropped before `global`, restoring the thread-local even
        // if `f` panics, while the allocation is still alive.
        let _guard = LOCAL_CTX.with(|cell| CtxGuard(cell.replace(Some(ptr))));
        f(unsafe { ptr.as_ref() })
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

    /// Returns the nearest [`config::Object`] in the context chain.
    pub fn config(&self) -> &config::Object {
        self.find(|n| n.config.as_deref())
            .expect("Context chain missing config — global root must have one")
    }

    pub fn config_get_current(&self, key: &config::Key) -> Option<&config::Value> {
        self.config_get(key, self.assoc().map(|a| a.as_ref()))
    }

    pub fn config_get_current_as<'a, T: config::ValueRef>(
        &'a self,
        key: &config::Key,
    ) -> Option<<T as config::ValueRef>::Ref<'a>> {
        self.config_get_as::<T>(key, self.assoc().map(|a| a.as_ref()))
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

    fn detect_parent_config_layer_id(&self) -> Option<&config::LayerId> {
        let mut node = self.prev.as_deref();
        while let Some(n) = node {
            if let Some(cfg) = n.config.as_deref()
                && let Some(layer_id) = cfg.layer_id()
            {
                return Some(layer_id);
            }
            node = n.prev.as_deref();
        }
        Some(&config::GLOBAL_LAYER_ID)
    }
}

pub struct ContextConfigIter<'a> {
    node: Option<&'a Context>,
    cfg_iter: Option<ConfigIter<'a>>,
}

impl<'a> Iterator for ContextConfigIter<'a> {
    type Item = (
        &'a config::Key,
        &'a config::Value,
        Option<&'a config::Condition>,
        &'a config::LayerId,
    );
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(cfg_iter) = &mut self.cfg_iter {
                if let Some(item) = cfg_iter.next() {
                    return Some(item);
                }
                self.cfg_iter = None;
            }
            let node = self.node?;
            self.node = node.prev.as_deref();
            if let Some(cfg) = node.config.as_deref() {
                let parent_layer_id = self.node.and_then(|n| n.detect_parent_config_layer_id());
                self.cfg_iter = Some(cfg.config_iter(parent_layer_id));
            }
        }
    }
}

impl ConfigValues for Context {
    type Iter<'a>
        = ContextConfigIter<'a>
    where
        Self: 'a;

    fn config_iter<'a>(&'a self, parent_layer_id: Option<&'a config::LayerId>) -> Self::Iter<'a>
    where
        Self: 'a,
    {
        let parent_layer_id = parent_layer_id.or_else(|| self.detect_parent_config_layer_id());
        ContextConfigIter {
            node: Some(self),
            cfg_iter: self.config.as_deref().map(|cfg| cfg.config_iter(parent_layer_id)),
        }
    }

    fn config_default_of(&self, key: &config::Key) -> Option<&config::Value> {
        let mut node = Some(self);
        while let Some(n) = node {
            if let Some(cfg) = n.config.as_deref()
                && let Some(value) = cfg.default_of(key)
            {
                return Some(value);
            }
            node = n.prev.as_deref();
        }
        None
    }

    fn config_get_explicit(&self, key: &config::Key, assoc: Option<&AssocDescription>) -> Option<&config::Value> {
        let mut node = Some(self);
        while let Some(n) = node {
            if let Some(cfg) = n.config.as_deref()
                && let Some(value) = cfg.values().get_ranked(key, assoc)
            {
                return Some(value.0);
            }

            node = n.prev.as_deref();
        }
        None
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
    /// [`Context::config_get`]).
    pub fn config(mut self, c: Arc<config::Object>) -> Self {
        self.ctx_mut().config = Some(c);
        self
    }

    /// Replaces the global root context with this layer and returns the
    /// previous global root. Use the returned `Arc` to restore it afterwards.
    ///
    /// The installed root is always self-contained: any of the required fields
    /// (tag dictionary, UID dictionary, base configuration) that this builder
    /// does not set are inherited from the previous root, and the new root has no
    /// parent. This preserves the invariant that the global root carries every
    /// required field, so accessors like [`Context::config`] and
    /// [`Context::tag_dict`] can never fail, and repeated installs do not grow a
    /// parent chain.
    pub fn install_global(self) -> Arc<Context> {
        let mut prev = None;
        GLOBAL_CTX.rcu(|old| {
            prev = Some(Arc::clone(old));
            Arc::new(Context {
                assoc: self.ctx.assoc.clone(),
                action: self.ctx.action,
                tag_dict: self.ctx.tag_dict.clone().or_else(|| old.tag_dict.clone()),
                uid_dict: self.ctx.uid_dict.clone().or_else(|| old.uid_dict.clone()),
                config: self.ctx.config.clone().or_else(|| old.config.clone()),
                prev: None,
            })
        });
        prev.expect("ArcSwap::rcu always invokes its closure at least once")
    }

    /// Installs this context layer for the duration of `f`, then restores
    /// whatever context was active before.
    pub fn provide<F: FnOnce() -> T, T>(self, f: F) -> T {
        // SAFETY: `Arc::as_ptr` is non-null and preserves provenance over the
        // whole `ArcInner` allocation, so a nested `extend()` can reconstruct an
        // `Arc` from this pointer (see `extend`). Going through `&Context` would
        // narrow provenance to the data range and make that reconstruction UB.
        let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&self.ctx) as *mut Context) };
        // `_guard` is dropped before `self.ctx`, restoring the thread-local even
        // if `f` panics, while the allocation is still alive.
        let _guard = LOCAL_CTX.with(|cell| CtxGuard(cell.replace(Some(ptr))));
        f()
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

        // Restores the thread-local even if `inner.poll` panics; `this.ctx`
        // outlives this frame, so the pointer stays valid for the poll.
        let _guard = LOCAL_CTX.with(|cell| CtxGuard(cell.replace(Some(ptr))));
        inner.poll(cx)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;
    use std::borrow::Cow;

    fn make_assoc(id: u64, peer: &'static str, local: &'static str) -> Arc<AssocDescription> {
        Arc::new(AssocDescription {
            id,
            is_tls_used: false,
            is_incoming: false,
            is_virtual: true,
            peer_aet: Some(Cow::Borrowed(peer)),
            local_aet: Some(Cow::Borrowed(local)),
            peer_addr: None,
            local_addr: None,
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

    use crate::config::{
        Condition, GlobalConfig, Key, Object, Value,
        map::{Conditionals, Map},
        meta,
    };

    const KEY: Key = Key::new("test");
    const TEST_KEYS: &[meta::KeyMeta] = &[];

    config_object_meta!( fn test_object_meta() = &TEST_KEYS );

    fn config_with<const N: usize>(values: [(Key, Conditionals); N]) -> Arc<config::Object> {
        Arc::new(Object::new(test_object_meta(), Map::from_iter(values)))
    }

    #[test]
    fn config_get_any_reads_unconditional_layer() {
        let cfg = config_with([(KEY, Value::Int(7).into())]);

        Context::extend().config(cfg).provide(|| {
            Context::with_current(|ctx| {
                assert_matches!(ctx.config_get_current(&KEY), Some(Value::Int(7)));
            });
        });
    }

    #[test]
    fn config_get_any_missing_key_is_none() {
        let cfg = config_with([]);
        Context::extend().config(cfg).provide(|| {
            Context::with_current(|ctx| assert!(ctx.config_get_current(&KEY).is_none()));
        });
    }

    #[test]
    fn config_get_any_selects_conditional_by_association() {
        let cfg = config_with([
            (KEY, Value::Int(0).into()),
            (
                KEY,
                (
                    Value::Int(1),
                    Condition {
                        peer_aet: Some("PEER".into()),
                        ..Default::default()
                    },
                )
                    .into(),
            ),
        ]);
        let assoc = make_assoc(1, "PEER", "LOCAL");

        Context::extend().assoc(assoc).config(cfg).provide(|| {
            Context::with_current(|ctx| {
                assert!(matches!(ctx.config_get_current(&KEY), Some(Value::Int(1))));
            });
        });
    }

    #[test]
    fn inner_layer_overrides_outer_else_falls_through() {
        let outer = config_with([(KEY, Value::Int(1).into())]);
        // Inner layer has no value for KEY: lookup must fall through to outer.
        let inner = config_with([]);

        Context::extend().config(outer).provide(|| {
            Context::extend().config(inner).provide(|| {
                Context::with_current(|ctx| {
                    assert!(matches!(ctx.config_get_current(&KEY), Some(Value::Int(1))));
                });
            });
        });
    }

    #[test]
    fn install_global_inherits_missing_required_fields() {
        // Installing a root that only sets a configuration must not drop the
        // dictionaries: the new self-contained root inherits them from the
        // previous root, so the required-field accessors can never panic.
        // Mutates the process-global context; serialize against every other
        // test that swaps global state.
        let _guard = crate::config::subst::lock_global_for_test();
        let cfg = config_with([]);
        let prev = Context::extend().config(Arc::clone(&cfg)).install_global();

        Context::with_current(|ctx| {
            // Inherited from the previous root rather than the builder.
            let _ = ctx.tag_dict();
            let _ = ctx.uid_dict();
            // The field we explicitly installed.
            assert!(Arc::ptr_eq(&GlobalConfig::current(), &cfg));
        });

        Context::global().store(prev);
    }
}
