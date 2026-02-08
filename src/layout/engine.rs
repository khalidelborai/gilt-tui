//! TaffyTree wrapper for layout computation.
//!
//! [`LayoutEngine`] synchronizes the DOM tree to a taffy layout tree,
//! runs layout computation, and provides results as [`Region`]s.

use std::collections::HashMap;

use taffy::prelude::*;

use crate::css::styles::Styles;
use crate::dom::node::NodeId;
use crate::dom::tree::Dom;
use crate::geometry::Region;

use super::resolve::resolve_styles;

/// Wraps a [`TaffyTree`] and maintains a mapping from DOM [`NodeId`]s to
/// taffy node ids. Provides methods to sync, compute, and query layout.
pub struct LayoutEngine {
    /// The taffy tree, parameterized with our DOM NodeId as context data.
    tree: TaffyTree<NodeId>,
    /// Maps DOM NodeId -> taffy NodeId for quick lookup.
    node_map: HashMap<NodeId, taffy::prelude::NodeId>,
    /// The taffy root node, if a layout has been synced.
    root: Option<taffy::prelude::NodeId>,
}

impl LayoutEngine {
    /// Create a new, empty layout engine.
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            node_map: HashMap::new(),
            root: None,
        }
    }

    /// Synchronize the taffy tree with the DOM structure.
    ///
    /// Walks the DOM depth-first from the root, creating or updating taffy nodes
    /// to match. Stale taffy nodes (those whose DOM NodeId no longer exists) are
    /// removed. The taffy tree's parent/child relationships are rebuilt to mirror
    /// the DOM tree.
    ///
    /// `styles` maps each DOM NodeId to its resolved [`Styles`]. Nodes without
    /// an entry get `Styles::default()`.
    pub fn sync_tree(
        &mut self,
        dom: &Dom,
        styles: &HashMap<NodeId, Styles>,
        viewport: (u16, u16),
    ) {
        let dom_root = match dom.root() {
            Some(r) => r,
            None => {
                // Empty DOM: clear everything.
                self.clear();
                return;
            }
        };

        // Walk DOM depth-first to collect the set of live node ids.
        let live_nodes = dom.walk_depth_first(dom_root);
        let live_set: std::collections::HashSet<NodeId> = live_nodes.iter().copied().collect();

        // Remove stale taffy nodes (DOM nodes that no longer exist).
        let stale_keys: Vec<NodeId> = self
            .node_map
            .keys()
            .filter(|k| !live_set.contains(k))
            .copied()
            .collect();
        for key in stale_keys {
            if let Some(taffy_id) = self.node_map.remove(&key) {
                let _ = self.tree.remove(taffy_id);
            }
        }

        // Create or update taffy nodes for all live DOM nodes.
        for &dom_id in &live_nodes {
            let node_styles = styles.get(&dom_id).cloned().unwrap_or_default();
            let taffy_style = resolve_styles(&node_styles, viewport);

            if let Some(&taffy_id) = self.node_map.get(&dom_id) {
                // Update existing node's style.
                let _ = self.tree.set_style(taffy_id, taffy_style);
            } else {
                // Create new taffy node.
                let taffy_id = self
                    .tree
                    .new_leaf_with_context(taffy_style, dom_id)
                    .expect("taffy node creation should not fail");
                self.node_map.insert(dom_id, taffy_id);
            }
        }

        // Rebuild parent-child relationships in taffy to match DOM.
        for &dom_id in &live_nodes {
            let dom_children = dom.children(dom_id);
            let taffy_children: Vec<taffy::prelude::NodeId> = dom_children
                .iter()
                .filter_map(|&child_id| self.node_map.get(&child_id).copied())
                .collect();

            if let Some(&taffy_id) = self.node_map.get(&dom_id) {
                let _ = self.tree.set_children(taffy_id, &taffy_children);
            }
        }

        // Set the taffy root.
        self.root = self.node_map.get(&dom_root).copied();
    }

    /// Run taffy layout computation on the root node.
    ///
    /// `available_width` and `available_height` define the available space,
    /// typically the terminal size in cells.
    pub fn compute(&mut self, available_width: f32, available_height: f32) {
        if let Some(root) = self.root {
            let _ = self.tree.compute_layout(
                root,
                taffy::geometry::Size {
                    width: AvailableSpace::Definite(available_width),
                    height: AvailableSpace::Definite(available_height),
                },
            );
        }
    }

    /// Get the layout result for a single DOM node as a [`Region`].
    ///
    /// Returns `None` if the node is not in the layout tree.
    /// Taffy's f32 coordinates are rounded to the nearest integer cell.
    pub fn get_layout(&self, node: NodeId) -> Option<Region> {
        let taffy_id = self.node_map.get(&node)?;
        let layout = self.tree.layout(*taffy_id).ok()?;
        Some(Region {
            x: layout.location.x.round() as i32,
            y: layout.location.y.round() as i32,
            width: layout.size.width.round() as i32,
            height: layout.size.height.round() as i32,
        })
    }

    /// Get layout results for all nodes as a map of DOM NodeId -> [`Region`].
    pub fn get_all_layouts(&self) -> HashMap<NodeId, Region> {
        let mut result = HashMap::new();
        for (&dom_id, &taffy_id) in &self.node_map {
            if let Ok(layout) = self.tree.layout(taffy_id) {
                result.insert(
                    dom_id,
                    Region {
                        x: layout.location.x.round() as i32,
                        y: layout.location.y.round() as i32,
                        width: layout.size.width.round() as i32,
                        height: layout.size.height.round() as i32,
                    },
                );
            }
        }
        result
    }

    /// Clear all state, removing all taffy nodes and mappings.
    fn clear(&mut self) {
        // Remove all nodes from taffy.
        let keys: Vec<_> = self.node_map.drain().map(|(_, v)| v).collect();
        for taffy_id in keys {
            let _ = self.tree.remove(taffy_id);
        }
        self.root = None;
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::scalar::{Scalar, ScalarBox};
    use crate::css::styles::{Border, BorderKind, Dock, LayoutDirection, Styles};
    use crate::dom::node::NodeData;
    use crate::dom::tree::Dom;

    const VP: (u16, u16) = (80, 24);

    /// Helper: build a simple DOM with root and two children.
    fn simple_dom() -> (Dom, NodeId, NodeId, NodeId) {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let a = dom.insert_child(root, NodeData::new("A"));
        let b = dom.insert_child(root, NodeData::new("B"));
        (dom, root, a, b)
    }

    #[test]
    fn new_engine_is_empty() {
        let engine = LayoutEngine::new();
        assert!(engine.node_map.is_empty());
        assert!(engine.root.is_none());
    }

    #[test]
    fn default_engine_is_empty() {
        let engine = LayoutEngine::default();
        assert!(engine.node_map.is_empty());
    }

    #[test]
    fn sync_empty_dom() {
        let dom = Dom::new();
        let styles = HashMap::new();
        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        assert!(engine.root.is_none());
        assert!(engine.node_map.is_empty());
    }

    #[test]
    fn sync_single_node() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let styles = HashMap::new();
        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        assert!(engine.root.is_some());
        assert!(engine.node_map.contains_key(&root));
    }

    #[test]
    fn sync_with_children() {
        let (dom, root, a, b) = simple_dom();
        let styles = HashMap::new();
        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);

        assert!(engine.node_map.contains_key(&root));
        assert!(engine.node_map.contains_key(&a));
        assert!(engine.node_map.contains_key(&b));

        // Verify taffy tree has the right children for root.
        let taffy_root = engine.node_map[&root];
        let children = engine.tree.children(taffy_root).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn compute_simple_layout() {
        let (dom, root, a, b) = simple_dom();
        let mut styles = HashMap::new();
        // Root takes full space.
        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        // Children have fixed heights.
        let mut a_style = Styles::new();
        a_style.height = Some(Scalar::cells(10.0));
        styles.insert(a, a_style);

        let mut b_style = Styles::new();
        b_style.height = Some(Scalar::cells(14.0));
        styles.insert(b, b_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let root_layout = engine.get_layout(root).unwrap();
        assert_eq!(root_layout.width, 80);
        assert_eq!(root_layout.height, 24);
        assert_eq!(root_layout.x, 0);
        assert_eq!(root_layout.y, 0);

        let a_layout = engine.get_layout(a).unwrap();
        assert_eq!(a_layout.height, 10);
        assert_eq!(a_layout.y, 0);

        let b_layout = engine.get_layout(b).unwrap();
        assert_eq!(b_layout.height, 14);
        assert_eq!(b_layout.y, 10);
    }

    #[test]
    fn horizontal_layout() {
        let (dom, root, a, b) = simple_dom();
        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.layout = Some(LayoutDirection::Horizontal);
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut a_style = Styles::new();
        a_style.width = Some(Scalar::cells(30.0));
        styles.insert(a, a_style);

        let mut b_style = Styles::new();
        b_style.width = Some(Scalar::cells(50.0));
        styles.insert(b, b_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let a_layout = engine.get_layout(a).unwrap();
        assert_eq!(a_layout.x, 0);
        assert_eq!(a_layout.width, 30);

        let b_layout = engine.get_layout(b).unwrap();
        assert_eq!(b_layout.x, 30);
        assert_eq!(b_layout.width, 50);
    }

    #[test]
    fn get_all_layouts() {
        let (dom, root, a, b) = simple_dom();
        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let all = engine.get_all_layouts();
        assert!(all.contains_key(&root));
        assert!(all.contains_key(&a));
        assert!(all.contains_key(&b));
    }

    #[test]
    fn get_layout_nonexistent() {
        let engine = LayoutEngine::new();
        // Create a NodeId that doesn't exist in the engine.
        let mut dom = Dom::new();
        let id = dom.insert(NodeData::new("X"));
        assert!(engine.get_layout(id).is_none());
    }

    #[test]
    fn sync_removes_stale_nodes() {
        let (mut dom, root, a, b) = simple_dom();
        let styles = HashMap::new();
        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);

        assert_eq!(engine.node_map.len(), 3);

        // Remove node 'b' from DOM.
        dom.remove(b);
        engine.sync_tree(&dom, &styles, VP);

        assert_eq!(engine.node_map.len(), 2);
        assert!(!engine.node_map.contains_key(&b));
        assert!(engine.node_map.contains_key(&root));
        assert!(engine.node_map.contains_key(&a));
    }

    #[test]
    fn resync_updates_styles() {
        let (dom, root, a, _b) = simple_dom();
        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut a_style = Styles::new();
        a_style.height = Some(Scalar::cells(5.0));
        styles.insert(a, a_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let a_layout = engine.get_layout(a).unwrap();
        assert_eq!(a_layout.height, 5);

        // Change A's height.
        let mut a_style2 = Styles::new();
        a_style2.height = Some(Scalar::cells(12.0));
        styles.insert(a, a_style2);

        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let a_layout2 = engine.get_layout(a).unwrap();
        assert_eq!(a_layout2.height, 12);
    }

    #[test]
    fn layout_with_padding() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let child = dom.insert_child(root, NodeData::new("Child"));

        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        root_style.padding = Some(ScalarBox::all(Scalar::cells(2.0)));
        styles.insert(root, root_style);

        let mut child_style = Styles::new();
        child_style.width = Some(Scalar::cells(20.0));
        child_style.height = Some(Scalar::cells(5.0));
        styles.insert(child, child_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let child_layout = engine.get_layout(child).unwrap();
        // Child should be offset by the padding.
        assert_eq!(child_layout.x, 2);
        assert_eq!(child_layout.y, 2);
    }

    #[test]
    fn layout_with_border() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let child = dom.insert_child(root, NodeData::new("Child"));

        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        root_style.border = Some(Border {
            kind: BorderKind::Thin,
            color: None,
        });
        styles.insert(root, root_style);

        let mut child_style = Styles::new();
        child_style.width = Some(Scalar::cells(10.0));
        child_style.height = Some(Scalar::cells(5.0));
        styles.insert(child, child_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let child_layout = engine.get_layout(child).unwrap();
        // Child should be offset by border (1 cell each side).
        assert_eq!(child_layout.x, 1);
        assert_eq!(child_layout.y, 1);
    }

    #[test]
    fn deep_tree_layout() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let a = dom.insert_child(root, NodeData::new("A"));
        let b = dom.insert_child(a, NodeData::new("B"));
        let c = dom.insert_child(b, NodeData::new("C"));

        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut c_style = Styles::new();
        c_style.width = Some(Scalar::cells(10.0));
        c_style.height = Some(Scalar::cells(3.0));
        styles.insert(c, c_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let c_layout = engine.get_layout(c).unwrap();
        assert_eq!(c_layout.width, 10);
        assert_eq!(c_layout.height, 3);
    }

    #[test]
    fn sync_then_clear_via_empty_dom() {
        let (dom, _root, _a, _b) = simple_dom();
        let styles = HashMap::new();
        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        assert_eq!(engine.node_map.len(), 3);

        // Sync with empty DOM.
        let empty_dom = Dom::new();
        engine.sync_tree(&empty_dom, &styles, VP);
        assert!(engine.node_map.is_empty());
        assert!(engine.root.is_none());
    }

    #[test]
    fn dock_top_layout() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let docked = dom.insert_child(root, NodeData::new("Docked"));

        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut docked_style = Styles::new();
        docked_style.dock = Some(Dock::Top);
        docked_style.height = Some(Scalar::cells(3.0));
        styles.insert(docked, docked_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let docked_layout = engine.get_layout(docked).unwrap();
        assert_eq!(docked_layout.y, 0);
        assert_eq!(docked_layout.height, 3);
    }

    #[test]
    fn display_none_zero_size() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Root"));
        let hidden = dom.insert_child(root, NodeData::new("Hidden"));

        let mut styles = HashMap::new();

        let mut root_style = Styles::new();
        root_style.width = Some(Scalar::cells(80.0));
        root_style.height = Some(Scalar::cells(24.0));
        styles.insert(root, root_style);

        let mut hidden_style = Styles::new();
        hidden_style.display = Some(crate::css::styles::Display::None);
        hidden_style.width = Some(Scalar::cells(50.0));
        hidden_style.height = Some(Scalar::cells(10.0));
        styles.insert(hidden, hidden_style);

        let mut engine = LayoutEngine::new();
        engine.sync_tree(&dom, &styles, VP);
        engine.compute(80.0, 24.0);

        let hidden_layout = engine.get_layout(hidden).unwrap();
        assert_eq!(hidden_layout.width, 0);
        assert_eq!(hidden_layout.height, 0);
    }
}
