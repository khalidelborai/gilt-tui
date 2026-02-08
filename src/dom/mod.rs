//! DOM arena: slotmap-backed widget tree with CSS class/id queries.

pub mod node;
pub mod tree;
pub mod query;

pub use node::{NodeId, NodeData};
pub use tree::Dom;
