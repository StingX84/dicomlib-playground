//! Synchronous, thread-safe event source with prioritized subscribers.
//!
//! An [`Event<A>`] is an observable source that fan-outs a single value `&A` to
//! any number of subscribers. Each subscriber is a `Fn(&A) -> Result<()>`
//! callback; returning `Err` stops the propagation chain and that error is
//! returned from [`Event::fire`] (the equivalent of returning `false` from a
//! C++ handler).
//!
//! Subscriptions are RAII handles: dropping the returned [`Subscription`]
//! cancels the subscription. A subscription also becomes inactive once the
//! owning [`Event`] is dropped.
//!
//! # Thread safety
//!
//! [`Event`], [`EventObserver`] and [`Subscription`] are all `Send + Sync`.
//! [`Event::fire`] takes a snapshot of the current subscribers under a short
//! lock and releases it before invoking any callback, so handlers may freely
//! subscribe, unsubscribe, or fire other events from any thread without
//! deadlocking. Handlers added while a `fire` is in progress are not invoked by
//! that in-flight `fire`; handlers removed while a `fire` is in progress are
//! skipped.
//!
//! Because callbacks are `Fn` (not `FnMut`), recursive `fire` of the same event
//! from within a handler is safe. Mutable per-handler state must be held behind
//! interior mutability (e.g. a `Mutex`).
//!
//! # Example
//!
//! ```
//! use dpx_dicom_core::event::Event;
//!
//! let event: Event<String> = Event::new();
//!
//! // Keep the returned handle alive for as long as the subscription is wanted.
//! let _sub = event.subscribe(|msg: &String| {
//!     println!("received: {msg}");
//!     Ok(())
//! });
//!
//! event.fire(&"hello".to_string()).unwrap();
//! ```

use crate::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

/// Default subscriber priority used by [`Event::subscribe`] and
/// [`EventObserver::subscribe`]. Higher priority is invoked earlier.
pub const DEFAULT_PRIORITY: i32 = 100;

type BoxHandler<A> = Box<dyn Fn(&A) -> Result<()> + Send + Sync>;

struct Subscriber<A> {
    id: u64,
    priority: i32,
    active: AtomicBool,
    handler: BoxHandler<A>,
}

struct Registry<A> {
    /// Sorted by descending priority; subscribers of equal priority keep insertion
    /// order (a newer one is invoked after older ones of the same priority).
    subscribers: Vec<Arc<Subscriber<A>>>,
    next_id: u64,
}

struct Shared<A> {
    inner: Mutex<Registry<A>>,
}

// A poisoned mutex here only means some handler panicked on another thread; the
// registry itself stays structurally valid, so recovering the guard is safe and
// preferable to propagating a panic.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl<A> Shared<A> {
    fn add(self: &Arc<Self>, priority: i32, handler: BoxHandler<A>) -> Subscription<A> {
        let mut reg = lock(&self.inner);
        let id = reg.next_id;
        reg.next_id += 1;
        let pos = reg
            .subscribers
            .iter()
            .position(|e| e.priority < priority)
            .unwrap_or(reg.subscribers.len());
        reg.subscribers.insert(
            pos,
            Arc::new(Subscriber {
                id,
                priority,
                active: AtomicBool::new(true),
                handler,
            }),
        );
        Subscription {
            shared: Arc::downgrade(self),
            id,
        }
    }

    fn remove(&self, id: u64) {
        let mut reg = lock(&self.inner);
        if let Some(pos) = reg.subscribers.iter().position(|s| s.id == id) {
            // Mark inactive first so any concurrent `fire` holding this entry in
            // its snapshot skips it instead of invoking a cancelled handler.
            reg.subscribers[pos].active.store(false, Ordering::Release);
            reg.subscribers.remove(pos);
        }
    }

    fn contains(&self, id: u64) -> bool {
        lock(&self.inner).subscribers.iter().any(|s| s.id == id)
    }

    fn fire(&self, arg: &A) -> Result<()> {
        let snapshot = {
            let reg = lock(&self.inner);
            reg.subscribers.clone()
        };
        for sub in &snapshot {
            if sub.active.load(Ordering::Acquire) {
                (sub.handler)(arg)?;
            }
        }
        Ok(())
    }
}

