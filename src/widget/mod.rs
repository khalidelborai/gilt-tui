//! Widget system: trait, lifecycle, scrolling.

pub mod traits;
pub mod lifecycle;
pub mod scroll;

pub use traits::{Widget, WidgetBuilder, WidgetExt};
pub use lifecycle::{LifecycleEvent, LifecycleTracker};
pub use scroll::{ScrollState, ScrollbarState};
