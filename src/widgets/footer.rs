//! Footer widget: app footer bar with left-aligned content.
//!
//! The footer renders its content left-aligned on one line, padded to fill
//! the region width. Typically docked to the bottom of the screen.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

/// An app footer widget with left-aligned content.
///
/// Typically docked to the bottom of the screen via its default CSS.
///
/// # Examples
///
/// ```ignore
/// let ft = Footer::new("Press Q to quit");
/// ```
pub struct Footer {
    content: String,
}

impl Footer {
    /// Create a new footer with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Return the footer content.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Widget for Footer {
    fn widget_type(&self) -> &str {
        "Footer"
    }

    fn default_css(&self) -> &str {
        "Footer { height: 1; dock: bottom; width: 1fr; }"
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        let style = CellStyle::from_styles(styles);
        let width = region.width as usize;

        // Render content left-aligned, truncated to width, padded with spaces.
        let truncated: String = self.content.chars().take(width).collect();
        let mut strip = Strip::new(region.y, region.x);
        strip.push_str(&truncated, style.clone());
        strip.fill(region.width, style);

        vec![strip]
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

    fn region(w: i32, h: i32) -> Region {
        Region::new(0, 0, w, h)
    }

    fn styles() -> Styles {
        Styles::new()
    }

    #[test]
    fn widget_type_is_footer() {
        let f = Footer::new("x");
        assert_eq!(f.widget_type(), "Footer");
    }

    #[test]
    fn default_css_has_dock_bottom() {
        let f = Footer::new("x");
        assert!(f.default_css().contains("dock: bottom"));
        assert!(f.default_css().contains("height: 1"));
    }

    #[test]
    fn can_focus_is_false() {
        let f = Footer::new("x");
        assert!(!f.can_focus());
    }

    #[test]
    fn render_left_aligned() {
        let f = Footer::new("Status");
        let strips = f.render(region(20, 1), &styles());
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 20);
        assert_eq!(strips[0].cells[0].ch, 'S');
        assert_eq!(strips[0].cells[5].ch, 's');
        // Remaining cells are spaces
        assert_eq!(strips[0].cells[6].ch, ' ');
    }

    #[test]
    fn render_truncates_long_content() {
        let f = Footer::new("This is a very long footer text");
        let strips = f.render(region(5, 1), &styles());
        assert_eq!(strips[0].width(), 5);
        assert_eq!(strips[0].cells[0].ch, 'T');
        assert_eq!(strips[0].cells[4].ch, ' '); // "This " -> 'T','h','i','s',' '
    }

    #[test]
    fn render_zero_region() {
        let f = Footer::new("x");
        assert!(f.render(region(0, 1), &styles()).is_empty());
        assert!(f.render(region(10, 0), &styles()).is_empty());
    }

    #[test]
    fn render_applies_styles() {
        let f = Footer::new("x");
        let mut s = styles();
        s.background = Some("gray".into());
        let strips = f.render(region(10, 1), &s);
        assert_eq!(strips[0].cells[0].style.bg, Some("gray".into()));
    }

    #[test]
    fn content_accessor() {
        let f = Footer::new("test content");
        assert_eq!(f.content(), "test content");
    }

    #[test]
    fn render_empty_content() {
        let f = Footer::new("");
        let strips = f.render(region(5, 1), &styles());
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 5);
        for cell in &strips[0].cells {
            assert_eq!(cell.ch, ' ');
        }
    }

    #[test]
    fn as_any_downcast() {
        let f = Footer::new("test");
        let any_ref = f.as_any();
        let downcasted = any_ref.downcast_ref::<Footer>().unwrap();
        assert_eq!(downcasted.content(), "test");
    }
}
