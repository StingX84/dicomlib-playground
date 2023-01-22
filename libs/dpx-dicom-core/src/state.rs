use crate::*;
use std::{
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::RwLockReadGuard,
};

thread_local! {
    static LOCAL_DICOM : Cell<Option<NonNull<State>>> = Cell::new(None);
}

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

    pub fn into_global(self) -> Self {
        core::mem::replace(
            Self::global()
                .write()
                .expect("Global DICOM state was poisoned!")
                .deref_mut(),
            self,
        )
    }

    #[cfg(feature = "unstable")]
    pub fn global() -> &'static RwLock<State> {
        use std::sync::OnceLock;
        static GLOBAL_DICOM: OnceLock<RwLock<State>> = OnceLock::new();
        GLOBAL_DICOM.get_or_init(|| RwLock::new(State::new()))
    }

    #[cfg(not(feature = "unstable"))]
    pub fn global() -> &'static RwLock<State> {
        use std::{mem::MaybeUninit, sync::Once};
        struct OnceDicomInit {
            once: Once,
            value: UnsafeCell<MaybeUninit<RwLock<State>>>,
        }
        impl OnceDicomInit {
            const fn new() -> Self {
                Self {
                    once: Once::new(),
                    value: UnsafeCell::new(MaybeUninit::uninit()),
                }
            }

            fn get(&self) -> &RwLock<State> {
                if !self.once.is_completed() {
                    let slot = &self.value;
                    self.once.call_once(|| unsafe {
                        (*slot.get()).write(RwLock::new(State::default()));
                    });
                }
                // SAFETY: `self.value` is initialized and contains a valid `RwLock`.
                unsafe { (*self.value.get()).assume_init_ref() }
            }
        }

        unsafe impl Sync for OnceDicomInit {}

        impl Drop for OnceDicomInit {
            fn drop(&mut self) {
                if self.once.is_completed() {
                    // SAFETY: The cell is initialized and being dropped, so it can't
                    // be accessed again.
                    unsafe { (*self.value.get()).assume_init_drop() };
                }
            }
        }

        static GLOBAL_DICOM: OnceDicomInit = OnceDicomInit::new();
        GLOBAL_DICOM.get()
    }

    pub fn provide_current_for<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        LOCAL_DICOM.with(move |cell| {
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
        LOCAL_DICOM.with(move |cell| match cell.get() {
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
        LOCAL_DICOM.with(move |cell| match cell.get() {
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
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl StateBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_tag_dictionary(self, v: tag::Dictionary) -> Self {
        Self {
            tag_dict: Some(v),
            ..self
        }
    }

    pub fn with_uid_dictionary(self, v: uid::Dictionary) -> Self {
        Self {
            uid_dict: Some(v),
            ..self
        }
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
