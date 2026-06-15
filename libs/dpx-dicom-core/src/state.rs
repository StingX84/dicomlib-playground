use crate::*;
use std::{
    cell::Cell,
    fmt::Debug,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
    sync::{LazyLock, RwLockReadGuard},
    task,
};

thread_local! {
    static LOCAL_DICOM: Cell<Option<NonNull<State>>> = const { Cell::new(None) };
}

static GLOBAL_DICOM: LazyLock<RwLock<State>> = LazyLock::new(|| RwLock::new(State::new()));

#[derive(Debug, Clone)]
pub struct State {
    tag_dict: Arc<tag::Dictionary>,
    uid_dict: Arc<uid::Dictionary>,
}

#[derive(Debug, Default)]
pub struct StateBuilder {
    tag_dict: Option<tag::Dictionary>,
    uid_dict: Option<uid::Dictionary>,
}

pub enum CurrentState {
    Global(RwLockReadGuard<'static, State>),
    Local(&'static State),
}

impl State {
    pub fn new() -> Self {
        Self {
            tag_dict: Default::default(),
            uid_dict: Default::default(),
        }
    }

    pub fn global() -> &'static RwLock<State> {
        &GLOBAL_DICOM
    }

    pub fn into_global(self) -> Self {
        core::mem::replace(
            Self::global()
                .write()
                .expect("Global DICOM state was poisoned!")
                .deref_mut(),
            self,
        )
    }

    pub fn provide_current_for<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        LOCAL_DICOM.with(|cell| {
            let old = cell.replace(Some(NonNull::from(self)));
            let rv = f();
            cell.set(old);
            rv
        })
    }

    pub fn with_current<F, T>(f: F) -> T
    where
        F: FnOnce(&State) -> T,
    {
        LOCAL_DICOM.with(|cell| match cell.get() {
            Some(ptr) => f(unsafe { ptr.as_ref() }),
            None => {
                let read_lock = Self::global()
                    .read()
                    .expect("Global DICOM state was poisoned");
                read_lock.provide_current_for(move || Self::with_current(f))
            }
        })
    }

    pub fn current() -> CurrentState {
        LOCAL_DICOM.with(|cell| match cell.get() {
            Some(ptr) => CurrentState::Local(unsafe { ptr.as_ref() }),
            None => CurrentState::Global(
                Self::global()
                    .read()
                    .expect("Global DICOM state was poisoned"),
            ),
        })
    }

    pub fn tag_dictionary(&self) -> &tag::Dictionary {
        self.tag_dict.deref()
    }
    pub fn set_tag_dictionary(&mut self, d: uid::Dictionary) {
        self.uid_dict = Arc::new(d);
    }
    pub fn uid_dictionary(&self) -> &uid::Dictionary {
        self.uid_dict.deref()
    }
    pub fn set_uid_dictionary(&mut self, d: uid::Dictionary) {
        self.uid_dict = Arc::new(d);
    }

    /// Wraps `future` so that `self` is the current thread-local [`State`] for every
    /// `poll()` of the returned future, regardless of which thread executes the poll.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use dpx_dicom_core::State;
    /// let state = Arc::new(State::new());
    /// // scope() returns a Future; hand it to your async runtime:
    /// //   tokio::spawn(Arc::clone(&state).scope(async { ... }))
    /// let _scoped = Arc::clone(&state).scope(async {
    ///     State::with_current(|s| { let _ = s.tag_dictionary(); });
    /// });
    /// ```
    pub fn scope<F: Future>(self: Arc<Self>, future: F) -> StateScope<F> {
        StateScope { state: self, inner: future }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl StateBuilder {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn with_tag_dictionary(self, v: tag::Dictionary) -> Self {
        Self { tag_dict: Some(v), ..self }
    }

    pub fn with_uid_dictionary(self, v: uid::Dictionary) -> Self {
        Self { uid_dict: Some(v), ..self }
    }

    pub fn build(self) -> State {
        State {
            tag_dict: Arc::new(self.tag_dict.unwrap_or_default()),
            uid_dict: Arc::new(self.uid_dict.unwrap_or_default()),
        }
    }
}

impl Debug for CurrentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global(lock) => Debug::fmt(lock.deref(), f),
            Self::Local(r) => Debug::fmt(*r, f),
        }
    }
}

impl Deref for CurrentState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Global(lock) => lock.deref(),
            Self::Local(r) => r,
        }
    }
}

/// Wraps a `Future` so that this `State` is installed as the thread-local current state
/// before every `poll()` and restored afterwards. Created by [`State::scope`].
pub struct StateScope<F: Future> {
    state: Arc<State>,
    inner: F,
}

impl<F: Future> Future for StateScope<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        // SAFETY:
        // - `state: Arc<State>` is always Unpin; we only take a raw pointer from it,
        //   never move it out of the struct.
        // - `inner: F` is structurally pinned: we re-pin it via `Pin::new_unchecked`
        //   without moving it, which is valid because `self` was already pinned.
        let this = unsafe { self.get_unchecked_mut() };
        let state_ptr = NonNull::from(this.state.as_ref());
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };

        LOCAL_DICOM.with(|cell| {
            let old = cell.replace(Some(state_ptr));
            let result = inner.poll(cx);
            cell.set(old);
            result
        })
    }
}
