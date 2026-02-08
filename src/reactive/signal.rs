//! Signal<T> and create_signal() for reactive state.
//!
//! Fine-grained reactive primitives: signals store values, effects auto-track
//! reads, and memos cache derived computations. Modeled after Leptos's
//! client-side reactivity (single-threaded, synchronous, thread-local runtime).

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// IDs
// ---------------------------------------------------------------------------

/// Identifies a signal slot inside the [`Runtime`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(usize);

/// Identifies an effect slot inside the [`Runtime`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(usize);

// ---------------------------------------------------------------------------
// Runtime internals
// ---------------------------------------------------------------------------

struct SignalState {
    value: Box<dyn Any>,
    subscribers: HashSet<EffectId>,
}

struct EffectState {
    /// The effect closure. Wrapped in `Option` so we can temporarily take it
    /// out while running (avoids holding a `RefMut` on the runtime across the
    /// user callback).
    callback: Option<Box<dyn FnMut()>>,
    dependencies: HashSet<SignalId>,
    active: bool,
}

struct Runtime {
    signals: Vec<SignalState>,
    effects: Vec<EffectState>,
    /// The effect currently executing (for auto-tracking).
    tracking: Option<EffectId>,
    /// When > 0 we are inside a `batch()` call — effects are deferred.
    batch_depth: usize,
    /// Effects that need to be re-run once the outermost batch ends.
    pending_effects: Vec<EffectId>,
    /// Guard against recursive effect execution triggered by `set` inside an
    /// effect that is itself being executed by the notification loop.
    running_effects: bool,
}

impl Runtime {
    fn new() -> Self {
        Self {
            signals: Vec::new(),
            effects: Vec::new(),
            tracking: None,
            batch_depth: 0,
            pending_effects: Vec::new(),
            running_effects: false,
        }
    }
}

thread_local! {
    pub(crate) static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::new());
}

// ---------------------------------------------------------------------------
// Signal creation
// ---------------------------------------------------------------------------

/// Create a reactive signal with the given initial value.
///
/// Returns a `(ReadSignal<T>, WriteSignal<T>)` pair. Reading inside an effect
/// automatically subscribes that effect to changes.
pub fn create_signal<T: 'static>(initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let id = SignalId(rt.signals.len());
        rt.signals.push(SignalState {
            value: Box::new(initial),
            subscribers: HashSet::new(),
        });
        id
    });

    (
        ReadSignal {
            id,
            _marker: PhantomData,
        },
        WriteSignal {
            id,
            _marker: PhantomData,
        },
    )
}

// ---------------------------------------------------------------------------
// ReadSignal
// ---------------------------------------------------------------------------

/// Read-half of a signal. `Copy` — only stores an id.
pub struct ReadSignal<T: 'static> {
    id: SignalId,
    _marker: PhantomData<T>,
}

// Manual impls so we don't require T: Copy/Clone for the signal itself.
impl<T: 'static> Copy for ReadSignal<T> {}
impl<T: 'static> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> fmt::Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadSignal")
            .field("id", &self.id.0)
            .finish()
    }
}

impl<T: 'static> ReadSignal<T> {
    /// Read the current value, subscribing the running effect (if any).
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone())
    }

    /// Read by reference without cloning. Still subscribes the running effect.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        RUNTIME.with(|rt| {
            // -- track dependency --
            {
                let mut rt_ref = rt.borrow_mut();
                if let Some(eid) = rt_ref.tracking {
                    rt_ref.signals[self.id.0].subscribers.insert(eid);
                    rt_ref.effects[eid.0].dependencies.insert(self.id);
                }
            }
            // -- read value (immutable borrow is fine now) --
            let rt_ref = rt.borrow();
            let any_ref = &rt_ref.signals[self.id.0].value;
            f(any_ref.downcast_ref::<T>().expect("signal type mismatch"))
        })
    }

    /// Read without tracking — will not subscribe any running effect.
    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        RUNTIME.with(|rt| {
            let rt_ref = rt.borrow();
            let any_ref = &rt_ref.signals[self.id.0].value;
            any_ref
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone()
        })
    }
}

