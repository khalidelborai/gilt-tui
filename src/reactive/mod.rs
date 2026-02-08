//! Reactive state: signals, effects, memos.
//!
//! Leptos-style fine-grained reactivity for driving widget updates.
//!
//! - [`create_signal`] — create a read/write signal pair.
//! - [`create_effect`] — auto-tracking side effect.
//! - [`create_memo`] — cached derived computation.
//! - [`batch`] — coalesce multiple writes into one notification pass.

pub mod signal;
pub mod effect;

pub use signal::{create_signal, ReadSignal, WriteSignal};
pub use effect::{batch, create_effect, create_effect_with_id, create_memo, dispose_effect, EffectId};