/// A thread-safe, observable source of `&A` events.
///
/// Hold the `Event` in the producing component and call [`fire`](Event::fire)
/// to notify subscribers. Hand out [`observer`](Event::observer) to components
/// that should only be able to subscribe, never fire.
pub struct Event<A> {
    shared: Arc<Shared<A>>,
}

impl<A> Event<A> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                inner: Mutex::new(Registry {
                    subscribers: Vec::new(),
                    next_id: 0,
                }),
            }),
        }
    }

    /// Subscribe with default priority. Higher priority is invoked earlier;
    /// equal priorities preserve subscription order.
    #[must_use = "dropping the Subscription immediately cancels the subscription"]
    pub fn subscribe<F>(&self, handler: F) -> Subscription<A>
    where
        F: Fn(&A) -> Result<()> + Send + Sync + 'static,
    {
        self.shared.add(DEFAULT_PRIORITY, Box::new(handler))
    }

    /// Subscribe with an explicit priority. See [`DEFAULT_PRIORITY`].
    #[must_use = "dropping the Subscription immediately cancels the subscription"]
    pub fn subscribe_with_priority<F>(&self, priority: i32, handler: F) -> Subscription<A>
    where
        F: Fn(&A) -> Result<()> + Send + Sync + 'static,
    {
        self.shared.add(priority, Box::new(handler))
    }

    /// A cloneable, subscribe-only handle to this event. Subscriptions created
    /// through it become inactive once this `Event` is dropped.
    #[must_use]
    pub fn observer(&self) -> EventObserver<A> {
        EventObserver {
            shared: Arc::downgrade(&self.shared),
        }
    }

    /// Invoke every active subscriber in priority order with `arg`.
    ///
    /// Propagation stops at the first subscriber that returns `Err`, and that
    /// error is returned. Returns `Ok(())` if every subscriber returned `Ok`.
    pub fn fire(&self, arg: &A) -> Result<()> {
        self.shared.fire(arg)
    }

    /// `true` if at least one subscriber is currently registered.
    #[must_use]
    pub fn has_subscribers(&self) -> bool {
        !lock(&self.shared.inner).subscribers.is_empty()
    }
}

impl<A> Default for Event<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// A subscribe-only handle obtained from [`Event::observer`].
///
/// Cloning is cheap and shares the same underlying event. Once the originating
/// [`Event`] is dropped, [`subscribe`](EventObserver::subscribe) returns a
/// detached [`Subscription`] and [`has_subscribers`](EventObserver::has_subscribers)
/// returns `false`.
pub struct EventObserver<A> {
    shared: Weak<Shared<A>>,
}

impl<A> Clone for EventObserver<A> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<A> EventObserver<A> {
    #[must_use = "dropping the Subscription immediately cancels the subscription"]
    pub fn subscribe<F>(&self, handler: F) -> Subscription<A>
    where
        F: Fn(&A) -> Result<()> + Send + Sync + 'static,
    {
        self.subscribe_with_priority(DEFAULT_PRIORITY, handler)
    }

    #[must_use = "dropping the Subscription immediately cancels the subscription"]
    pub fn subscribe_with_priority<F>(&self, priority: i32, handler: F) -> Subscription<A>
    where
        F: Fn(&A) -> Result<()> + Send + Sync + 'static,
    {
        match self.shared.upgrade() {
            Some(shared) => shared.add(priority, Box::new(handler)),
            None => Subscription::detached(),
        }
    }

    #[must_use]
    pub fn has_subscribers(&self) -> bool {
        self.shared
            .upgrade()
            .is_some_and(|s| !lock(&s.inner).subscribers.is_empty())
    }
}

/// RAII handle owning a single subscription.
///
/// The subscription stays active until this handle is dropped, [`reset`] is
/// called, or the originating [`Event`] is dropped. The handle is move-only;
/// there is intentionally no `Clone`.
///
/// [`reset`]: Subscription::reset
#[must_use = "dropping the Subscription immediately cancels the subscription"]
pub struct Subscription<A> {
    shared: Weak<Shared<A>>,
    id: u64,
}

impl<A> Subscription<A> {
    /// An inactive handle that is not tied to any subscription.
    pub fn detached() -> Self {
        Self {
            shared: Weak::new(),
            id: 0,
        }
    }