// ---------------------------------------------------------------------------
// WriteSignal
// ---------------------------------------------------------------------------

/// Write-half of a signal. `Copy` — only stores an id.
pub struct WriteSignal<T: 'static> {
    id: SignalId,
    _marker: PhantomData<T>,
}

impl<T: 'static> Copy for WriteSignal<T> {}
impl<T: 'static> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> fmt::Debug for WriteSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteSignal")
            .field("id", &self.id.0)
            .finish()
    }
}

impl<T: 'static> WriteSignal<T> {
    /// Overwrite the signal value and notify subscribers.
    pub fn set(&self, value: T) {
        let subs = RUNTIME.with(|rt| {
            let mut rt_ref = rt.borrow_mut();
            rt_ref.signals[self.id.0].value = Box::new(value);
            rt_ref.signals[self.id.0]
                .subscribers
                .iter()
                .copied()
                .collect::<Vec<_>>()
        });
        notify_subscribers(subs);
    }

    /// Mutate the value in-place and notify subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let subs = RUNTIME.with(|rt| {
            let mut rt_ref = rt.borrow_mut();
            let any_mut = &mut rt_ref.signals[self.id.0].value;
            let val = any_mut
                .downcast_mut::<T>()
                .expect("signal type mismatch");
            f(val);
            rt_ref.signals[self.id.0]
                .subscribers
                .iter()
                .copied()
                .collect::<Vec<_>>()
        });
        notify_subscribers(subs);
    }
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

/// Create a side-effect that auto-tracks signal reads.
///
/// The closure runs immediately once (establishing initial subscriptions),
/// then re-runs whenever any tracked signal changes.
pub fn create_effect(f: impl FnMut() + 'static) {
    let eid = RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        let eid = EffectId(rt_ref.effects.len());
        rt_ref.effects.push(EffectState {
            callback: Some(Box::new(f)),
            dependencies: HashSet::new(),
            active: true,
        });
        eid
    });
    run_effect(eid);
}

/// Create a memoised derived computation.
///
/// The function `f` is run immediately and whenever its dependencies change.
/// The returned `ReadSignal<T>` only notifies *its* subscribers when the
/// computed value actually changes (by `PartialEq`).
pub fn create_memo<T: Clone + PartialEq + 'static>(
    mut f: impl FnMut() -> T + 'static,
) -> ReadSignal<T> {
    // We store the memo's value in a normal signal so downstream effects /
    // memos can depend on it.
    let initial = false; // will be set by the first effect run
    let _ = initial;
    // Create a signal to hold the memo value. We'll initialise it with a
    // dummy that gets overwritten immediately by the first effect execution.
    //
    // To avoid requiring T: Default we run f() once outside the effect to
    // get the real initial value. This means the first effect run will
    // compare against itself and skip the write — that's fine.

    // First, compute initial value *with tracking disabled* so we don't
    // accidentally subscribe a parent effect.  We'll let the inner effect
    // do the real tracked run.
    //
    // Actually, we need to run with tracking so the memo effect picks up
    // dependencies. But we want to avoid subscribing a *parent* effect that
    // may be running right now. The trick: we temporarily clear `tracking`,
    // run f inside our own effect, and the effect machinery handles the rest.

    // Simplest approach: create signal with a value computed eagerly, then
    // wrap in an effect that keeps it up-to-date.
    let first_value: T = RUNTIME.with(|rt| {
        // Temporarily clear tracking so the eager evaluation doesn't
        // subscribe a parent effect.
        let prev = rt.borrow_mut().tracking.take();
        let val = f();
        rt.borrow_mut().tracking = prev;
        val
    });

    let (read, write) = create_signal(first_value);

    // The effect will re-run f whenever its dependencies change and
    // conditionally write the new value (only if it differs).
    create_effect(move || {
        let new_val = f();
        let changed = read.with(|old| old != &new_val);
        if changed {
            write.set(new_val);
        }
    });

    read
}

