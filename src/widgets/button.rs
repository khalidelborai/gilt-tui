//! Button widget: an interactive, focusable button.
//!
//! Renders a label centered within its region. Supports a `disabled` state
//! that prevents focus.

use std::any::Any;

use crate::css::styles::{Styles, TextAlign};
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// An interactive button widget with a centered label.
///
/// Buttons can receive focus (unless disabled). The label is rendered centered
/// both horizontally and vertically within the available region.
///
/// # Examples
///
/// ```ignore
/// let btn = Button::new("Submit");
/// let disabled_btn = Button::new("Locked").disabled(true);
/// ```
pub struct Button {
    label: String,
    disabled: bool,
}

impl Button {
    /// Create a new button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    /// Set whether the button is disabled (builder pattern).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Return the button label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the button is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl Widget for Button {
    fn widget_type(&self) -> &str {
        "Button"
    }

    fn default_css(&self) -> &str {
        "Button { height: 3; min-width: 10; text-align: center; }"
    }

    fn can_focus(&self) -> bool {
        !self.disabled
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        let style = CellStyle::from_styles(styles);
        let width = region.width as usize;

        // Determine which row gets the label.
        let label_row = if region.height >= 3 {
            1 // middle row for 3+ height
        } else {
            0 // first row for short buttons
        };

        // Determine text alignment — default to center.
        let text_align = styles.text_align.unwrap_or(TextAlign::Center);

        // Truncate label to fit width.
        let label: String = self.label.chars().take(width).collect();
        let label_len = label.len();

        (0..region.height)
            .map(|row| {
                let mut strip = Strip::new(region.y + row, region.x);
                if row == label_row {
                    let pad_left = match text_align {
                        TextAlign::Left => 0,
                        TextAlign::Center => {
                            if label_len < width {
                                (width - label_len) / 2
                            } else {
                                0
                            }
                        }
                        TextAlign::Right => width.saturating_sub(label_len),
                    };
                    // Left padding
                    for _ in 0..pad_left {
                        strip.push(' ', style.clone());
                    }
                    // Label text
                    strip.push_str(&label, style.clone());
                    // Fill remaining width
                    strip.fill(region.width, style.clone());
                } else {
                    // Empty row — fill with background
                    strip.fill(region.width, style.clone());
                }
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
    fn widget_type_is_button() {
        let b = Button::new("OK");
        assert_eq!(b.widget_type(), "Button");
    }

    #[test]
    fn default_css_has_expected_props() {
        let b = Button::new("OK");
        let css = b.default_css();
        assert!(css.contains("height: 3"));
        assert!(css.contains("min-width: 10"));
        assert!(css.contains("text-align: center"));
    }

    #[test]
    fn can_focus_when_enabled() {
        let b = Button::new("OK");
        assert!(b.can_focus());
    }

    #[test]
    fn cannot_focus_when_disabled() {
        let b = Button::new("OK").disabled(true);
        assert!(!b.can_focus());
    }

    #[test]
    fn label_accessor() {
        let b = Button::new("Submit");
        assert_eq!(b.label(), "Submit");
    }

    #[test]
    fn is_disabled_accessor() {
        let b = Button::new("OK").disabled(true);
        assert!(b.is_disabled());
        let b2 = Button::new("OK");
        assert!(!b2.is_disabled());
    }

    #[test]
    fn render_label_centered_height_3() {
        let b = Button::new("OK");
        let strips = b.render(region(10, 3), &styles());
        assert_eq!(strips.len(), 3);
        // Label on row 1 (middle), centered: "    OK    "
        let row1 = &strips[1];
        assert_eq!(row1.width(), 10);
        // Find the 'O' — it should be at position 4 (10-2=8, 8/2=4)
        assert_eq!(row1.cells[4].ch, 'O');
        assert_eq!(row1.cells[5].ch, 'K');
    }

    #[test]
    fn render_label_on_first_row_when_short() {
        let b = Button::new("OK");
        let strips = b.render(region(10, 1), &styles());
        assert_eq!(strips.len(), 1);
        // Label on row 0, centered
        assert_eq!(strips[0].cells[4].ch, 'O');
        assert_eq!(strips[0].cells[5].ch, 'K');
    }

    #[test]
    fn render_label_on_first_row_height_2() {
        let b = Button::new("OK");
        let strips = b.render(region(10, 2), &styles());
        assert_eq!(strips.len(), 2);
        // Height < 3, label on row 0
        assert_eq!(strips[0].cells[4].ch, 'O');
    }

    #[test]
    fn render_truncates_long_label() {
        let b = Button::new("Very Long Label Text");
        let strips = b.render(region(5, 3), &styles());
        assert_eq!(strips[1].width(), 5);
        // "Very " truncated to 5 chars — label centered: "Very " fits exactly
        assert_eq!(strips[1].cells[0].ch, 'V');
    }

    #[test]
    fn render_zero_region() {
        let b = Button::new("OK");
        assert!(b.render(region(0, 3), &styles()).is_empty());
        assert!(b.render(region(10, 0), &styles()).is_empty());
    }

    #[test]
    fn render_fills_empty_rows() {
        let b = Button::new("X");
        let strips = b.render(region(5, 3), &styles());
        // Row 0 and 2 should be all spaces
        for cell in &strips[0].cells {
            assert_eq!(cell.ch, ' ');
        }
        for cell in &strips[2].cells {
            assert_eq!(cell.ch, ' ');
        }
    }

    #[test]
    fn render_applies_styles() {
        let b = Button::new("OK");
        let mut s = styles();
        s.color = Some("green".into());
        let strips = b.render(region(10, 3), &s);
        assert_eq!(strips[1].cells[4].style.fg, Some("green".into()));
    }

    #[test]
    fn render_positions_correct() {
        let b = Button::new("X");
        let r = Region::new(5, 10, 8, 3);
        let strips = b.render(r, &styles());
        assert_eq!(strips[0].y, 10);
        assert_eq!(strips[0].x_offset, 5);
        assert_eq!(strips[1].y, 11);
        assert_eq!(strips[2].y, 12);
    }

    #[test]
    fn as_any_downcast() {
        let b = Button::new("test");
        let any_ref = b.as_any();
        let downcasted = any_ref.downcast_ref::<Button>().unwrap();
        assert_eq!(downcasted.label(), "test");
    }
}