    /// `true` while the subscription is registered and the event is alive.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.shared.upgrade().is_some_and(|s| s.contains(self.id))
    }

    /// Cancel the subscription now. Subsequent calls are no-ops.
    pub fn reset(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            shared.remove(self.id);
        }
        self.shared = Weak::new();
    }

    /// Detach this handle without cancelling: the subscription then lives until
    /// the originating [`Event`] is dropped.
    pub fn forget(mut self) {
        self.shared = Weak::new();
    }
}

impl<A> Drop for Subscription<A> {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            shared.remove(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom_err;
    use std::sync::Mutex as StdMutex;

    /// Returns a shared log plus a factory that builds handlers pushing `marker`.
    fn logging() -> (Arc<StdMutex<Vec<i32>>>, impl Fn(i32) -> BoxHandler<()>) {
        let log = Arc::new(StdMutex::new(Vec::new()));
        let log_for_factory = log.clone();
        let make = move |marker: i32| -> BoxHandler<()> {
            let log = log_for_factory.clone();
            Box::new(move |_: &()| {
                log.lock().unwrap().push(marker);
                Ok(())
            })
        };
        (log, make)
    }

    #[test]
    fn fires_in_priority_then_insertion_order() {
        let (log, make) = logging();
        let event: Event<()> = Event::new();

        let h1 = make(1);
        let h2 = make(2);
        let h3 = make(3);
        let _a = event.subscribe_with_priority(100, move |a| h1(a));
        let _b = event.subscribe_with_priority(100, move |a| h2(a));
        let _c = event.subscribe_with_priority(200, move |a| h3(a));

        event.fire(&()).unwrap();
        assert_eq!(*log.lock().unwrap(), vec![3, 1, 2]);
    }

    #[test]
    fn drop_cancels_subscription() {
        let event: Event<()> = Event::new();
        let calls = Arc::new(StdMutex::new(0));
        let c = calls.clone();
        let sub = event.subscribe(move |_| {
            *c.lock().unwrap() += 1;
            Ok(())
        });
        assert!(event.has_subscribers());
        event.fire(&()).unwrap();
        drop(sub);
        assert!(!event.has_subscribers());
        event.fire(&()).unwrap();
        assert_eq!(*calls.lock().unwrap(), 1);
    }

    #[test]
    fn reset_makes_inactive() {
        let event: Event<()> = Event::new();
        let mut sub = event.subscribe(|_| Ok(()));
        assert!(sub.is_active());
        sub.reset();
        assert!(!sub.is_active());
        assert!(!event.has_subscribers());
    }

    #[test]
    fn err_stops_chain_and_propagates() {
        let event: Event<()> = Event::new();
        let reached = Arc::new(StdMutex::new(false));
        let r = reached.clone();

        let _first = event.subscribe_with_priority(200, |_| Err(dicom_err!(Internal, "stop here")));
        let _second = event.subscribe_with_priority(100, move |_| {
            *r.lock().unwrap() = true;
            Ok(())
        });

        let err = event.fire(&()).unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::Internal);
        assert!(!*reached.lock().unwrap());
    }

    #[test]
    fn observer_detaches_after_event_dropped() {
        let event: Event<()> = Event::new();
        let observer = event.observer();
        drop(event);

        let sub = observer.subscribe(|_| Ok(()));
        assert!(!sub.is_active());
        assert!(!observer.has_subscribers());
    }

    #[test]
    fn unsubscribe_from_within_handler() {
        let event: Arc<Event<()>> = Arc::new(Event::new());
        let inner_sub: Arc<StdMutex<Option<Subscription<()>>>> = Arc::new(StdMutex::new(None));

        let held = inner_sub.clone();
        let later_calls = Arc::new(StdMutex::new(0));
        let lc = later_calls.clone();
        *inner_sub.lock().unwrap() = Some(event.subscribe_with_priority(50, move |_| {
            *lc.lock().unwrap() += 1;
            Ok(())
        }));

        let held2 = held.clone();
        let _first = event.subscribe_with_priority(100, move |_| {
            // Cancel the lower-priority subscriber before it runs this round.
            if let Some(s) = held2.lock().unwrap().take() {
                drop(s);
            }
            Ok(())
        });

        event.fire(&()).unwrap();
        assert_eq!(*later_calls.lock().unwrap(), 0);
    }

    #[test]
    fn is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Event<String>>();
        assert_send_sync::<EventObserver<String>>();
        assert_send_sync::<Subscription<String>>();
    }
}
