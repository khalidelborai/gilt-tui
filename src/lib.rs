//! # gilt-tui
//!
//! A CSS-styled, declarative terminal UI framework built on [gilt](https://crates.io/crates/gilt).
//!
//! gilt-tui brings CSS styling, a retained DOM, and async reactivity to Rust terminal applications.
//! Inspired by Python's [textual](https://textual.textualize.io/), but designed as a Rust-native
//! system with type-safe CSS properties, Leptos-style signals, and builder + RSX composition.

pub mod geometry;

pub mod dom;
pub mod css;
pub mod layout;
pub mod widget;
pub mod event;
pub mod reactive;
pub mod render;

pub mod app;
pub mod screen;

pub mod widgets;
