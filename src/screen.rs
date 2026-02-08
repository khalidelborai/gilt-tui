//! Screen stack, focus chain, widget lookup.
//!
//! [`Screen`] owns the DOM, styles, layout engine, compositor, lifecycle tracker,
//! and focus chain for a single screen of the application. [`FocusChain`] maintains
//! the tab-order of focusable, visible, non-disabled nodes.

use std::collections::HashMap;

use crate::css::styles::Styles;
use crate::css::stylesheet::CompiledStylesheet;
use crate::dom::node::NodeId;
use crate::dom::tree::Dom;
use crate::layout::LayoutEngine;
use crate::render::compositor::Compositor;
use crate::widget::lifecycle::LifecycleTracker;

// ---------------------------------------------------------------------------
// FocusChain
// ---------------------------------------------------------------------------

/// Maintains an ordered list of focusable nodes for tab navigation.
///
/// The chain is rebuilt from the DOM whenever the tree changes. Focus cycles
/// through the chain in forward (Tab) or backward (Shift+Tab / BackTab) order.
#[derive(Debug)]
pub struct FocusChain {
    /// Focusable nodes in tab order (depth-first).
    nodes: Vec<NodeId>,
    /// Index of the currently focused node, or `None` if no focus.
    current: Option<usize>,
}

