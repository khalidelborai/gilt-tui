//! Headless testing framework: Pilot, snapshot helpers.
//!
//! Use the [`Pilot`] to programmatically drive an [`App`](crate::app::App) without
//! a real terminal. Use [`render_to_string`] and related helpers to capture widget
//! output as plain text for snapshot-style assertions.

pub mod pilot;
pub mod snapshot;

pub use pilot::Pilot;
pub use snapshot::render_to_string;
