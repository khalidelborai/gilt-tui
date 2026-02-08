//! Widget trait: compose, render, mount/unmount.
//!
//! The `Widget` trait is the core abstraction for all UI elements in gilt-tui.
//! Every widget knows its type name, default CSS, and how to render itself into
//! strips within a given region. The `WidgetExt` trait adds builder-style
//! convenience methods for attaching CSS ids and classes.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::Strip;

// ---------------------------------------------------------------------------
// Widget trait
// ---------------------------------------------------------------------------

/// Core trait implemented by all widgets in gilt-tui.
///
/// Widget is designed to be object-safe: the core methods use `&self` and return
/// owned types. Methods that require `Self: Sized` are on the `WidgetExt`
/// extension trait instead.
pub trait Widget {
    /// The CSS type name for this widget (e.g. "Button", "Container").
    ///
    /// Used for CSS type selectors.
    fn widget_type(&self) -> &str;

    /// Default CSS for this widget type. Returns an empty string if none.
    ///
    /// This CSS is applied at the lowest priority (before any user styles).
    fn default_css(&self) -> &str {
        ""
    }

    /// Render this widget's content into strips within the given region.
    ///
    /// The `region` defines the available space in terminal cells. The `styles`
    /// are the fully-resolved CSS styles for this widget (after cascade).
    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip>;

    /// Whether this widget can receive keyboard/mouse focus.
    ///
    /// Defaults to `false`. Override for interactive widgets like buttons and inputs.
    fn can_focus(&self) -> bool {
        false
    }

    /// Compose child widgets. This is the Textual-style "compose" method.
    ///
    /// Returns child widgets that should be mounted as children of this widget
    /// in the DOM. Defaults to an empty vec (leaf widget).
    fn children(&self) -> Vec<Box<dyn Widget>> {
        Vec::new()
    }

    /// Downcast to `&dyn Any` for runtime type inspection.
    fn as_any(&self) -> &dyn Any;

    /// Downcast to `&mut dyn Any` for mutable runtime type inspection.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ---------------------------------------------------------------------------
// WidgetExt
// ---------------------------------------------------------------------------

/// Extension trait providing builder-style convenience methods for widgets.
///
/// Automatically implemented for all types that implement `Widget`.
pub trait WidgetExt: Widget {
    /// Wrap this widget with a CSS id.
    fn with_id(self, id: &str) -> WidgetBuilder<Self>
    where
        Self: Sized,
    {
        WidgetBuilder {
            widget: self,
            id: Some(id.to_owned()),
            classes: Vec::new(),
        }
    }

    /// Wrap this widget with a single CSS class.
    fn with_class(self, class: &str) -> WidgetBuilder<Self>
    where
        Self: Sized,
    {
        WidgetBuilder {
            widget: self,
            id: None,
            classes: vec![class.to_owned()],
        }
    }

    /// Wrap this widget with multiple CSS classes.
    fn with_classes(self, classes: &[&str]) -> WidgetBuilder<Self>
    where
        Self: Sized,
    {
        WidgetBuilder {
            widget: self,
            id: None,
            classes: classes.iter().map(|c| (*c).to_owned()).collect(),
        }
    }
}

// Blanket implementation: every Widget gets WidgetExt for free.
impl<T: Widget> WidgetExt for T {}

// ---------------------------------------------------------------------------
// WidgetBuilder
// ---------------------------------------------------------------------------

/// A wrapper around a widget that adds id and class metadata.
///
/// Created by `WidgetExt::with_id`, `with_class`, or `with_classes`.
/// Delegates all `Widget` methods to the inner widget.
#[derive(Debug)]
pub struct WidgetBuilder<W: Widget> {
    /// The wrapped widget.
    pub widget: W,
    /// Optional CSS id.
    pub id: Option<String>,
    /// CSS classes.
    pub classes: Vec<String>,
}

impl<W: Widget> WidgetBuilder<W> {
    /// Set the CSS id (chainable).
    pub fn set_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_owned());
        self
    }

    /// Add a CSS class (chainable).
    pub fn add_class(mut self, class: &str) -> Self {
        let class = class.to_owned();
        if !self.classes.contains(&class) {
            self.classes.push(class);
        }
        self
    }

    /// Add multiple CSS classes (chainable).
    pub fn add_classes(mut self, classes: &[&str]) -> Self {
        for &class in classes {
            let class = class.to_owned();
            if !self.classes.contains(&class) {
                self.classes.push(class);
            }
        }
        self
    }
}

