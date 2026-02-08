//! Rendering pipeline: compositor, strip assembly, terminal driver.

pub mod compositor;
pub mod strip;
pub mod driver;

pub use strip::{Strip, StyledCell, CellStyle};
pub use compositor::{Compositor, CellUpdate};
pub use driver::Driver;
