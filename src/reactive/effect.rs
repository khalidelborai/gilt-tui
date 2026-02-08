//! Auto-tracking side effects and computed memos.
//!
//! This module re-exports the effect and memo APIs from [`super::signal`] and
//! provides additional convenience functions for working with reactive
//! computations.
//!
//! # Effects
//!
//! An effect is a closure that re-runs whenever any signal it reads changes:
//!
//! ```ignore
//! let (count, set_count) = create_signal(0);
//! create_effect(move || {
//!     println!("count = {}", count.get());
//! });
//! set_count.set(1); // prints "count = 1"
//! ```
//!
//! # Memos
//!
//! A memo is a cached derived computation — it only notifies downstream
//! subscribers when its output actually changes:
//!
//! ```ignore
//! let (count, set_count) = create_signal(3);
//! let doubled = create_memo(move || count.get() * 2);
//! assert_eq!(doubled.get(), 6);
//! ```
//!
//! # Batching
//!
//! Use [`batch`] to group multiple signal writes so that effects run only once:
//!
//! ```ignore
//! batch(|| {
//!     set_a.set(1);
//!     set_b.set(2);
//! });
//! // Effects depending on a and/or b execute once here.
//! ```

pub use super::signal::{
    batch, create_effect, create_effect_with_id, create_memo, dispose_effect, EffectId,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::signal::{create_signal, reset_runtime, ReadSignal};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn setup() {
        reset_runtime();
    }

    // ------------------------------------------------------------------
    // Effect basics
    // ------------------------------------------------------------------

    #[test]
    fn effect_runs_on_creation() {
        setup();
        let ran = Rc::new(Cell::new(false));
        let ran_c = ran.clone();
        create_effect(move || {
            ran_c.set(true);
        });
        assert!(ran.get());
    }

    #[test]
    fn effect_reruns_on_signal_change() {
        setup();
        let (r, w) = create_signal(0_i32);
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_c = log.clone();
        create_effect(move || {
            log_c.borrow_mut().push(r.get());
        });
        assert_eq!(*log.borrow(), vec![0]);
        w.set(42);
        assert_eq!(*log.borrow(), vec![0, 42]);
    }

    #[test]
    fn effect_tracks_multiple_signals() {
        setup();
        let (a, set_a) = create_signal(1_i32);
        let (b, set_b) = create_signal(2_i32);
        let sum = Rc::new(Cell::new(0));
        let sum_c = sum.clone();
        create_effect(move || {
            sum_c.set(a.get() + b.get());
        });
        assert_eq!(sum.get(), 3);
        set_a.set(10);
        assert_eq!(sum.get(), 12);
        set_b.set(20);
        assert_eq!(sum.get(), 30);
    }

    #[test]
    fn effect_retracks_conditional_deps() {
        setup();
        let (flag, set_flag) = create_signal(true);
        let (x, _set_x) = create_signal(100_i32);
        let (y, set_y) = create_signal(200_i32);
        let result = Rc::new(Cell::new(0));
        let result_c = result.clone();

        create_effect(move || {
            let v = if flag.get() { x.get() } else { y.get() };
            result_c.set(v);
        });
        assert_eq!(result.get(), 100);

        // After switching, changing y should trigger
        set_flag.set(false);
        assert_eq!(result.get(), 200);
        set_y.set(999);
        assert_eq!(result.get(), 999);
    }

    #[test]
    fn effect_with_id_returns_id() {
        setup();
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        let eid = create_effect_with_id(move || {
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1);
        // eid should be a valid EffectId
        let _ = eid;
    }

    // ------------------------------------------------------------------
    // Memo
    // ------------------------------------------------------------------

    #[test]
    fn memo_computes_correctly() {
        setup();
        let (r, w) = create_signal(4_i32);
        let squared = create_memo(move || r.get() * r.get());
        assert_eq!(squared.get(), 16);
        w.set(5);
        assert_eq!(squared.get(), 25);
    }

    #[test]
    fn memo_only_notifies_on_change() {
        setup();
        let (r, w) = create_signal(5_i32);
        // Memo: value clamped to [0, 10]
        let clamped: ReadSignal<i32> = create_memo(move || r.get().clamp(0, 10));
        let downstream_runs = Rc::new(Cell::new(0_u32));
        let dr = downstream_runs.clone();
        create_effect(move || {
            let _ = clamped.get();
            dr.set(dr.get() + 1);
        });
        assert_eq!(downstream_runs.get(), 1);

        // Change to 7 -> clamped changes from 5 to 7
        w.set(7);
        assert_eq!(downstream_runs.get(), 2);

        // Change to 15 -> clamped = 10
        w.set(15);
        assert_eq!(downstream_runs.get(), 3);
        assert_eq!(clamped.get(), 10);

        // Change to 20 -> clamped still 10, downstream should NOT re-run
        w.set(20);
        assert_eq!(downstream_runs.get(), 3);
    }

    #[test]
    fn memo_chain() {
        setup();
        let (base, set_base) = create_signal(2_i32);
        let doubled = create_memo(move || base.get() * 2);
        let quadrupled = create_memo(move || doubled.get() * 2);
        assert_eq!(quadrupled.get(), 8);
        set_base.set(5);
        assert_eq!(doubled.get(), 10);
        assert_eq!(quadrupled.get(), 20);
    }

    #[test]
    fn memo_with_string() {
        setup();
        let (name, set_name) = create_signal(String::from("alice"));
        let upper = create_memo(move || name.get().to_uppercase());
        assert_eq!(upper.get(), "ALICE");
        set_name.set(String::from("bob"));
        assert_eq!(upper.get(), "BOB");
    }

    // ------------------------------------------------------------------
    // Batch
    // ------------------------------------------------------------------

    #[test]
    fn batch_coalesces_notifications() {
        setup();
        let (a, set_a) = create_signal(0_i32);
        let (b, set_b) = create_signal(0_i32);
        let runs = Rc::new(Cell::new(0_u32));
        let runs_c = runs.clone();
        create_effect(move || {
            let _ = a.get() + b.get();
            runs_c.set(runs_c.get() + 1);
        });
        assert_eq!(runs.get(), 1);

        batch(|| {
            set_a.set(10);
            set_b.set(20);
        });
        assert_eq!(runs.get(), 2); // only one extra run
    }

    #[test]
    fn batch_nested_only_flushes_once() {
        setup();
        let (r, w) = create_signal(0_i32);
        let runs = Rc::new(Cell::new(0_u32));
        let runs_c = runs.clone();
        create_effect(move || {
            let _ = r.get();
            runs_c.set(runs_c.get() + 1);
        });
        assert_eq!(runs.get(), 1);

        batch(|| {
            w.set(1);
            batch(|| {
                w.set(2);
            });
            w.set(3);
        });
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn batch_with_no_changes() {
        setup();
        let (_r, _w) = create_signal(0_i32);
        // Should not panic
        batch(|| {});
    }

    // ------------------------------------------------------------------
    // Dispose
    // ------------------------------------------------------------------

    #[test]
    fn dispose_stops_effect_from_running() {
        setup();
        let (r, w) = create_signal(0_i32);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();
        let eid = create_effect_with_id(move || {
            log_c.borrow_mut().push(r.get());
        });
        w.set(1);
        assert_eq!(*log.borrow(), vec![0, 1]);
        dispose_effect(eid);
        w.set(2);
        w.set(3);
        assert_eq!(*log.borrow(), vec![0, 1]); // no more entries
    }

    #[test]
    fn dispose_idempotent() {
        setup();
        let eid = create_effect_with_id(|| {});
        dispose_effect(eid);
        dispose_effect(eid); // should not panic
    }

    // ------------------------------------------------------------------
    // Nested / advanced
    // ------------------------------------------------------------------

    #[test]
    fn nested_effect_both_track() {
        setup();
        let (r, w) = create_signal(0_i32);
        let outer_runs = Rc::new(Cell::new(0_u32));
        let inner_runs = Rc::new(Cell::new(0_u32));
        let outer_c = outer_runs.clone();
        let inner_c = inner_runs.clone();

        create_effect(move || {
            let _ = r.get();
            outer_c.set(outer_c.get() + 1);
            if outer_c.get() == 1 {
                let inner_cc = inner_c.clone();
                create_effect(move || {
                    let _ = r.get();
                    inner_cc.set(inner_cc.get() + 1);
                });
            }
        });
        assert_eq!(outer_runs.get(), 1);
        assert_eq!(inner_runs.get(), 1);

        w.set(1);
        assert_eq!(outer_runs.get(), 2);
        assert_eq!(inner_runs.get(), 2);
    }

    #[test]
    fn effect_that_sets_signal_does_not_infinite_loop() {
        setup();
        let (a, set_a) = create_signal(0_i32);
        let (b, _set_b) = create_signal(0_i32);
        let b_write = _set_b;
        let runs = Rc::new(Cell::new(0_u32));
        let runs_c = runs.clone();

        // Effect: read a, write b
        create_effect(move || {
            let v = a.get();
            b_write.set(v * 2);
            runs_c.set(runs_c.get() + 1);
        });

        assert_eq!(b.get(), 0);
        set_a.set(5);
        assert_eq!(b.get(), 10);
        // Should have run a finite number of times (no infinite loop)
        assert!(runs.get() <= 5);
    }

    #[test]
    fn memo_as_effect_dependency() {
        setup();
        let (r, w) = create_signal(3_i32);
        let doubled = create_memo(move || r.get() * 2);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();
        create_effect(move || {
            log_c.borrow_mut().push(doubled.get());
        });
        assert_eq!(*log.borrow(), vec![6]);
        w.set(5);
        assert_eq!(*log.borrow(), vec![6, 10]);
    }

    #[test]
    fn multiple_memos_from_same_signal() {
        setup();
        let (r, w) = create_signal(10_i32);
        let plus_one = create_memo(move || r.get() + 1);
        let times_two = create_memo(move || r.get() * 2);
        assert_eq!(plus_one.get(), 11);
        assert_eq!(times_two.get(), 20);
        w.set(5);
        assert_eq!(plus_one.get(), 6);
        assert_eq!(times_two.get(), 10);
    }

    #[test]
    fn effect_runs_after_batch_even_if_value_same_intermediate() {
        setup();
        // Within a batch: set to 1, then back to 0. The effect should still
        // run because the signal was dirty during the batch, even though the
        // final value may match the original. (This is standard behavior —
        // signals don't diff, memos do.)
        let (r, w) = create_signal(0_i32);
        let runs = Rc::new(Cell::new(0_u32));
        let runs_c = runs.clone();
        create_effect(move || {
            let _ = r.get();
            runs_c.set(runs_c.get() + 1);
        });
        assert_eq!(runs.get(), 1);

        batch(|| {
            w.set(1);
            w.set(0); // back to original
        });
        // Effect should still run (signal doesn't compare values)
        assert_eq!(runs.get(), 2);
    }
}