impl<W: Widget + 'static> Widget for WidgetBuilder<W> {
    fn widget_type(&self) -> &str {
        self.widget.widget_type()
    }

    fn default_css(&self) -> &str {
        self.widget.default_css()
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        self.widget.render(region, styles)
    }

    fn can_focus(&self) -> bool {
        self.widget.can_focus()
    }

    fn children(&self) -> Vec<Box<dyn Widget>> {
        self.widget.children()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::styles::Styles;
    use crate::geometry::Region;
    use crate::render::strip::{CellStyle, Strip};

    // -----------------------------------------------------------------------
    // Test widget
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct TestLabel {
        text: String,
    }

    impl TestLabel {
        fn new(text: &str) -> Self {
            Self {
                text: text.to_owned(),
            }
        }
    }

    impl Widget for TestLabel {
        fn widget_type(&self) -> &str {
            "Label"
        }

        fn default_css(&self) -> &str {
            "Label { color: white; }"
        }

        fn render(&self, region: Region, _styles: &Styles) -> Vec<Strip> {
            if region.width <= 0 || region.height <= 0 {
                return Vec::new();
            }
            let mut strip = Strip::new(region.y, region.x);
            let text: String = self.text.chars().take(region.width as usize).collect();
            strip.push_str(&text, CellStyle::default());
            vec![strip]
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct FocusableWidget;

    impl Widget for FocusableWidget {
        fn widget_type(&self) -> &str {
            "Button"
        }

        fn can_focus(&self) -> bool {
            true
        }

        fn render(&self, _region: Region, _styles: &Styles) -> Vec<Strip> {
            Vec::new()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct ParentWidget;

    impl Widget for ParentWidget {
        fn widget_type(&self) -> &str {
            "Container"
        }

        fn render(&self, _region: Region, _styles: &Styles) -> Vec<Strip> {
            Vec::new()
        }

        fn children(&self) -> Vec<Box<dyn Widget>> {
            vec![
                Box::new(TestLabel::new("child1")),
                Box::new(TestLabel::new("child2")),
            ]
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    // -----------------------------------------------------------------------
    // Widget trait
    // -----------------------------------------------------------------------

    #[test]
    fn widget_type_name() {
        let label = TestLabel::new("hello");
        assert_eq!(label.widget_type(), "Label");
    }

    #[test]
    fn widget_default_css() {
        let label = TestLabel::new("hello");
        assert_eq!(label.default_css(), "Label { color: white; }");
    }

    #[test]
    fn widget_default_css_empty() {
        let btn = FocusableWidget;
        assert_eq!(btn.default_css(), "");
    }

    #[test]
    fn widget_render_produces_strips() {
        let label = TestLabel::new("Hi");
        let region = Region::new(0, 0, 10, 1);
        let styles = Styles::new();
        let strips = label.render(region, &styles);
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 2);
        assert_eq!(strips[0].cells[0].ch, 'H');
        assert_eq!(strips[0].cells[1].ch, 'i');
    }

    #[test]
    fn widget_render_empty_region() {
        let label = TestLabel::new("Hi");
        let region = Region::new(0, 0, 0, 0);
        let styles = Styles::new();
        let strips = label.render(region, &styles);
        assert!(strips.is_empty());
    }

    #[test]
    fn widget_render_truncates_to_width() {
        let label = TestLabel::new("Hello World");
        let region = Region::new(0, 0, 5, 1);
        let styles = Styles::new();
        let strips = label.render(region, &styles);
        assert_eq!(strips[0].width(), 5);
        assert_eq!(strips[0].cells[4].ch, 'o');
    }

    #[test]
    fn widget_can_focus_default_false() {
        let label = TestLabel::new("x");
        assert!(!label.can_focus());
    }

    #[test]
    fn widget_can_focus_overridden() {
        let btn = FocusableWidget;
        assert!(btn.can_focus());
    }

    #[test]
    fn widget_children_default_empty() {
        let label = TestLabel::new("x");
        assert!(label.children().is_empty());
    }

    #[test]
    fn widget_children_compose() {
        let parent = ParentWidget;
        let kids = parent.children();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].widget_type(), "Label");
        assert_eq!(kids[1].widget_type(), "Label");
    }

    #[test]
    fn widget_as_any_downcast() {
        let label = TestLabel::new("test");
        let any_ref = label.as_any();
        let downcasted = any_ref.downcast_ref::<TestLabel>().unwrap();
        assert_eq!(downcasted.text, "test");
    }

    #[test]
    fn widget_as_any_mut_downcast() {
        let mut label = TestLabel::new("test");
        let any_mut = label.as_any_mut();
        let downcasted = any_mut.downcast_mut::<TestLabel>().unwrap();
        downcasted.text = "modified".to_owned();
        assert_eq!(downcasted.text, "modified");
    }

    // -----------------------------------------------------------------------
    // WidgetExt
    // -----------------------------------------------------------------------

    #[test]
    fn widget_ext_with_id() {
        let label = TestLabel::new("hello");
        let built = label.with_id("my-label");
        assert_eq!(built.id, Some("my-label".to_owned()));
        assert!(built.classes.is_empty());
        assert_eq!(built.widget_type(), "Label");
    }

    #[test]
    fn widget_ext_with_class() {
        let label = TestLabel::new("hello");
        let built = label.with_class("primary");
        assert!(built.id.is_none());
        assert_eq!(built.classes, vec!["primary"]);
    }

    #[test]
    fn widget_ext_with_classes() {
        let label = TestLabel::new("hello");
        let built = label.with_classes(&["primary", "large"]);
        assert_eq!(built.classes, vec!["primary", "large"]);
    }

    // -----------------------------------------------------------------------
    // WidgetBuilder
    // -----------------------------------------------------------------------

    #[test]
    fn widget_builder_delegates_widget_type() {
        let built = TestLabel::new("x").with_id("t");
        assert_eq!(built.widget_type(), "Label");
    }

    #[test]
    fn widget_builder_delegates_default_css() {
        let built = TestLabel::new("x").with_id("t");
        assert_eq!(built.default_css(), "Label { color: white; }");
    }

    #[test]
    fn widget_builder_delegates_render() {
        let built = TestLabel::new("AB").with_id("t");
        let region = Region::new(0, 0, 10, 1);
        let styles = Styles::new();
        let strips = built.render(region, &styles);
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 2);
    }

    #[test]
    fn widget_builder_delegates_can_focus() {
        let built = FocusableWidget.with_id("btn");
        assert!(built.can_focus());
    }

    #[test]
    fn widget_builder_delegates_children() {
        let built = ParentWidget.with_id("container");
        assert_eq!(built.children().len(), 2);
    }

    #[test]
    fn widget_builder_chainable() {
        let built = TestLabel::new("x")
            .with_id("t")
            .add_class("primary")
            .add_class("large")
            .add_class("primary"); // duplicate — should not add
        assert_eq!(built.id, Some("t".to_owned()));
        assert_eq!(built.classes, vec!["primary", "large"]);
    }

    #[test]
    fn widget_builder_set_id() {
        let built = TestLabel::new("x")
            .with_class("a")
            .set_id("new-id");
        assert_eq!(built.id, Some("new-id".to_owned()));
        assert_eq!(built.classes, vec!["a"]);
    }

    #[test]
    fn widget_builder_add_classes() {
        let built = TestLabel::new("x")
            .with_id("t")
            .add_classes(&["a", "b", "c"]);
        assert_eq!(built.classes, vec!["a", "b", "c"]);
    }

    #[test]
    fn widget_builder_as_any() {
        let built = TestLabel::new("test").with_id("t");
        let any_ref = built.as_any();
        let downcasted = any_ref.downcast_ref::<WidgetBuilder<TestLabel>>().unwrap();
        assert_eq!(downcasted.widget.text, "test");
        assert_eq!(downcasted.id, Some("t".to_owned()));
    }

    // -----------------------------------------------------------------------
    // Object safety
    // -----------------------------------------------------------------------

    #[test]
    fn widget_is_object_safe() {
        // Verify Widget can be used as dyn Widget (trait object).
        let label: Box<dyn Widget> = Box::new(TestLabel::new("dynamic"));
        assert_eq!(label.widget_type(), "Label");
        assert_eq!(label.render(Region::new(0, 0, 5, 1), &Styles::new()).len(), 1);
    }

    #[test]
    fn widget_builder_is_object_safe() {
        let built: Box<dyn Widget> = Box::new(TestLabel::new("x").with_id("t"));
        assert_eq!(built.widget_type(), "Label");
    }
}