// ---------------------------------------------------------------------------
// Batch
// ---------------------------------------------------------------------------

/// Batch multiple signal writes so that effects run only once.
///
/// ```ignore
/// batch(|| {
///     set_a(1);
///     set_b(2);
/// });
/// // Effects that depend on a and/or b run once here.
/// ```
pub fn batch(f: impl FnOnce()) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().batch_depth += 1;
    });

    f();

    let pending = RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        rt_ref.batch_depth -= 1;
        if rt_ref.batch_depth == 0 {
            // Deduplicate
            let mut seen = HashSet::new();
            let pending: Vec<EffectId> = rt_ref
                .pending_effects
                .drain(..)
                .filter(|id| seen.insert(*id))
                .collect();
            pending
        } else {
            Vec::new()
        }
    });

    for eid in pending {
        run_effect(eid);
    }
}

// ---------------------------------------------------------------------------
// Dispose
// ---------------------------------------------------------------------------

/// Deactivate an effect so it no longer re-runs when its dependencies change.
pub fn dispose_effect(eid: EffectId) {
    RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        if eid.0 < rt_ref.effects.len() {
            rt_ref.effects[eid.0].active = false;
            // Remove from all signal subscriber lists.
            let deps: Vec<SignalId> = rt_ref.effects[eid.0].dependencies.drain().collect();
            for sid in deps {
                rt_ref.signals[sid.0].subscribers.remove(&eid);
            }
        }
    });
}

/// Create an effect and return its [`EffectId`] so it can later be disposed.
pub fn create_effect_with_id(f: impl FnMut() + 'static) -> EffectId {
    let eid = RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        let eid = EffectId(rt_ref.effects.len());
        rt_ref.effects.push(EffectState {
            callback: Some(Box::new(f)),
            dependencies: HashSet::new(),
            active: true,
        });
        eid
    });
    run_effect(eid);
    eid
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Run a single effect: clear old deps, set tracking, execute callback.
fn run_effect(eid: EffectId) {
    // Check if effect is still active; take the callback out.
    let maybe_cb = RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        if eid.0 >= rt_ref.effects.len() || !rt_ref.effects[eid.0].active {
            return None;
        }
        // Clear old dependency edges.
        let old_deps: Vec<SignalId> = rt_ref.effects[eid.0].dependencies.drain().collect();
        for sid in old_deps {
            rt_ref.signals[sid.0].subscribers.remove(&eid);
        }
        // Take the callback out so we can run it without borrowing Runtime.
        rt_ref.effects[eid.0].callback.take()
    });

    let Some(mut cb) = maybe_cb else {
        return;
    };

    // Set tracking context.
    let prev_tracking = RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        let prev = rt_ref.tracking.take();
        rt_ref.tracking = Some(eid);
        prev
    });

    // Run the user callback — signal reads will subscribe us.
    cb();

    // Restore tracking and put the callback back.
    RUNTIME.with(|rt| {
        let mut rt_ref = rt.borrow_mut();
        rt_ref.tracking = prev_tracking;
        // Put callback back (only if effect still active).
        if eid.0 < rt_ref.effects.len() && rt_ref.effects[eid.0].active {
            rt_ref.effects[eid.0].callback = Some(cb);
        }
    });
}

