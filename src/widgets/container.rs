//! Container widget: holds child widgets and arranges them.
//!
//! Container itself renders nothing visible — it just fills its region with
//! background if a background style is set. Its children are accessed through
//! the Container-specific `children_ref()` and `take_children()` methods,
//! since the framework uses these directly during DOM construction.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

/// A layout container that holds child widgets.
///
/// Container does not render content itself. It fills its region with background
/// color if set, and provides access to its children for the layout engine.
///
/// # Examples
///
/// ```ignore
/// use gilt_tui::widgets::{Container, Static, Button};
///
/// let container = Container::new()
///     .with_child(Static::new("Hello"))
///     .with_child(Button::new("Click me"));
/// ```
pub struct Container {
    children: Vec<Box<dyn Widget>>,
    id: Option<String>,
    classes: Vec<String>,
}

impl Container {
    /// Create a new empty container.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            id: None,
            classes: Vec::new(),
        }
    }

    /// Create a container with the "vertical" CSS class.
    ///
    /// This is syntactic sugar — the actual layout direction comes from CSS.
    pub fn vertical() -> Self {
        Self::new().with_class("vertical")
    }

    /// Create a container with the "horizontal" CSS class.
    ///
    /// This is syntactic sugar — the actual layout direction comes from CSS.
    pub fn horizontal() -> Self {
        Self::new().with_class("horizontal")
    }

    /// Add a child widget (builder pattern).
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Set the CSS id (builder pattern).
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_owned());
        self
    }

    /// Add a CSS class (builder pattern).
    pub fn with_class(mut self, class: &str) -> Self {
        let class = class.to_owned();
        if !self.classes.contains(&class) {
            self.classes.push(class);
        }
        self
    }

    /// Borrow the children immutably.
    pub fn children_ref(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    /// Take ownership of the children, leaving the container empty.
    ///
    /// Used by the framework during DOM construction.
    pub fn take_children(&mut self) -> Vec<Box<dyn Widget>> {
        std::mem::take(&mut self.children)
    }

    /// Return the CSS id, if set.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Return the CSS classes.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// The number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Container {
    fn widget_type(&self) -> &str {
        "Container"
    }

    fn default_css(&self) -> &str {
        "Container { layout: vertical; width: 1fr; height: 1fr; }"
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        // Container renders only background fill strips.
        let style = CellStyle::from_styles(styles);
        (0..region.height)
            .map(|row| {
                let mut strip = Strip::new(region.y + row, region.x);
                strip.fill(region.width, style.clone());
                strip
            })
            .collect()
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
    use crate::widgets::static_widget::Static;

    fn region(w: i32, h: i32) -> Region {
        Region::new(0, 0, w, h)
    }

    fn styles() -> Styles {
        Styles::new()
    }

    #[test]
    fn widget_type_is_container() {
        let c = Container::new();
        assert_eq!(c.widget_type(), "Container");
    }

    #[test]
    fn default_css_has_layout() {
        let c = Container::new();
        assert!(c.default_css().contains("layout: vertical"));
        assert!(c.default_css().contains("width: 1fr"));
    }

    #[test]
    fn can_focus_is_false() {
        let c = Container::new();
        assert!(!c.can_focus());
    }

    #[test]
    fn children_trait_returns_empty() {
        // The Widget trait's children() returns empty for Container.
        let c = Container::new().with_child(Static::new("x"));
        assert!(c.children().is_empty());
    }

    #[test]
    fn children_ref_returns_children() {
        let c = Container::new()
            .with_child(Static::new("a"))
            .with_child(Static::new("b"));
        assert_eq!(c.children_ref().len(), 2);
        assert_eq!(c.children_ref()[0].widget_type(), "Static");
        assert_eq!(c.children_ref()[1].widget_type(), "Static");
    }

    #[test]
    fn take_children_empties_container() {
        let mut c = Container::new()
            .with_child(Static::new("a"))
            .with_child(Static::new("b"));
        let kids = c.take_children();
        assert_eq!(kids.len(), 2);
        assert_eq!(c.child_count(), 0);
    }

    #[test]
    fn vertical_has_class() {
        let c = Container::vertical();
        assert!(c.classes().contains(&"vertical".to_owned()));
    }

    #[test]
    fn horizontal_has_class() {
        let c = Container::horizontal();
        assert!(c.classes().contains(&"horizontal".to_owned()));
    }

    #[test]
    fn with_id_sets_id() {
        let c = Container::new().with_id("main");
        assert_eq!(c.id(), Some("main"));
    }

    #[test]
    fn with_class_deduplicates() {
        let c = Container::new()
            .with_class("foo")
            .with_class("foo")
            .with_class("bar");
        assert_eq!(c.classes().len(), 2);
    }

    #[test]
    fn render_fills_background() {
        let c = Container::new();
        let mut s = styles();
        s.background = Some("blue".into());
        let strips = c.render(region(5, 3), &s);
        assert_eq!(strips.len(), 3);
        for strip in &strips {
            assert_eq!(strip.width(), 5);
            for cell in &strip.cells {
                assert_eq!(cell.ch, ' ');
                assert_eq!(cell.style.bg, Some("blue".into()));
            }
        }
    }

    #[test]
    fn render_zero_region() {
        let c = Container::new();
        let strips = c.render(region(0, 0), &styles());
        assert!(strips.is_empty());
    }

    #[test]
    fn render_correct_positions() {
        let c = Container::new();
        let r = Region::new(5, 10, 3, 2);
        let strips = c.render(r, &styles());
        assert_eq!(strips[0].y, 10);
        assert_eq!(strips[0].x_offset, 5);
        assert_eq!(strips[1].y, 11);
    }

    #[test]
    fn child_count() {
        let c = Container::new()
            .with_child(Static::new("a"))
            .with_child(Static::new("b"))
            .with_child(Static::new("c"));
        assert_eq!(c.child_count(), 3);
    }

    #[test]
    fn as_any_downcast() {
        let c = Container::new().with_id("test-id");
        let any_ref = c.as_any();
        let downcasted = any_ref.downcast_ref::<Container>().unwrap();
        assert_eq!(downcasted.id(), Some("test-id"));
    }

    #[test]
    fn default_creates_empty() {
        let c = Container::default();
        assert_eq!(c.child_count(), 0);
        assert!(c.id().is_none());
        assert!(c.classes().is_empty());
    }
}