impl FocusChain {
    /// Create a new, empty focus chain.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current: None,
        }
    }

    /// Rebuild the focus chain from the DOM.
    ///
    /// Walks the DOM depth-first from the root and collects all nodes that are
    /// focusable, visible, and not disabled. If the previously focused node is
    /// still in the new chain, focus is preserved; otherwise focus is cleared.
    pub fn rebuild(&mut self, dom: &Dom) {
        let old_focused = self.current_node();

        self.nodes.clear();
        self.current = None;

        let root = match dom.root() {
            Some(r) => r,
            None => return,
        };

        for id in dom.walk_depth_first(root) {
            if let Some(data) = dom.get(id) {
                if data.focusable && data.visible && !data.disabled {
                    self.nodes.push(id);
                }
            }
        }

        // Try to preserve the previously focused node.
        if let Some(old_id) = old_focused {
            if let Some(pos) = self.nodes.iter().position(|&n| n == old_id) {
                self.current = Some(pos);
            }
        }
    }

    /// The currently focused node, if any.
    pub fn current_node(&self) -> Option<NodeId> {
        self.current.and_then(|idx| self.nodes.get(idx).copied())
    }

    /// Move focus to the next node in the chain. Wraps around.
    ///
    /// Returns the newly focused node, or `None` if the chain is empty.
    pub fn focus_next(&mut self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            return None;
        }
        let next = match self.current {
            Some(idx) => (idx + 1) % self.nodes.len(),
            None => 0,
        };
        self.current = Some(next);
        self.nodes.get(next).copied()
    }

    /// Move focus to the previous node in the chain. Wraps around.
    ///
    /// Returns the newly focused node, or `None` if the chain is empty.
    pub fn focus_previous(&mut self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            return None;
        }
        let prev = match self.current {
            Some(0) => self.nodes.len() - 1,
            Some(idx) => idx - 1,
            None => self.nodes.len() - 1,
        };
        self.current = Some(prev);
        self.nodes.get(prev).copied()
    }

    /// Focus a specific node by id. Returns `true` if the node was found.
    pub fn focus_node(&mut self, id: NodeId) -> bool {
        if let Some(pos) = self.nodes.iter().position(|&n| n == id) {
            self.current = Some(pos);
            true
        } else {
            false
        }
    }

    /// Clear focus (no node focused).
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Number of focusable nodes in the chain.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for FocusChain {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

/// A single screen: DOM, styles, layout, compositor, lifecycle, focus.
///
/// The `Screen` is the central owner of all per-screen state. It is created
/// with a viewport size and can be resized.
pub struct Screen {
    /// The DOM tree.
    pub dom: Dom,
    /// Computed styles per node.
    pub styles: HashMap<NodeId, Styles>,
    /// Taffy-based layout engine.
    pub layout: LayoutEngine,
    /// Screen buffer and dirty tracking.
    pub compositor: Compositor,
    /// Widget mount/unmount/update tracking.
    pub lifecycle: LifecycleTracker,
    /// Tab-order focus chain.
    pub focus: FocusChain,
    /// Compiled CSS stylesheets to apply.
    pub css: Vec<CompiledStylesheet>,
}

impl Screen {
    /// Create a new screen with the given viewport dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            dom: Dom::new(),
            styles: HashMap::new(),
            layout: LayoutEngine::new(),
            compositor: Compositor::new(width, height),
            lifecycle: LifecycleTracker::new(),
            focus: FocusChain::new(),
            css: Vec::new(),
        }
    }

    /// Resize the screen viewport.
    ///
    /// Updates the compositor dimensions and marks the entire screen dirty.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.compositor.resize(width, height);
    }

    /// The currently focused node, if any.
    pub fn focused_node(&self) -> Option<NodeId> {
        self.focus.current_node()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::node::NodeData;

    // ── FocusChain ───────────────────────────────────────────────────

    #[test]
    fn new_chain_is_empty() {
        let chain = FocusChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert!(chain.current_node().is_none());
    }

    #[test]
    fn default_chain_is_empty() {
        let chain = FocusChain::default();
        assert!(chain.is_empty());
    }

    #[test]
    fn rebuild_from_dom() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let _b = dom.insert_child(root, NodeData::new("B").focusable(true));
        let _c = dom.insert_child(root, NodeData::new("C")); // not focusable

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        assert_eq!(chain.len(), 2);
        assert!(chain.current_node().is_none()); // no focus yet
    }

    #[test]
    fn rebuild_skips_invisible_nodes() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b_id = dom.insert_child(root, NodeData::new("B").focusable(true));

        // Make B invisible.
        dom.get_mut(b_id).unwrap().visible = false;

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn rebuild_skips_disabled_nodes() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let _b = dom.insert_child(
            root,
            NodeData::new("B").focusable(true).disabled(true),
        );

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn focus_next_cycles() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b = dom.insert_child(root, NodeData::new("B").focusable(true));
        let c = dom.insert_child(root, NodeData::new("C").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        // First call: focus first node.
        assert_eq!(chain.focus_next(), Some(a));
        assert_eq!(chain.current_node(), Some(a));

        // Second: focus second.
        assert_eq!(chain.focus_next(), Some(b));
        assert_eq!(chain.current_node(), Some(b));

        // Third: focus third.
        assert_eq!(chain.focus_next(), Some(c));

        // Fourth: wrap around.
        assert_eq!(chain.focus_next(), Some(a));
    }

    #[test]
    fn focus_previous_cycles() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b = dom.insert_child(root, NodeData::new("B").focusable(true));
        let c = dom.insert_child(root, NodeData::new("C").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        // No current focus, previous goes to last.
        assert_eq!(chain.focus_previous(), Some(c));

        // Previous from last goes to middle.
        assert_eq!(chain.focus_previous(), Some(b));

        // Previous from middle goes to first.
        assert_eq!(chain.focus_previous(), Some(a));

        // Previous from first wraps to last.
        assert_eq!(chain.focus_previous(), Some(c));
    }

    #[test]
    fn focus_next_empty_chain() {
        let mut chain = FocusChain::new();
        assert!(chain.focus_next().is_none());
    }

    #[test]
    fn focus_previous_empty_chain() {
        let mut chain = FocusChain::new();
        assert!(chain.focus_previous().is_none());
    }

    #[test]
    fn focus_node_by_id() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b = dom.insert_child(root, NodeData::new("B").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        assert!(chain.focus_node(b));
        assert_eq!(chain.current_node(), Some(b));
    }

    #[test]
    fn focus_node_not_in_chain() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let c = dom.insert_child(root, NodeData::new("C")); // not focusable

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);

        assert!(!chain.focus_node(c));
        assert!(chain.current_node().is_none());
    }

    #[test]
    fn clear_focus() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);
        chain.focus_next();
        assert!(chain.current_node().is_some());

        chain.clear();
        assert!(chain.current_node().is_none());
    }

    #[test]
    fn rebuild_preserves_focus() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b = dom.insert_child(root, NodeData::new("B").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);
        chain.focus_node(b);
        assert_eq!(chain.current_node(), Some(b));

        // Add a new node and rebuild.
        let _c = dom.insert_child(root, NodeData::new("C").focusable(true));
        chain.rebuild(&dom);

        // Focus should still be on b.
        assert_eq!(chain.current_node(), Some(b));
    }

    #[test]
    fn rebuild_clears_focus_if_removed() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let _a = dom.insert_child(root, NodeData::new("A").focusable(true));
        let b = dom.insert_child(root, NodeData::new("B").focusable(true));

        let mut chain = FocusChain::new();
        chain.rebuild(&dom);
        chain.focus_node(b);
        assert_eq!(chain.current_node(), Some(b));

        // Remove b from DOM.
        dom.remove(b);
        chain.rebuild(&dom);

        // Focus should be cleared (b no longer exists).
        assert!(chain.current_node().is_none());
    }

    // ── Screen ───────────────────────────────────────────────────────

    #[test]
    fn new_screen() {
        let screen = Screen::new(80, 24);
        assert!(screen.dom.is_empty());
        assert!(screen.styles.is_empty());
        assert!(screen.css.is_empty());
        assert_eq!(screen.compositor.width, 80);
        assert_eq!(screen.compositor.height, 24);
        assert!(screen.focused_node().is_none());
    }

    #[test]
    fn screen_resize() {
        let mut screen = Screen::new(80, 24);
        screen.resize(120, 40);
        assert_eq!(screen.compositor.width, 120);
        assert_eq!(screen.compositor.height, 40);
        assert!(screen.compositor.is_dirty());
    }

    #[test]
    fn screen_focused_node_delegates_to_focus_chain() {
        let mut screen = Screen::new(80, 24);
        let root = screen.dom.insert(NodeData::new("Root"));
        let a = screen
            .dom
            .insert_child(root, NodeData::new("A").focusable(true));

        screen.focus.rebuild(&screen.dom);
        screen.focus.focus_next();

        assert_eq!(screen.focused_node(), Some(a));
    }
}
