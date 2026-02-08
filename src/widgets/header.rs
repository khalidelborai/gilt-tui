//! Header widget: app header bar with title and optional subtitle.
//!
//! The header renders a title centered on the first row. If a subtitle is
//! provided and the region has at least 2 rows, the subtitle is centered
//! on the second row.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// An app header widget with a centered title and optional subtitle.
///
/// Typically docked to the top of the screen via its default CSS.
///
/// # Examples
///
/// ```ignore
/// let hdr = Header::new("My App").with_subtitle("v1.0");
/// ```
pub struct Header {
    title: String,
    subtitle: Option<String>,
}

impl Header {
    /// Create a new header with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
        }
    }

    /// Set the subtitle (builder pattern).
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Return the title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Return the subtitle, if any.
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }
}

/// Center `text` within `width` characters, returning a String padded with
/// spaces on both sides. Truncates if text is wider than width.
fn center_text(text: &str, width: usize) -> String {
    let truncated: String = text.chars().take(width).collect();
    let text_len = truncated.chars().count();
    if text_len >= width {
        return truncated;
    }
    let pad_left = (width - text_len) / 2;
    let pad_right = width - text_len - pad_left;
    format!(
        "{}{}{}",
        " ".repeat(pad_left),
        truncated,
        " ".repeat(pad_right)
    )
}

impl Widget for Header {
    fn widget_type(&self) -> &str {
        "Header"
    }

    fn default_css(&self) -> &str {
        "Header { height: 1; dock: top; width: 1fr; }"
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        let style = CellStyle::from_styles(styles);
        let width = region.width as usize;
        let mut strips = Vec::new();

        // Title on row 0
        let title_text = center_text(&self.title, width);
        let mut title_strip = Strip::new(region.y, region.x);
        title_strip.push_str(&title_text, style.clone());
        strips.push(title_strip);

        // Subtitle on row 1 (if set and region is tall enough)
        if let Some(ref subtitle) = self.subtitle {
            if region.height >= 2 {
                let sub_text = center_text(subtitle, width);
                let mut sub_strip = Strip::new(region.y + 1, region.x);
                sub_strip.push_str(&sub_text, style.clone());
                strips.push(sub_strip);
            }
        }

        strips
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
    fn widget_type_is_header() {
        let h = Header::new("Title");
        assert_eq!(h.widget_type(), "Header");
    }

    #[test]
    fn default_css_has_dock_top() {
        let h = Header::new("Title");
        assert!(h.default_css().contains("dock: top"));
        assert!(h.default_css().contains("height: 1"));
    }

    #[test]
    fn can_focus_is_false() {
        let h = Header::new("Title");
        assert!(!h.can_focus());
    }

    #[test]
    fn render_title_centered() {
        let h = Header::new("Hi");
        let strips = h.render(region(10, 1), &styles());
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].width(), 10);
        // "Hi" centered in 10: 4 spaces + "Hi" + 4 spaces
        assert_eq!(strips[0].cells[4].ch, 'H');
        assert_eq!(strips[0].cells[5].ch, 'i');
    }

    #[test]
    fn render_with_subtitle() {
        let h = Header::new("Title").with_subtitle("Sub");
        let strips = h.render(region(10, 2), &styles());
        assert_eq!(strips.len(), 2);
        // Subtitle on row 1
        // "Sub" centered in 10: 3 spaces + "Sub" + 4 spaces (or similar)
        let sub_chars: String = strips[1].cells.iter().map(|c| c.ch).collect();
        assert!(sub_chars.contains("Sub"));
    }

    #[test]
    fn render_subtitle_omitted_when_height_1() {
        let h = Header::new("Title").with_subtitle("Sub");
        let strips = h.render(region(10, 1), &styles());
        assert_eq!(strips.len(), 1); // subtitle not rendered
    }

    #[test]
    fn render_zero_region() {
        let h = Header::new("Title");
        assert!(h.render(region(0, 1), &styles()).is_empty());
        assert!(h.render(region(10, 0), &styles()).is_empty());
    }

    #[test]
    fn render_truncates_long_title() {
        let h = Header::new("Very Long Title That Exceeds Width");
        let strips = h.render(region(5, 1), &styles());
        assert_eq!(strips[0].width(), 5);
    }

    #[test]
    fn title_and_subtitle_accessors() {
        let h = Header::new("T").with_subtitle("S");
        assert_eq!(h.title(), "T");
        assert_eq!(h.subtitle(), Some("S"));

        let h2 = Header::new("T");
        assert!(h2.subtitle().is_none());
    }

    #[test]
    fn render_applies_styles() {
        let h = Header::new("Hi");
        let mut s = styles();
        s.background = Some("cyan".into());
        let strips = h.render(region(10, 1), &s);
        assert_eq!(strips[0].cells[0].style.bg, Some("cyan".into()));
    }

    #[test]
    fn as_any_downcast() {
        let h = Header::new("test");
        let any_ref = h.as_any();
        let downcasted = any_ref.downcast_ref::<Header>().unwrap();
        assert_eq!(downcasted.title(), "test");
    }
}
