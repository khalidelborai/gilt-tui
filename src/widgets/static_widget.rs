//! Static widget: displays fixed text content.
//!
//! The simplest widget in gilt-tui. It renders one or more lines of
//! immutable text within the given region, applying CSS-derived styles.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Static
// ---------------------------------------------------------------------------

/// A widget that displays fixed, non-interactive text content.
///
/// Lines are split on `'\n'`. Each line is rendered as one [`Strip`], truncated
/// to the region width and limited to the region height.
///
/// # Examples
///
/// ```ignore
/// let label = Static::new("Hello, world!");
/// ```
pub struct Static {
    content: String,
}

impl Static {
    /// Create a new `Static` widget with the given text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Return the text content.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Widget for Static {
    fn widget_type(&self) -> &str {
        "Static"
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        let style = CellStyle::from_styles(styles);
        let max_width = region.width as usize;
        let max_height = region.height as usize;

        self.content
            .split('\n')
            .take(max_height)
            .enumerate()
            .map(|(i, line)| {
                let mut strip = Strip::new(region.y + i as i32, region.x);
                let truncated: String = line.chars().take(max_width).collect();
                strip.push_str(&truncated, style.clone());
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

    fn region(w: i32, h: i32) -> Region {
        Region::new(0, 0, w, h)
    }

    fn styles() -> Styles {
        Styles::new()
    }

    #[test]
    fn widget_type_is_static() {
        let w = Static::new("hi");
        assert_eq!(w.widget_type(), "Static");
    }

    #[test]
    fn default_css_is_empty() {
        let w = Static::new("hi");
        assert_eq!(w.default_css(), "");
    }

    #[test]
    fn can_focus_is_false() {
        let w = Static::new("hi");
        assert!(!w.can_focus());
    }

    #[test]
    fn children_is_empty() {
        let w = Static::new("hi");
        assert!(w.children().is_empty());
    }

    #[test]
    fn render_single_line() {
        let w = Static::new("Hello");
        let strips = w.render(region(10, 1), &styles());
        assert_eq!(strips.len(), 1);
        // "Hello" + 5 spaces to fill width 10
        assert_eq!(strips[0].width(), 10);
        assert_eq!(strips[0].cells[0].ch, 'H');
        assert_eq!(strips[0].cells[4].ch, 'o');
    }

    #[test]
    fn render_truncates_to_width() {
        let w = Static::new("Hello, world!");
        let strips = w.render(region(5, 1), &styles());
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 5);
        assert_eq!(strips[0].cells[4].ch, 'o');
    }

    #[test]
    fn render_multiline() {
        let w = Static::new("Line1\nLine2\nLine3");
        let strips = w.render(region(10, 5), &styles());
        assert_eq!(strips.len(), 3);
        assert_eq!(strips[0].cells[0].ch, 'L');
        assert_eq!(strips[1].cells[0].ch, 'L');
        assert_eq!(strips[2].cells[0].ch, 'L');
    }

    #[test]
    fn render_limits_to_region_height() {
        let w = Static::new("A\nB\nC\nD\nE");
        let strips = w.render(region(10, 3), &styles());
        assert_eq!(strips.len(), 3);
        assert_eq!(strips[0].cells[0].ch, 'A');
        assert_eq!(strips[2].cells[0].ch, 'C');
    }

    #[test]
    fn render_empty_content() {
        let w = Static::new("");
        let strips = w.render(region(10, 1), &styles());
        // One strip for the single (empty) line, filled with spaces
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 10);
        assert_eq!(strips[0].cells[0].ch, ' ');
    }

    #[test]
    fn render_zero_width_region() {
        let w = Static::new("Hello");
        let strips = w.render(region(0, 5), &styles());
        assert!(strips.is_empty());
    }

    #[test]
    fn render_zero_height_region() {
        let w = Static::new("Hello");
        let strips = w.render(region(10, 0), &styles());
        assert!(strips.is_empty());
    }

    #[test]
    fn render_applies_styles() {
        let w = Static::new("Hi");
        let mut s = styles();
        s.color = Some("red".into());
        let strips = w.render(region(5, 1), &s);
        assert_eq!(strips[0].cells[0].style.fg, Some("red".into()));
    }

    #[test]
    fn render_y_offsets_correct() {
        let w = Static::new("A\nB\nC");
        let r = Region::new(5, 10, 20, 5);
        let strips = w.render(r, &styles());
        assert_eq!(strips[0].y, 10);
        assert_eq!(strips[0].x_offset, 5);
        assert_eq!(strips[1].y, 11);
        assert_eq!(strips[2].y, 12);
    }

    #[test]
    fn content_accessor() {
        let w = Static::new("test content");
        assert_eq!(w.content(), "test content");
    }

    #[test]
    fn as_any_downcast() {
        let w = Static::new("downcast");
        let any_ref = w.as_any();
        let downcasted = any_ref.downcast_ref::<Static>().unwrap();
        assert_eq!(downcasted.content(), "downcast");
    }

    #[test]
    fn as_any_mut_downcast() {
        let mut w = Static::new("original");
        let any_mut = w.as_any_mut();
        let downcasted = any_mut.downcast_mut::<Static>().unwrap();
        assert_eq!(downcasted.content(), "original");
    }
}
