//! # gilt-tui
//!
//! A CSS-styled, declarative terminal UI framework built on [gilt](https://crates.io/crates/gilt).
//!
//! gilt-tui brings CSS styling, a retained DOM, and fine-grained reactivity to Rust terminal
//! applications. Inspired by Python's [Textual](https://textual.textualize.io/), but designed
//! as a Rust-native system with type-safe CSS properties, Leptos-style signals, and builder
//! composition.
//!
//! ## Core Systems
//!
//! - **[`css`]** — Custom CSS engine: tokenizer, parser, specificity, cascade
//! - **[`dom`]** — Slotmap-backed DOM arena with tree operations and selector matching
//! - **[`layout`]** — Taffy-powered flexbox/grid layout with CSS scalar resolution
//! - **[`widget`]** — Widget trait, lifecycle tracking, scroll state
//! - **[`widgets`]** — Built-in widgets: Static, Container, Button, Header, Footer, Input
//! - **[`event`]** — Input events, message bubbling, key bindings
//! - **[`reactive`]** — Signals, effects, memos (Leptos-style auto-tracking)
//! - **[`render`]** — Strip-based compositor with dirty tracking and crossterm driver
//! - **[`app`]** — Application struct tying everything together
//! - **[`screen`]** — Screen management with focus chain
//! - **[`geometry`]** — Offset, Size, Region, Spacing primitives

// Foundation
pub mod geometry;

// Core systems
pub mod css;
pub mod dom;
pub mod layout;

// Widget system
pub mod widget;
pub mod widgets;

// Events and reactivity
pub mod event;
pub mod reactive;

// Rendering
pub mod render;

// Application
pub mod app;
pub mod screen;

// Proc macros (feature-gated)
#[cfg(feature = "macros")]
pub use gilt_tui_macros::{view, css};