/// Notify a list of subscriber effects that a signal changed.
fn notify_subscribers(subs: Vec<EffectId>) {
    if subs.is_empty() {
        return;
    }

    let batching = RUNTIME.with(|rt| {
        let rt_ref = rt.borrow();
        rt_ref.batch_depth > 0
    });

    if batching {
        RUNTIME.with(|rt| {
            let mut rt_ref = rt.borrow_mut();
            rt_ref.pending_effects.extend(subs);
        });
        return;
    }

    // Guard against re-entrant notification (effect -> set -> effect -> ...).
    let already_running = RUNTIME.with(|rt| {
        let rt_ref = rt.borrow();
        rt_ref.running_effects
    });

    if already_running {
        // We're already inside the notification loop. Queue for later.
        RUNTIME.with(|rt| {
            let mut rt_ref = rt.borrow_mut();
            rt_ref.pending_effects.extend(subs);
        });
        return;
    }

    RUNTIME.with(|rt| {
        rt.borrow_mut().running_effects = true;
    });

    let mut queue: Vec<EffectId> = subs;
    while !queue.is_empty() {
        let current_batch = std::mem::take(&mut queue);
        for eid in current_batch {
            let active = RUNTIME.with(|rt| {
                let rt_ref = rt.borrow();
                eid.0 < rt_ref.effects.len() && rt_ref.effects[eid.0].active
            });
            if active {
                run_effect(eid);
            }
        }
        // Check if running effects triggered more pending effects.
        RUNTIME.with(|rt| {
            let mut rt_ref = rt.borrow_mut();
            queue.append(&mut rt_ref.pending_effects);
        });
    }

    RUNTIME.with(|rt| {
        rt.borrow_mut().running_effects = false;
    });
}

