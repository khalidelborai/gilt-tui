//! Widget lifecycle: mount, unmount, update cycle.
//!
//! The `LifecycleTracker` records which widgets are currently mounted in the DOM
//! and accumulates lifecycle events (`Mount`, `Unmount`, `Update`) that can be
//! drained and processed by the application loop.

use std::collections::HashSet;

use crate::dom::node::NodeId;

// ---------------------------------------------------------------------------
// LifecycleEvent
// ---------------------------------------------------------------------------

/// Events that occur during the widget lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// A widget was mounted into the DOM.
    Mount { node_id: NodeId },
    /// A widget was unmounted from the DOM.
    Unmount { node_id: NodeId },
    /// A widget's state or props changed and it needs re-rendering.
    Update { node_id: NodeId },
}

// ---------------------------------------------------------------------------
// LifecycleTracker
// ---------------------------------------------------------------------------

/// Tracks which nodes are currently mounted and accumulates lifecycle events.
///
/// The tracker maintains a set of mounted node ids and a queue of pending events.
/// Events can be drained to process them (e.g., run mount handlers, trigger
/// re-renders on update, clean up on unmount).
#[derive(Debug)]
pub struct LifecycleTracker {
    /// Set of currently mounted node ids.
    mounted: HashSet<NodeId>,
    /// Pending lifecycle events, in order of occurrence.
    pending: Vec<LifecycleEvent>,
}

impl LifecycleTracker {
    /// Create a new, empty lifecycle tracker.
    pub fn new() -> Self {
        Self {
            mounted: HashSet::new(),
            pending: Vec::new(),
        }
    }

    /// Record that a node has been mounted.
    ///
    /// If the node was already mounted, this is a no-op (no duplicate event).
    pub fn on_mount(&mut self, id: NodeId) {
        if self.mounted.insert(id) {
            self.pending.push(LifecycleEvent::Mount { node_id: id });
        }
    }

    /// Record that a node has been unmounted.
    ///
    /// If the node was not mounted, this is a no-op (no spurious event).
    pub fn on_unmount(&mut self, id: NodeId) {
        if self.mounted.remove(&id) {
            self.pending.push(LifecycleEvent::Unmount { node_id: id });
        }
    }

    /// Record that a mounted node needs updating.
    ///
    /// If the node is not currently mounted, this is a no-op.
    pub fn on_update(&mut self, id: NodeId) {
        if self.mounted.contains(&id) {
            self.pending.push(LifecycleEvent::Update { node_id: id });
        }
    }

    /// Check whether a node is currently mounted.
    pub fn is_mounted(&self, id: NodeId) -> bool {
        self.mounted.contains(&id)
    }

    /// Return all currently mounted node ids.
    ///
    /// The order is not guaranteed (uses `HashSet` internally).
    pub fn mounted_nodes(&self) -> Vec<NodeId> {
        self.mounted.iter().copied().collect()
    }

    /// The number of currently mounted nodes.
    pub fn mounted_count(&self) -> usize {
        self.mounted.len()
    }

    /// Drain and return all pending lifecycle events.
    ///
    /// After calling this, the pending event queue is empty.
    pub fn pending_events(&mut self) -> Vec<LifecycleEvent> {
        std::mem::take(&mut self.pending)
    }

    /// Whether there are any pending events.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all state (mounted nodes and pending events).
    pub fn clear(&mut self) {
        self.mounted.clear();
        self.pending.clear();
    }
}

impl Default for LifecycleTracker {
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
    use slotmap::SlotMap;

    /// Helper to create a fresh NodeId from a slotmap.
    fn make_id(sm: &mut SlotMap<NodeId, ()>) -> NodeId {
        sm.insert(())
    }

    #[test]
    fn new_tracker_is_empty() {
        let tracker = LifecycleTracker::new();
        assert_eq!(tracker.mounted_count(), 0);
        assert!(tracker.mounted_nodes().is_empty());
        assert!(!tracker.has_pending());
    }

    #[test]
    fn mount_adds_node() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        assert!(tracker.is_mounted(id));
        assert_eq!(tracker.mounted_count(), 1);
    }

    #[test]
    fn mount_produces_event() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        let events = tracker.pending_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], LifecycleEvent::Mount { node_id: id });
    }

    #[test]
    fn double_mount_is_noop() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        tracker.on_mount(id); // duplicate
        assert_eq!(tracker.mounted_count(), 1);
        let events = tracker.pending_events();
        assert_eq!(events.len(), 1); // only one Mount event
    }

    #[test]
    fn unmount_removes_node() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        tracker.on_unmount(id);
        assert!(!tracker.is_mounted(id));
        assert_eq!(tracker.mounted_count(), 0);
    }

    #[test]
    fn unmount_produces_event() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        let _ = tracker.pending_events(); // drain mount event
        tracker.on_unmount(id);
        let events = tracker.pending_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], LifecycleEvent::Unmount { node_id: id });
    }

    #[test]
    fn unmount_not_mounted_is_noop() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_unmount(id); // never mounted
        assert!(!tracker.has_pending());
    }

    #[test]
    fn update_mounted_node() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        let _ = tracker.pending_events();
        tracker.on_update(id);
        let events = tracker.pending_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], LifecycleEvent::Update { node_id: id });
    }

    #[test]
    fn update_unmounted_is_noop() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_update(id); // never mounted
        assert!(!tracker.has_pending());
    }

    #[test]
    fn pending_events_drains() {
        let mut sm = SlotMap::with_key();
        let a = make_id(&mut sm);
        let b = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(a);
        tracker.on_mount(b);
        let events = tracker.pending_events();
        assert_eq!(events.len(), 2);

        // Second drain is empty.
        let events2 = tracker.pending_events();
        assert!(events2.is_empty());
    }

    #[test]
    fn mounted_nodes_returns_all() {
        let mut sm = SlotMap::with_key();
        let a = make_id(&mut sm);
        let b = make_id(&mut sm);
        let c = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(a);
        tracker.on_mount(b);
        tracker.on_mount(c);

        let nodes = tracker.mounted_nodes();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn clear_resets_everything() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        tracker.on_mount(id);
        tracker.clear();
        assert!(!tracker.is_mounted(id));
        assert_eq!(tracker.mounted_count(), 0);
        assert!(!tracker.has_pending());
    }

    #[test]
    fn full_lifecycle_sequence() {
        let mut sm = SlotMap::with_key();
        let id = make_id(&mut sm);
        let mut tracker = LifecycleTracker::new();

        // Mount
        tracker.on_mount(id);
        // Update
        tracker.on_update(id);
        // Unmount
        tracker.on_unmount(id);

        let events = tracker.pending_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], LifecycleEvent::Mount { node_id: id });
        assert_eq!(events[1], LifecycleEvent::Update { node_id: id });
        assert_eq!(events[2], LifecycleEvent::Unmount { node_id: id });
    }

    #[test]
    fn default_impl() {
        let tracker = LifecycleTracker::default();
        assert_eq!(tracker.mounted_count(), 0);
    }
}
