//! Tree operations: insert, remove, reparent, walk.

use std::collections::VecDeque;

use slotmap::{SecondaryMap, SlotMap};

use super::node::{NodeData, NodeId};

/// Empty slice constant for returning when a node has no children.
const EMPTY_CHILDREN: &[NodeId] = &[];

/// The central DOM tree, backed by a slotmap arena.
///
/// All nodes live in a single `SlotMap`. Parent/child relationships are stored
/// in secondary maps so that node removal is O(subtree size) and lookup is O(1).
pub struct Dom {
    pub(crate) nodes: SlotMap<NodeId, NodeData>,
    children: SecondaryMap<NodeId, Vec<NodeId>>,
    parent: SecondaryMap<NodeId, NodeId>,
    root: Option<NodeId>,
}

impl Dom {
    /// Create an empty DOM.
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            children: SecondaryMap::new(),
            parent: SecondaryMap::new(),
            root: None,
        }
    }

    /// Insert a root-level node (no parent).
    ///
    /// If no root has been set yet, this node becomes the root.
    pub fn insert(&mut self, data: NodeData) -> NodeId {
        let id = self.nodes.insert(data);
        self.children.insert(id, Vec::new());
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    /// Insert a node as a child of `parent`.
    ///
    /// # Panics
    ///
    /// Panics (debug) if `parent` does not exist in the tree.
    pub fn insert_child(&mut self, parent: NodeId, data: NodeData) -> NodeId {
        debug_assert!(
            self.nodes.contains_key(parent),
            "parent node does not exist"
        );
        let id = self.nodes.insert(data);
        self.children.insert(id, Vec::new());
        self.parent.insert(id, parent);
        self.children
            .get_mut(parent)
            .expect("parent must have children vec")
            .push(id);
        id
    }

    /// Remove a node and all its descendants recursively.
    ///
    /// Returns the `NodeData` for the removed node, or `None` if it didn't exist.
    pub fn remove(&mut self, id: NodeId) -> Option<NodeData> {
        if !self.nodes.contains_key(id) {
            return None;
        }

        // Detach from parent's children list.
        if let Some(parent_id) = self.parent.remove(id) {
            if let Some(siblings) = self.children.get_mut(parent_id) {
                siblings.retain(|&child| child != id);
            }
        }

        // Clear root if we're removing it.
        if self.root == Some(id) {
            self.root = None;
        }

        // Collect all descendants (BFS) to remove them.
        let mut to_remove = VecDeque::new();
        to_remove.push_back(id);
        let mut removed_root_data = None;

        while let Some(current) = to_remove.pop_front() {
            // Queue children before removing.
            if let Some(kids) = self.children.remove(current) {
                for &child in &kids {
                    to_remove.push_back(child);
                }
            }
            self.parent.remove(current);
            let data = self.nodes.remove(current);
            if current == id {
                removed_root_data = data;
            }
        }

        removed_root_data
    }

    /// Move `node` to become a child of `new_parent`.
    ///
    /// The node keeps its subtree intact. If `node` was previously a child of
    /// another parent, it is detached first.
    ///
    /// # Panics
    ///
    /// Panics (debug) if either `node` or `new_parent` does not exist.
    pub fn reparent(&mut self, node: NodeId, new_parent: NodeId) {
        debug_assert!(self.nodes.contains_key(node), "node does not exist");
        debug_assert!(
            self.nodes.contains_key(new_parent),
            "new_parent does not exist"
        );

        // Detach from old parent.
        if let Some(old_parent) = self.parent.remove(node) {
            if let Some(siblings) = self.children.get_mut(old_parent) {
                siblings.retain(|&child| child != node);
            }
        }

        // Attach to new parent.
        self.parent.insert(node, new_parent);
        self.children
            .get_mut(new_parent)
            .expect("new_parent must have children vec")
            .push(node);
    }

    /// Get the parent of a node, if it has one.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.parent.get(id).copied()
    }

    /// Get the children of a node. Returns an empty slice if the node has no children
    /// or does not exist.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.children
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or(EMPTY_CHILDREN)
    }

    /// Walk from `id` up to the root, collecting ancestor node ids.
    ///
    /// The returned vec does **not** include `id` itself; it starts with the
    /// immediate parent and ends at the root.
    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut current = id;
        while let Some(p) = self.parent.get(current).copied() {
            result.push(p);
            current = p;
        }
        result
    }

    /// Immutable access to a node's data.
    pub fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id)
    }

    /// Mutable access to a node's data.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(id)
    }

    /// The current root node, if set.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Explicitly set the root node.
    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    /// Number of nodes in the DOM.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the DOM is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Whether the DOM contains a node with the given id.
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// Pre-order depth-first traversal starting from `start`.
    pub fn walk_depth_first(&self, start: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut stack = vec![start];
        while let Some(current) = stack.pop() {
            if !self.nodes.contains_key(current) {
                continue;
            }
            result.push(current);
            // Push children in reverse so the first child is visited first.
            let kids = self.children(current);
            for &child in kids.iter().rev() {
                stack.push(child);
            }
        }
        result
    }

    /// Breadth-first traversal starting from `start`.
    pub fn walk_breadth_first(&self, start: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(current) = queue.pop_front() {
            if !self.nodes.contains_key(current) {
                continue;
            }
            result.push(current);
            for &child in self.children(current) {
                queue.push_back(child);
            }
        }
        result
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let root = dom.insert(NodeData::new("Container").with_id("root"));
        let a = dom.insert_child(root, NodeData::new("Panel").with_id("a").with_class("left"));
        let b = dom.insert_child(root, NodeData::new("Panel").with_id("b").with_class("right"));
        let c = dom.insert_child(a, NodeData::new("Button").with_id("c"));
        let d = dom.insert_child(a, NodeData::new("Label").with_id("d"));
        (dom, root, a, b, c, d)
    }

    #[test]
    fn insert_sets_root() {
        let mut dom = Dom::new();
        let id = dom.insert(NodeData::new("Root"));
        assert_eq!(dom.root(), Some(id));
    }

    #[test]
    fn insert_second_does_not_change_root() {
        let mut dom = Dom::new();
        let first = dom.insert(NodeData::new("First"));
        let _second = dom.insert(NodeData::new("Second"));
        assert_eq!(dom.root(), Some(first));
    }

    #[test]
    fn insert_child_parent_relationship() {
        let (dom, root, a, _b, c, _d) = build_tree();
        assert_eq!(dom.parent(a), Some(root));
        assert_eq!(dom.parent(c), Some(a));
        assert_eq!(dom.parent(root), None);
    }

    #[test]
    fn children_list() {
        let (dom, root, a, b, c, d) = build_tree();
        assert_eq!(dom.children(root), &[a, b]);
        assert_eq!(dom.children(a), &[c, d]);
        assert!(dom.children(c).is_empty());
    }

    #[test]
    fn ancestors() {
        let (dom, root, a, _b, c, _d) = build_tree();
        assert_eq!(dom.ancestors(c), vec![a, root]);
        assert_eq!(dom.ancestors(a), vec![root]);
        assert!(dom.ancestors(root).is_empty());
    }

    #[test]
    fn get_and_get_mut() {
        let (mut dom, _root, a, _b, _c, _d) = build_tree();
        assert_eq!(dom.get(a).unwrap().widget_type, "Panel");
        dom.get_mut(a).unwrap().widget_type = "Section".to_string();
        assert_eq!(dom.get(a).unwrap().widget_type, "Section");
    }

    #[test]
    fn len_and_is_empty() {
        let (dom, ..) = build_tree();
        assert_eq!(dom.len(), 5);
        assert!(!dom.is_empty());

        let empty = Dom::new();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn contains() {
        let (dom, _root, a, ..) = build_tree();
        assert!(dom.contains(a));
    }

    #[test]
    fn remove_leaf() {
        let (mut dom, _root, a, _b, c, d) = build_tree();
        let removed = dom.remove(c);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().widget_type, "Button");
        assert!(!dom.contains(c));
        assert_eq!(dom.children(a), &[d]);
        assert_eq!(dom.len(), 4);
    }

    #[test]
    fn remove_subtree() {
        let (mut dom, root, a, b, c, d) = build_tree();
        dom.remove(a);
        assert!(!dom.contains(a));
        assert!(!dom.contains(c));
        assert!(!dom.contains(d));
        assert!(dom.contains(root));
        assert!(dom.contains(b));
        assert_eq!(dom.children(root), &[b]);
        assert_eq!(dom.len(), 2);
    }

    #[test]
    fn remove_root() {
        let (mut dom, root, ..) = build_tree();
        dom.remove(root);
        assert!(dom.is_empty());
        assert_eq!(dom.root(), None);
    }

    #[test]
    fn remove_nonexistent() {
        let mut dom = Dom::new();
        // Create and remove to get a stale id.
        let id = dom.insert(NodeData::new("X"));
        dom.remove(id);
        assert!(dom.remove(id).is_none());
    }

    #[test]
    fn reparent() {
        let (mut dom, root, a, b, c, _d) = build_tree();
        // Move c from under a to under b.
        dom.reparent(c, b);
        assert_eq!(dom.parent(c), Some(b));
        assert!(!dom.children(a).contains(&c));
        assert!(dom.children(b).contains(&c));
        // Ancestors of c should now be [b, root].
        assert_eq!(dom.ancestors(c), vec![b, root]);
    }

    #[test]
    fn set_root() {
        let (mut dom, _root, a, ..) = build_tree();
        dom.set_root(a);
        assert_eq!(dom.root(), Some(a));
    }

    #[test]
    fn walk_depth_first() {
        let (dom, root, a, b, c, d) = build_tree();
        let order = dom.walk_depth_first(root);
        assert_eq!(order, vec![root, a, c, d, b]);
    }

    #[test]
    fn walk_depth_first_subtree() {
        let (dom, _root, a, _b, c, d) = build_tree();
        let order = dom.walk_depth_first(a);
        assert_eq!(order, vec![a, c, d]);
    }

    #[test]
    fn walk_breadth_first() {
        let (dom, root, a, b, c, d) = build_tree();
        let order = dom.walk_breadth_first(root);
        assert_eq!(order, vec![root, a, b, c, d]);
    }

    #[test]
    fn walk_breadth_first_subtree() {
        let (dom, _root, a, _b, c, d) = build_tree();
        let order = dom.walk_breadth_first(a);
        assert_eq!(order, vec![a, c, d]);
    }

    #[test]
    fn default_impl() {
        let dom = Dom::default();
        assert!(dom.is_empty());
        assert_eq!(dom.root(), None);
    }
}
