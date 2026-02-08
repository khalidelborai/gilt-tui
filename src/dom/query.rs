//! DOM queries: by id, class, type; generic predicate matching.

use super::node::{NodeData, NodeId};
use super::tree::Dom;

impl Dom {
    /// Find the first node whose `id` field matches the given string.
    ///
    /// Iterates all nodes in the arena (not just the tree rooted at `root`).
    pub fn query_by_id(&self, id: &str) -> Option<NodeId> {
        self.iter_nodes()
            .find(|(_, data)| data.id.as_deref() == Some(id))
            .map(|(node_id, _)| node_id)
    }

    /// Find all nodes that have the given CSS class.
    pub fn query_by_class(&self, class: &str) -> Vec<NodeId> {
        self.iter_nodes()
            .filter(|(_, data)| data.has_class(class))
            .map(|(node_id, _)| node_id)
            .collect()
    }

    /// Find all nodes whose `widget_type` matches the given string.
    pub fn query_by_type(&self, widget_type: &str) -> Vec<NodeId> {
        self.iter_nodes()
            .filter(|(_, data)| data.widget_type == widget_type)
            .map(|(node_id, _)| node_id)
            .collect()
    }

    /// Find all nodes matching an arbitrary predicate.
    pub fn query_all(&self, predicate: impl Fn(&NodeData) -> bool) -> Vec<NodeId> {
        self.iter_nodes()
            .filter(|(_, data)| predicate(data))
            .map(|(node_id, _)| node_id)
            .collect()
    }

    /// Iterate over all `(NodeId, &NodeData)` pairs in the arena.
    ///
    /// This is a helper used by the query methods. It iterates in slotmap
    /// insertion order, which is deterministic but not tree-order.
    fn iter_nodes(&self) -> impl Iterator<Item = (NodeId, &NodeData)> {
        self.nodes.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::dom::node::NodeData;
    use crate::dom::tree::Dom;

    /// Build a test tree for queries:
    /// ```text
    ///       root (Container #root)
    ///      /    \
    ///    a       b
    ///  (Panel    (Panel
    ///   #sidebar  #main
    ///   .nav)     .content)
    ///   / \
    ///  c   d
    /// (Button  (Button
    ///  #save    #cancel
    ///  .primary .danger
    ///  .btn)    .btn)
    /// ```
    fn build_query_tree() -> Dom {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Container").with_id("root"));
        let a = dom.insert_child(
            root,
            NodeData::new("Panel")
                .with_id("sidebar")
                .with_class("nav"),
        );
        let _b = dom.insert_child(
            root,
            NodeData::new("Panel")
                .with_id("main")
                .with_class("content"),
        );
        let _c = dom.insert_child(
            a,
            NodeData::new("Button")
                .with_id("save")
                .with_class("primary")
                .with_class("btn"),
        );
        let _d = dom.insert_child(
            a,
            NodeData::new("Button")
                .with_id("cancel")
                .with_class("danger")
                .with_class("btn"),
        );
        dom
    }

    #[test]
    fn query_by_id_found() {
        let dom = build_query_tree();
        let id = dom.query_by_id("sidebar");
        assert!(id.is_some());
        assert_eq!(dom.get(id.unwrap()).unwrap().widget_type, "Panel");
    }

    #[test]
    fn query_by_id_not_found() {
        let dom = build_query_tree();
        assert!(dom.query_by_id("nonexistent").is_none());
    }

    #[test]
    fn query_by_class_single() {
        let dom = build_query_tree();
        let navs = dom.query_by_class("nav");
        assert_eq!(navs.len(), 1);
        assert_eq!(dom.get(navs[0]).unwrap().id.as_deref(), Some("sidebar"));
    }

    #[test]
    fn query_by_class_multiple() {
        let dom = build_query_tree();
        let btns = dom.query_by_class("btn");
        assert_eq!(btns.len(), 2);
    }

    #[test]
    fn query_by_class_empty() {
        let dom = build_query_tree();
        assert!(dom.query_by_class("nonexistent").is_empty());
    }

    #[test]
    fn query_by_type() {
        let dom = build_query_tree();
        let buttons = dom.query_by_type("Button");
        assert_eq!(buttons.len(), 2);
        let panels = dom.query_by_type("Panel");
        assert_eq!(panels.len(), 2);
        let containers = dom.query_by_type("Container");
        assert_eq!(containers.len(), 1);
    }

    #[test]
    fn query_by_type_empty() {
        let dom = build_query_tree();
        assert!(dom.query_by_type("Slider").is_empty());
    }

    #[test]
    fn query_all_custom_predicate() {
        let dom = build_query_tree();
        // Find all nodes that have an id starting with "s".
        let results = dom.query_all(|data| {
            data.id
                .as_ref()
                .is_some_and(|id| id.starts_with('s'))
        });
        // "sidebar" and "save"
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_all_focusable() {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Container"));
        let _a = dom.insert_child(root, NodeData::new("Input").focusable(true));
        let _b = dom.insert_child(root, NodeData::new("Label"));
        let _c = dom.insert_child(root, NodeData::new("Button").focusable(true));

        let focusable = dom.query_all(|data| data.focusable);
        assert_eq!(focusable.len(), 2);
    }

    #[test]
    fn query_on_empty_dom() {
        let dom = Dom::new();
        assert!(dom.query_by_id("x").is_none());
        assert!(dom.query_by_class("x").is_empty());
        assert!(dom.query_by_type("X").is_empty());
        assert!(dom.query_all(|_| true).is_empty());
    }
}
