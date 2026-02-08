//! Snapshot rendering helpers.
//!
//! Functions for converting rendered widget output (strips, compositor screens)
//! into plain-text strings suitable for snapshot testing and assertions.

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::compositor::Compositor;
use crate::render::strip::Strip;
use crate::widget::Widget;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a widget to a plain text string using default (empty) styles.
///
/// The widget is rendered into a region of `width` x `height` cells starting at
/// the origin. Each row becomes one line in the output string, with trailing
/// spaces trimmed. Lines are separated by `'\n'`. The final line does not have
/// a trailing newline.
///
/// # Examples
///
/// ```ignore
/// use gilt_tui::testing::render_to_string;
/// use gilt_tui::widgets::Static;
///
/// let output = render_to_string(&Static::new("Hello"), 20, 1);
/// assert!(output.contains("Hello"));
/// ```
pub fn render_to_string(widget: &dyn Widget, width: i32, height: i32) -> String {
    render_to_styled_string(widget, width, height, &Styles::new())
}

/// Render a widget to a plain text string with custom styles.
///
/// Same as [`render_to_string`] but applies the given styles when rendering.
pub fn render_to_styled_string(
    widget: &dyn Widget,
    width: i32,
    height: i32,
    styles: &Styles,
) -> String {
    let region = Region::new(0, 0, width, height);
    let strips = widget.render(region, styles);
    strips_to_string(&strips, width, height)
}

/// Convert raw strips to a plain text string.
///
/// Builds a `width` x `height` grid of spaces, then overlays each strip's cells
/// at the appropriate (x, y) positions. Each row is right-trimmed of spaces, and
/// rows are joined with `'\n'`.
pub fn strips_to_string(strips: &[Strip], width: i32, height: i32) -> String {
    if width <= 0 || height <= 0 {
        return String::new();
    }

    let w = width as usize;
    let h = height as usize;

    // Initialize a blank grid.
    let mut grid: Vec<Vec<char>> = vec![vec![' '; w]; h];

    // Overlay strips onto the grid.
    for strip in strips {
        let y = strip.y;
        if y < 0 || y >= height {
            continue;
        }
        let row = y as usize;
        for (i, cell) in strip.cells.iter().enumerate() {
            let x = strip.x_offset + i as i32;
            if x < 0 || x >= width {
                continue;
            }
            grid[row][x as usize] = cell.ch;
        }
    }

    // Convert grid to string, trimming trailing spaces per row.
    let lines: Vec<String> = grid
        .into_iter()
        .map(|row| {
            let s: String = row.into_iter().collect();
            s.trim_end().to_owned()
        })
        .collect();

    lines.join("\n")
}

