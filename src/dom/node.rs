//! Node types: NodeId, NodeData.

use slotmap::new_key_type;

new_key_type! {
    /// Unique identifier for a DOM node. Copy, lightweight (u64).
    pub struct NodeId;
}

/// Data associated with a single DOM node.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Widget type name (e.g. "Button", "Container").
    pub widget_type: String,
    /// Optional unique id (CSS #id selector).
    pub id: Option<String>,
    /// CSS classes (for .class selector).
    pub classes: Vec<String>,
    /// Whether this node is visible.
    pub visible: bool,
    /// Whether this node can receive focus.
    pub focusable: bool,
    /// Whether this node is disabled.
    pub disabled: bool,
}

impl NodeData {
    /// Create a new `NodeData` with the given widget type and sensible defaults.
    pub fn new(widget_type: impl Into<String>) -> Self {
        Self {
            widget_type: widget_type.into(),
            id: None,
            classes: Vec::new(),
            visible: true,
            focusable: false,
            disabled: false,
        }
    }

    /// Set the CSS id (builder).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a single CSS class (builder).
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        let class = class.into();
        if !self.classes.contains(&class) {
            self.classes.push(class);
        }
        self
    }

    /// Add multiple CSS classes (builder).
    pub fn with_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for class in classes {
            let class = class.into();
            if !self.classes.contains(&class) {
                self.classes.push(class);
            }
        }
        self
    }

    /// Set whether this node can receive focus (builder).
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Set whether this node is disabled (builder).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Check whether this node has a given CSS class.
    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == class)
    }

    /// Add a CSS class. No-op if already present.
    pub fn add_class(&mut self, class: &str) {
        if !self.has_class(class) {
            self.classes.push(class.to_owned());
        }
    }

    /// Remove a CSS class. No-op if not present.
    pub fn remove_class(&mut self, class: &str) {
        self.classes.retain(|c| c != class);
    }

    /// Toggle a CSS class: add if absent, remove if present.
    pub fn toggle_class(&mut self, class: &str) {
        if self.has_class(class) {
            self.remove_class(class);
        } else {
            self.add_class(class);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let data = NodeData::new("Button");
        assert_eq!(data.widget_type, "Button");
        assert!(data.id.is_none());
        assert!(data.classes.is_empty());
        assert!(data.visible);
        assert!(!data.focusable);
        assert!(!data.disabled);
    }

    #[test]
    fn builder_with_id() {
        let data = NodeData::new("Label").with_id("title");
        assert_eq!(data.id.as_deref(), Some("title"));
    }

    #[test]
    fn builder_with_class() {
        let data = NodeData::new("Panel").with_class("primary").with_class("large");
        assert_eq!(data.classes, vec!["primary", "large"]);
    }

    #[test]
    fn builder_with_class_dedup() {
        let data = NodeData::new("Panel").with_class("primary").with_class("primary");
        assert_eq!(data.classes, vec!["primary"]);
    }

    #[test]
    fn builder_with_classes() {
        let data = NodeData::new("Box").with_classes(["a", "b", "c"]);
        assert_eq!(data.classes, vec!["a", "b", "c"]);
    }

    #[test]
    fn builder_with_classes_dedup() {
        let data = NodeData::new("Box").with_class("a").with_classes(["a", "b"]);
        assert_eq!(data.classes, vec!["a", "b"]);
    }

    #[test]
    fn builder_focusable_disabled() {
        let data = NodeData::new("Input").focusable(true).disabled(true);
        assert!(data.focusable);
        assert!(data.disabled);
    }

    #[test]
    fn has_class() {
        let data = NodeData::new("X").with_class("active");
        assert!(data.has_class("active"));
        assert!(!data.has_class("inactive"));
    }

    #[test]
    fn add_class_idempotent() {
        let mut data = NodeData::new("X");
        data.add_class("foo");
        data.add_class("foo");
        assert_eq!(data.classes.len(), 1);
    }

    #[test]
    fn remove_class() {
        let mut data = NodeData::new("X").with_class("a").with_class("b");
        data.remove_class("a");
        assert!(!data.has_class("a"));
        assert!(data.has_class("b"));
    }

    #[test]
    fn remove_class_noop() {
        let mut data = NodeData::new("X");
        data.remove_class("nonexistent"); // should not panic
        assert!(data.classes.is_empty());
    }

    #[test]
    fn toggle_class() {
        let mut data = NodeData::new("X");
        data.toggle_class("active");
        assert!(data.has_class("active"));
        data.toggle_class("active");
        assert!(!data.has_class("active"));
    }

    #[test]
    fn node_id_is_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<NodeId>();
    }
}
