//! Event dispatch: message queue and bubble path computation.
//!
//! [`EventDispatcher`] maintains a queue of [`Envelope`]s. The `bubble_path`
//! static method computes the traversal order from a node up to the DOM root
//! for bubble-phase message delivery.

use std::collections::VecDeque;

use super::message::Envelope;
use crate::dom::node::NodeId;
use crate::dom::tree::Dom;

// ---------------------------------------------------------------------------
// EventDispatcher
// ---------------------------------------------------------------------------

/// Queue-based event dispatcher.
///
/// Messages are enqueued via `push` and drained for processing via `drain`.
/// The dispatcher does not itself route messages — that responsibility belongs
/// to the application loop, which uses `bubble_path` and the DOM to walk
/// messages through the widget hierarchy.
#[derive(Debug)]
pub struct EventDispatcher {
    queue: VecDeque<Envelope>,
}

impl EventDispatcher {
    /// Create a new, empty dispatcher.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Enqueue a message envelope for later processing.
    pub fn push(&mut self, envelope: Envelope) {
        self.queue.push_back(envelope);
    }

    /// Drain all pending messages and return them as a `Vec`.
    ///
    /// The queue is empty after this call.
    pub fn drain(&mut self) -> Vec<Envelope> {
        self.queue.drain(..).collect()
    }

    /// Number of pending messages.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Compute the bubble path from `start` up to the root (inclusive).
    ///
    /// Returns `[start, parent, grandparent, ..., root]`.
    /// If `start` does not exist in the DOM, returns an empty vec.
    pub fn bubble_path(dom: &Dom, start: NodeId) -> Vec<NodeId> {
        if !dom.contains(start) {
            return Vec::new();
        }
        let mut path = vec![start];
        let ancestors = dom.ancestors(start);
        path.extend(ancestors);
        path
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::node::NodeData;
    use crate::event::message::{Custom, Quit, Refresh};

    /// Build a small test tree:
    /// ```text
    ///       root
    ///      /    \
    ///    a        b
    ///   / \
    ///  c   d
    /// ```
    fn build_tree() -> (Dom, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Container"));
        let a = dom.insert_child(root, NodeData::new("Panel"));
        let b = dom.insert_child(root, NodeData::new("Panel"));
        let c = dom.insert_child(a, NodeData::new("Button"));
        let d = dom.insert_child(a, NodeData::new("Label"));
        (dom, root, a, b, c, d)
    }

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn new_dispatcher_is_empty() {
        let disp = EventDispatcher::new();
        assert!(disp.is_empty());
        assert_eq!(disp.pending_count(), 0);
    }

    #[test]
    fn default_dispatcher_is_empty() {
        let disp = EventDispatcher::default();
        assert!(disp.is_empty());
    }

    // ── Push / Drain ─────────────────────────────────────────────────

    #[test]
    fn push_and_drain() {
        let (_, root, ..) = build_tree();
        let mut disp = EventDispatcher::new();
        disp.push(Envelope::new(Quit, root));
        disp.push(Envelope::new(Refresh, root));

        assert_eq!(disp.pending_count(), 2);
        assert!(!disp.is_empty());

        let messages = disp.drain();
        assert_eq!(messages.len(), 2);
        assert!(disp.is_empty());
        assert_eq!(disp.pending_count(), 0);
    }

    #[test]
    fn drain_empty() {
        let mut disp = EventDispatcher::new();
        let messages = disp.drain();
        assert!(messages.is_empty());
    }

    #[test]
    fn push_preserves_order() {
        let (_, root, ..) = build_tree();
        let mut disp = EventDispatcher::new();
        disp.push(Envelope::new(Custom::new("first"), root));
        disp.push(Envelope::new(Custom::new("second"), root));
        disp.push(Envelope::new(Custom::new("third"), root));

        let messages = disp.drain();
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages[0].downcast_ref::<Custom>().unwrap().0,
            "first"
        );
        assert_eq!(
            messages[1].downcast_ref::<Custom>().unwrap().0,
            "second"
        );
        assert_eq!(
            messages[2].downcast_ref::<Custom>().unwrap().0,
            "third"
        );
    }

    // ── Bubble path ──────────────────────────────────────────────────

    #[test]
    fn bubble_path_from_leaf() {
        let (dom, root, a, _, c, _) = build_tree();
        let path = EventDispatcher::bubble_path(&dom, c);
        assert_eq!(path, vec![c, a, root]);
    }

    #[test]
    fn bubble_path_from_root() {
        let (dom, root, ..) = build_tree();
        let path = EventDispatcher::bubble_path(&dom, root);
        assert_eq!(path, vec![root]);
    }

    #[test]
    fn bubble_path_from_middle() {
        let (dom, root, a, ..) = build_tree();
        let path = EventDispatcher::bubble_path(&dom, a);
        assert_eq!(path, vec![a, root]);
    }

    #[test]
    fn bubble_path_nonexistent_node() {
        let (mut dom, ..) = build_tree();
        // Create and remove a node to get a stale id.
        let stale = dom.insert(NodeData::new("Ghost"));
        dom.remove(stale);
        let path = EventDispatcher::bubble_path(&dom, stale);
        assert!(path.is_empty());
    }

    #[test]
    fn bubble_path_sibling() {
        let (dom, root, _, b, ..) = build_tree();
        let path = EventDispatcher::bubble_path(&dom, b);
        assert_eq!(path, vec![b, root]);
    }
}