/// Convert a full compositor screen to a plain text string.
///
/// Reads every cell from the compositor's screen buffer and assembles them into
/// rows. Each row is right-trimmed of spaces and rows are joined with `'\n'`.
pub fn compositor_to_string(compositor: &Compositor) -> String {
    let w = compositor.width;
    let h = compositor.height;

    if w == 0 || h == 0 {
        return String::new();
    }

    let mut lines = Vec::with_capacity(h as usize);

    for y in 0..h {
        let mut row = String::with_capacity(w as usize);
        for x in 0..w {
            match compositor.get_cell(x, y) {
                Some(cell) => row.push(cell.ch),
                None => row.push(' '),
            }
        }
        lines.push(row.trim_end().to_owned());
    }

    lines.join("\n")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::strip::{CellStyle, Strip};
    use crate::widgets::{Button, Container, Footer, Header, Static};

    // ── render_to_string ─────────────────────────────────────────────

    #[test]
    fn render_static_to_text() {
        let widget = Static::new("Hello, World!");
        let output = render_to_string(&widget, 20, 1);
        assert!(output.contains("Hello, World!"));
    }

    #[test]
    fn render_static_multiline() {
        let widget = Static::new("Line1\nLine2\nLine3");
        let output = render_to_string(&widget, 10, 3);
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Line1"));
        assert!(lines[1].contains("Line2"));
        assert!(lines[2].contains("Line3"));
    }

    #[test]
    fn render_button_to_text() {
        let widget = Button::new("OK");
        let output = render_to_string(&widget, 10, 3);
        // Button renders 3 rows; label on row 1
        assert!(output.contains("OK"));
    }

    #[test]
    fn render_header_to_text() {
        let widget = Header::new("My App");
        let output = render_to_string(&widget, 20, 1);
        assert!(output.contains("My App"));
    }

    #[test]
    fn render_footer_to_text() {
        let widget = Footer::new("Status: OK");
        let output = render_to_string(&widget, 20, 1);
        assert!(output.contains("Status: OK"));
    }

    #[test]
    fn render_empty_widget() {
        let widget = Static::new("");
        let output = render_to_string(&widget, 10, 1);
        // Empty content renders as a row of spaces, which gets trimmed
        assert!(output.is_empty() || output.chars().all(|c| c == ' ' || c == '\n'));
    }

    #[test]
    fn render_zero_dimensions() {
        let widget = Static::new("Hello");
        let output = render_to_string(&widget, 0, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn render_trims_trailing_spaces() {
        let widget = Static::new("Hi");
        let output = render_to_string(&widget, 20, 1);
        // "Hi" fills to 20 chars in the strip, but trailing spaces are trimmed
        // The static widget fills the full width, so all 20 chars are present but trimmed.
        // Actually, Static fills with spaces to width. "Hi" + 18 spaces. All trimmed.
        // Since Static fills the entire width with the same style, the grid has all
        // the spaces, but they get trimmed. The result should be "Hi" or start with "Hi".
        let first_line = output.lines().next().unwrap_or("");
        assert!(first_line.starts_with("Hi"));
        // No trailing spaces after trimming
        assert!(!first_line.ends_with(' '));
    }

    // ── render_to_styled_string ──────────────────────────────────────

    #[test]
    fn render_with_custom_styles() {
        let widget = Static::new("Styled");
        let mut styles = Styles::new();
        styles.color = Some("red".into());
        // The text content should be the same regardless of color styles
        let output = render_to_styled_string(&widget, 20, 1, &styles);
        assert!(output.contains("Styled"));
    }

    // ── strips_to_string ─────────────────────────────────────────────

    #[test]
    fn strips_to_string_basic() {
        let mut strip = Strip::new(0, 0);
        strip.push_str("ABC", CellStyle::default());
        let output = strips_to_string(&[strip], 10, 1);
        assert!(output.starts_with("ABC"));
    }

    #[test]
    fn strips_to_string_with_offset() {
        let mut strip = Strip::new(0, 5);
        strip.push_str("XY", CellStyle::default());
        let output = strips_to_string(&[strip], 10, 1);
        // 5 spaces + "XY"
        assert_eq!(&output[5..7], "XY");
    }

    #[test]
    fn strips_to_string_multirow() {
        let mut s0 = Strip::new(0, 0);
        s0.push_str("Row0", CellStyle::default());
        let mut s1 = Strip::new(1, 0);
        s1.push_str("Row1", CellStyle::default());
        let output = strips_to_string(&[s0, s1], 10, 2);
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Row0"));
        assert!(lines[1].starts_with("Row1"));
    }

    #[test]
    fn strips_to_string_empty() {
        let output = strips_to_string(&[], 10, 3);
        // 3 rows of blank, all trimmed to empty
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert!(line.is_empty());
        }
    }

    #[test]
    fn strips_to_string_zero_dimensions() {
        let output = strips_to_string(&[], 0, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn strips_to_string_clips_out_of_bounds() {
        // Strip at y=5, but height is only 3 — should be ignored
        let mut strip = Strip::new(5, 0);
        strip.push_str("Ghost", CellStyle::default());
        let output = strips_to_string(&[strip], 10, 3);
        assert!(!output.contains("Ghost"));
    }

    // ── compositor_to_string ─────────────────────────────────────────

    #[test]
    fn compositor_to_string_blank() {
        let compositor = Compositor::new(10, 3);
        let output = compositor_to_string(&compositor);
        // All blank rows, trimmed to empty
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert!(line.is_empty());
        }
    }

    #[test]
    fn compositor_to_string_with_content() {
        let mut compositor = Compositor::new(10, 3);
        let mut strip = Strip::new(0, 0);
        strip.push_str("Hi", CellStyle::default());
        let region = Region::new(0, 0, 10, 3);
        compositor.place_strips(&[strip], &region);
        let output = compositor_to_string(&compositor);
        assert!(output.starts_with("Hi"));
    }

    #[test]
    fn compositor_to_string_zero_size() {
        let compositor = Compositor::new(0, 0);
        let output = compositor_to_string(&compositor);
        assert!(output.is_empty());
    }

    // ── Container default CSS ────────────────────────────────────────

    #[test]
    fn container_default_css_non_empty() {
        let container = Container::new();
        assert!(!container.default_css().is_empty());
    }
}