// ---------------------------------------------------------------------------
// Test helper: reset the thread-local runtime between tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) fn reset_runtime() {
    RUNTIME.with(|rt| {
        *rt.borrow_mut() = Runtime::new();
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Helper: reset before each test to avoid cross-contamination.
    fn setup() {
        reset_runtime();
    }

    #[test]
    fn create_and_read_signal() {
        setup();
        let (r, _w) = create_signal(42);
        assert_eq!(r.get(), 42);
    }

    #[test]
    fn set_and_read() {
        setup();
        let (r, w) = create_signal(0);
        w.set(7);
        assert_eq!(r.get(), 7);
    }

    #[test]
    fn update_in_place() {
        setup();
        let (r, w) = create_signal(vec![1, 2]);
        w.update(|v| v.push(3));
        assert_eq!(r.get(), vec![1, 2, 3]);
    }

    #[test]
    fn signal_with() {
        setup();
        let (r, _w) = create_signal(String::from("hello"));
        let len = r.with(|s| s.len());
        assert_eq!(len, 5);
    }

    #[test]
    fn get_untracked_does_not_subscribe() {
        setup();
        let (r, w) = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            let _ = r.get_untracked();
            count_c.set(count_c.get() + 1);
        });
        // Effect ran once at creation
        assert_eq!(count.get(), 1);
        // Setting the signal should NOT re-run the effect (untracked read)
        w.set(1);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn effect_runs_immediately() {
        setup();
        let ran = Rc::new(Cell::new(false));
        let ran_c = ran.clone();
        create_effect(move || {
            ran_c.set(true);
        });
        assert!(ran.get());
    }

    #[test]
    fn effect_tracks_single_signal() {
        setup();
        let (r, w) = create_signal(0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_c = log.clone();
        create_effect(move || {
            log_c.borrow_mut().push(r.get());
        });
        assert_eq!(*log.borrow(), vec![0]);
        w.set(1);
        assert_eq!(*log.borrow(), vec![0, 1]);
        w.set(2);
        assert_eq!(*log.borrow(), vec![0, 1, 2]);
    }

    #[test]
    fn effect_tracks_multiple_signals() {
        setup();
        let (a_r, a_w) = create_signal(1);
        let (b_r, b_w) = create_signal(10);
        let sum = Rc::new(Cell::new(0));
        let sum_c = sum.clone();
        create_effect(move || {
            sum_c.set(a_r.get() + b_r.get());
        });
        assert_eq!(sum.get(), 11);
        a_w.set(2);
        assert_eq!(sum.get(), 12);
        b_w.set(20);
        assert_eq!(sum.get(), 22);
    }

    #[test]
    fn effect_retracks_on_conditional_read() {
        setup();
        let (flag, set_flag) = create_signal(true);
        let (a, _set_a) = create_signal(1);
        let (b, set_b) = create_signal(2);
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_c = log.clone();

        create_effect(move || {
            let val = if flag.get() { a.get() } else { b.get() };
            log_c.borrow_mut().push(val);
        });
        assert_eq!(*log.borrow(), vec![1]);

        // Switch to reading b instead of a
        set_flag.set(false);
        assert_eq!(*log.borrow(), vec![1, 2]);

        // Changing b should trigger the effect now
        set_b.set(99);
        assert_eq!(*log.borrow(), vec![1, 2, 99]);
    }

    #[test]
    fn memo_basic() {
        setup();
        let (r, w) = create_signal(3);
        let doubled = create_memo(move || r.get() * 2);
        assert_eq!(doubled.get(), 6);
        w.set(5);
        assert_eq!(doubled.get(), 10);
    }

    #[test]
    fn memo_skips_when_unchanged() {
        setup();
        let (r, w) = create_signal(3);
        // Memo that clamps to max 10
        let clamped = create_memo(move || r.get().min(10));
        let run_count = Rc::new(Cell::new(0));
        let run_count_c = run_count.clone();
        create_effect(move || {
            let _ = clamped.get();
            run_count_c.set(run_count_c.get() + 1);
        });
        // Ran once at creation
        assert_eq!(run_count.get(), 1);
        // Change signal but memo output stays the same (3 -> 5, both < 10, so clamped changes)
        w.set(5);
        assert_eq!(run_count.get(), 2); // memo changed 3->5
        // Set to 15 -> clamped = 10
        w.set(15);
        assert_eq!(run_count.get(), 3); // memo changed 5->10
        // Set to 20 -> clamped = 10 still
        w.set(20);
        assert_eq!(run_count.get(), 3); // memo output unchanged!
    }

    #[test]
    fn batch_defers_effects() {
        setup();
        let (a_r, a_w) = create_signal(0);
        let (b_r, b_w) = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            let _ = a_r.get() + b_r.get();
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1); // initial run

        batch(|| {
            a_w.set(1);
            b_w.set(2);
        });
        // Should have run only once more (not twice)
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn dispose_stops_effect() {
        setup();
        let (r, w) = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        let eid = create_effect_with_id(move || {
            let _ = r.get();
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1);
        w.set(1);
        assert_eq!(count.get(), 2);

        dispose_effect(eid);
        w.set(2);
        assert_eq!(count.get(), 2); // no more runs
    }

    #[test]
    fn nested_effects() {
        setup();
        let (r, w) = create_signal(0);
        let outer = Rc::new(Cell::new(0));
        let inner = Rc::new(Cell::new(0));
        let outer_c = outer.clone();
        let inner_c = inner.clone();

        create_effect(move || {
            let val = r.get();
            outer_c.set(outer_c.get() + 1);
            // Create an inner effect the first time
            if val == 0 {
                let inner_cc = inner_c.clone();
                create_effect(move || {
                    let _ = r.get();
                    inner_cc.set(inner_cc.get() + 1);
                });
            }
        });

        // Outer ran once, inner created and ran once
        assert_eq!(outer.get(), 1);
        assert_eq!(inner.get(), 1);

        w.set(1);
        // Both outer and inner should have re-run
        assert_eq!(outer.get(), 2);
        assert_eq!(inner.get(), 2);
    }

    #[test]
    fn debug_read_signal() {
        setup();
        let (r, _w) = create_signal(42);
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("ReadSignal"));
        assert!(dbg.contains("id"));
    }

    #[test]
    fn debug_write_signal() {
        setup();
        let (_r, w) = create_signal(42);
        let dbg = format!("{:?}", w);
        assert!(dbg.contains("WriteSignal"));
        assert!(dbg.contains("id"));
    }

    #[test]
    fn signal_with_string_type() {
        setup();
        let (r, w) = create_signal(String::from("hello"));
        assert_eq!(r.get(), "hello");
        w.set(String::from("world"));
        assert_eq!(r.get(), "world");
    }

    #[test]
    fn multiple_effects_on_same_signal() {
        setup();
        let (r, w) = create_signal(0);
        let a = Rc::new(Cell::new(0));
        let b = Rc::new(Cell::new(0));
        let a_c = a.clone();
        let b_c = b.clone();

        create_effect(move || {
            a_c.set(r.get());
        });
        create_effect(move || {
            b_c.set(r.get() * 10);
        });

        assert_eq!(a.get(), 0);
        assert_eq!(b.get(), 0);

        w.set(3);
        assert_eq!(a.get(), 3);
        assert_eq!(b.get(), 30);
    }

    #[test]
    fn effect_set_during_effect() {
        setup();
        // Setting a signal inside an effect should work (no infinite loop)
        let (a_r, a_w) = create_signal(0);
        let (b_r, b_w) = create_signal(0);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();

        // Effect 1: when a changes, set b = a * 2
        create_effect(move || {
            let val = a_r.get();
            b_w.set(val * 2);
        });

        // Effect 2: log b's value
        create_effect(move || {
            log_c.borrow_mut().push(b_r.get());
        });

        // After setup: a=0 -> b=0, log = [0, 0]  (effect 2 ran at creation, then again when b set)
        a_w.set(5);
        // a=5 -> effect1 sets b=10 -> effect2 logs 10
        assert!(log.borrow().contains(&10));
    }

    #[test]
    fn batch_nested() {
        setup();
        let (r, w) = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            let _ = r.get();
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1);

        batch(|| {
            w.set(1);
            batch(|| {
                w.set(2);
            });
            // Inner batch shouldn't flush yet because outer batch is still open
            w.set(3);
        });
        // Should have only run once after the outermost batch completes
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn memo_chain() {
        setup();
        let (r, w) = create_signal(1);
        let doubled = create_memo(move || r.get() * 2);
        let quadrupled = create_memo(move || doubled.get() * 2);
        assert_eq!(quadrupled.get(), 4);
        w.set(3);
        assert_eq!(doubled.get(), 6);
        assert_eq!(quadrupled.get(), 12);
    }

    #[test]
    fn read_signal_copy() {
        setup();
        let (r, _w) = create_signal(42);
        let r2 = r; // Copy
        assert_eq!(r.get(), 42);
        assert_eq!(r2.get(), 42);
    }

    #[test]
    fn write_signal_copy() {
        setup();
        let (r, w) = create_signal(0);
        let w2 = w; // Copy
        w2.set(10);
        assert_eq!(r.get(), 10);
        w.set(20);
        assert_eq!(r.get(), 20);
    }

    #[test]
    fn effect_not_run_after_dispose() {
        setup();
        let (r, w) = create_signal(0);
        let values = Rc::new(RefCell::new(Vec::new()));
        let values_c = values.clone();
        let eid = create_effect_with_id(move || {
            values_c.borrow_mut().push(r.get());
        });
        assert_eq!(*values.borrow(), vec![0]);
        w.set(1);
        assert_eq!(*values.borrow(), vec![0, 1]);

        dispose_effect(eid);
        w.set(2);
        w.set(3);
        // Still only [0, 1]
        assert_eq!(*values.borrow(), vec![0, 1]);
    }

    #[test]
    fn many_signals_one_effect() {
        setup();
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        for i in 0..5 {
            let (r, w) = create_signal(i);
            reads.push(r);
            writes.push(w);
        }
        let sum = Rc::new(Cell::new(0));
        let sum_c = sum.clone();
        let reads_clone = reads.clone();
        create_effect(move || {
            let s: i32 = reads_clone.iter().map(|r| r.get()).sum();
            sum_c.set(s);
        });
        // 0+1+2+3+4 = 10
        assert_eq!(sum.get(), 10);
        writes[2].set(100);
        // 0+1+100+3+4 = 108
        assert_eq!(sum.get(), 108);
    }
}
