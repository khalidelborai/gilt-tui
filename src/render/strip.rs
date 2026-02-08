//! Strip: a horizontal line of styled terminal cells.
//!
//! A `Strip` is the fundamental rendering primitive in gilt-tui. It represents
//! a single horizontal row of `StyledCell`s that can be placed into the compositor's
//! screen buffer. Widgets produce `Vec<Strip>` from their `render()` method.

use crate::css::styles::Styles;

// ---------------------------------------------------------------------------
// CellStyle
// ---------------------------------------------------------------------------

/// Visual style for a single terminal cell.
///
/// This is a self-contained style type (no gilt dependency) used throughout
/// the rendering pipeline. Colors are stored as optional strings that can be
/// parsed as named colors or `#rrggbb` hex values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub reverse: bool,
}

impl CellStyle {
    /// Create a new `CellStyle` with all attributes unset/false.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert CSS `Styles` into a `CellStyle`, extracting color, background,
    /// and text_style flags.
    pub fn from_styles(styles: &Styles) -> Self {
        let flags = styles.text_style.unwrap_or_default();
        CellStyle {
            fg: styles.color.clone(),
            bg: styles.background.clone(),
            bold: flags.bold.unwrap_or(false),
            dim: flags.dim.unwrap_or(false),
            italic: flags.italic.unwrap_or(false),
            underline: flags.underline.unwrap_or(false),
            strikethrough: flags.strikethrough.unwrap_or(false),
            reverse: flags.reverse.unwrap_or(false),
        }
    }
}

// ---------------------------------------------------------------------------
// StyledCell
// ---------------------------------------------------------------------------

/// A single terminal cell: one character with associated style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledCell {
    pub ch: char,
    pub style: CellStyle,
}

impl StyledCell {
    /// Create a new styled cell.
    pub fn new(ch: char, style: CellStyle) -> Self {
        Self { ch, style }
    }

    /// A blank (space) cell with default style.
    pub fn blank() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }

    /// A blank (space) cell with the given style.
    pub fn blank_styled(style: CellStyle) -> Self {
        Self { ch: ' ', style }
    }
}

impl Default for StyledCell {
    fn default() -> Self {
        Self::blank()
    }
}

// ---------------------------------------------------------------------------
// Strip
// ---------------------------------------------------------------------------

/// A horizontal line of styled terminal cells.
///
/// Each Strip represents one row (at a given y position) starting at `x_offset`.
/// Widgets produce strips; the compositor places them into the screen buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strip {
    /// The row this strip occupies (0-based from top of region).
    pub y: i32,
    /// Starting x position for this strip's cells.
    pub x_offset: i32,
    /// The cells in left-to-right order.
    pub cells: Vec<StyledCell>,
}

impl Strip {
    /// Create a new empty strip at the given row and x offset.
    pub fn new(y: i32, x_offset: i32) -> Self {
        Self {
            y,
            x_offset,
            cells: Vec::new(),
        }
    }

    /// Push a single character with the given style.
    pub fn push(&mut self, ch: char, style: CellStyle) {
        self.cells.push(StyledCell::new(ch, style));
    }

    /// Push every character of `text` with the same style.
    pub fn push_str(&mut self, text: &str, style: CellStyle) {
        for ch in text.chars() {
            self.cells.push(StyledCell::new(ch, style.clone()));
        }
    }

    /// The width of this strip in cells.
    pub fn width(&self) -> i32 {
        self.cells.len() as i32
    }

    /// Crop the strip to only include cells whose x positions fall within
    /// `[x_start, x_end)` (absolute positions). Returns a new Strip.
    ///
    /// Cells outside the range are discarded. The returned strip's `x_offset`
    /// is adjusted to `x_start` (or the first cell's position if later).
    pub fn crop(&self, x_start: i32, x_end: i32) -> Strip {
        let mut result = Strip::new(self.y, x_start);
        for (i, cell) in self.cells.iter().enumerate() {
            let cell_x = self.x_offset + i as i32;
            if cell_x >= x_start && cell_x < x_end {
                if result.cells.is_empty() {
                    result.x_offset = cell_x;
                }
                result.cells.push(cell.clone());
            }
        }
        result
    }

    /// Pad the strip to exactly `width` cells using spaces with the given style.
    ///
    /// If the strip is already wider than `width`, it is truncated.
    pub fn fill(&mut self, width: i32, style: CellStyle) {
        let w = width as usize;
        if self.cells.len() < w {
            self.cells
                .resize(w, StyledCell::blank_styled(style));
        } else if self.cells.len() > w {
            self.cells.truncate(w);
        }
    }

    /// The rightmost x position (exclusive) of this strip.
    pub fn right(&self) -> i32 {
        self.x_offset + self.width()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::styles::{Styles, TextStyleFlags};

    fn red_style() -> CellStyle {
        CellStyle {
            fg: Some("red".into()),
            ..CellStyle::default()
        }
    }

    fn blue_bg_bold() -> CellStyle {
        CellStyle {
            bg: Some("blue".into()),
            bold: true,
            ..CellStyle::default()
        }
    }

    // -----------------------------------------------------------------------
    // CellStyle
    // -----------------------------------------------------------------------

    #[test]
    fn cell_style_default_is_empty() {
        let s = CellStyle::default();
        assert!(s.fg.is_none());
        assert!(s.bg.is_none());
        assert!(!s.bold);
        assert!(!s.dim);
        assert!(!s.italic);
        assert!(!s.underline);
        assert!(!s.strikethrough);
        assert!(!s.reverse);
    }

    #[test]
    fn cell_style_new_is_default() {
        assert_eq!(CellStyle::new(), CellStyle::default());
    }

    #[test]
    fn cell_style_from_styles_empty() {
        let styles = Styles::new();
        let cs = CellStyle::from_styles(&styles);
        assert_eq!(cs, CellStyle::default());
    }

    #[test]
    fn cell_style_from_styles_colors() {
        let mut styles = Styles::new();
        styles.color = Some("red".into());
        styles.background = Some("#ff00ff".into());
        let cs = CellStyle::from_styles(&styles);
        assert_eq!(cs.fg, Some("red".into()));
        assert_eq!(cs.bg, Some("#ff00ff".into()));
    }

    #[test]
    fn cell_style_from_styles_text_flags() {
        let mut styles = Styles::new();
        styles.text_style = Some(TextStyleFlags {
            bold: Some(true),
            italic: Some(true),
            dim: Some(false),
            underline: None,
            strikethrough: Some(true),
            reverse: None,
        });
        let cs = CellStyle::from_styles(&styles);
        assert!(cs.bold);
        assert!(cs.italic);
        assert!(!cs.dim);
        assert!(!cs.underline);
        assert!(cs.strikethrough);
        assert!(!cs.reverse);
    }

    // -----------------------------------------------------------------------
    // StyledCell
    // -----------------------------------------------------------------------

    #[test]
    fn styled_cell_new() {
        let cell = StyledCell::new('A', red_style());
        assert_eq!(cell.ch, 'A');
        assert_eq!(cell.style.fg, Some("red".into()));
    }

    #[test]
    fn styled_cell_blank() {
        let cell = StyledCell::blank();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.style, CellStyle::default());
    }

    #[test]
    fn styled_cell_blank_styled() {
        let style = blue_bg_bold();
        let cell = StyledCell::blank_styled(style.clone());
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.style, style);
    }

    #[test]
    fn styled_cell_default_is_blank() {
        assert_eq!(StyledCell::default(), StyledCell::blank());
    }

    // -----------------------------------------------------------------------
    // Strip — construction
    // -----------------------------------------------------------------------

    #[test]
    fn strip_new_empty() {
        let s = Strip::new(5, 0);
        assert_eq!(s.y, 5);
        assert_eq!(s.x_offset, 0);
        assert!(s.cells.is_empty());
        assert_eq!(s.width(), 0);
    }

    #[test]
    fn strip_push_single_char() {
        let mut s = Strip::new(0, 0);
        s.push('X', red_style());
        assert_eq!(s.width(), 1);
        assert_eq!(s.cells[0].ch, 'X');
        assert_eq!(s.cells[0].style, red_style());
    }

    #[test]
    fn strip_push_str() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hello", red_style());
        assert_eq!(s.width(), 5);
        assert_eq!(s.cells[0].ch, 'H');
        assert_eq!(s.cells[4].ch, 'o');
        for cell in &s.cells {
            assert_eq!(cell.style, red_style());
        }
    }

    #[test]
    fn strip_push_str_empty() {
        let mut s = Strip::new(0, 0);
        s.push_str("", red_style());
        assert_eq!(s.width(), 0);
    }

    #[test]
    fn strip_right() {
        let mut s = Strip::new(0, 10);
        s.push_str("abc", CellStyle::default());
        assert_eq!(s.x_offset, 10);
        assert_eq!(s.width(), 3);
        assert_eq!(s.right(), 13);
    }

    // -----------------------------------------------------------------------
    // Strip — crop
    // -----------------------------------------------------------------------

    #[test]
    fn strip_crop_full_range() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hello", red_style());
        let cropped = s.crop(0, 5);
        assert_eq!(cropped.width(), 5);
        assert_eq!(cropped.x_offset, 0);
    }

    #[test]
    fn strip_crop_subset() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hello", red_style());
        let cropped = s.crop(1, 4);
        assert_eq!(cropped.width(), 3);
        assert_eq!(cropped.x_offset, 1);
        assert_eq!(cropped.cells[0].ch, 'e');
        assert_eq!(cropped.cells[1].ch, 'l');
        assert_eq!(cropped.cells[2].ch, 'l');
    }

    #[test]
    fn strip_crop_with_offset() {
        let mut s = Strip::new(0, 5);
        s.push_str("World", red_style());
        // Cells are at positions 5, 6, 7, 8, 9
        let cropped = s.crop(6, 9);
        assert_eq!(cropped.width(), 3);
        assert_eq!(cropped.x_offset, 6);
        assert_eq!(cropped.cells[0].ch, 'o');
        assert_eq!(cropped.cells[1].ch, 'r');
        assert_eq!(cropped.cells[2].ch, 'l');
    }

    #[test]
    fn strip_crop_no_overlap() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hello", red_style());
        let cropped = s.crop(10, 20);
        assert_eq!(cropped.width(), 0);
    }

    #[test]
    fn strip_crop_partial_left() {
        let mut s = Strip::new(0, 3);
        s.push_str("abc", red_style());
        // Cells at 3, 4, 5; crop [0, 4) => only cell at 3
        let cropped = s.crop(0, 4);
        assert_eq!(cropped.width(), 1);
        assert_eq!(cropped.x_offset, 3);
        assert_eq!(cropped.cells[0].ch, 'a');
    }

    // -----------------------------------------------------------------------
    // Strip — fill
    // -----------------------------------------------------------------------

    #[test]
    fn strip_fill_pad() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hi", red_style());
        s.fill(5, CellStyle::default());
        assert_eq!(s.width(), 5);
        assert_eq!(s.cells[0].ch, 'H');
        assert_eq!(s.cells[1].ch, 'i');
        assert_eq!(s.cells[2].ch, ' ');
        assert_eq!(s.cells[3].ch, ' ');
        assert_eq!(s.cells[4].ch, ' ');
    }

    #[test]
    fn strip_fill_truncate() {
        let mut s = Strip::new(0, 0);
        s.push_str("Hello World", red_style());
        s.fill(5, CellStyle::default());
        assert_eq!(s.width(), 5);
        assert_eq!(s.cells[4].ch, 'o');
    }

    #[test]
    fn strip_fill_exact() {
        let mut s = Strip::new(0, 0);
        s.push_str("abc", red_style());
        s.fill(3, CellStyle::default());
        assert_eq!(s.width(), 3);
    }

    #[test]
    fn strip_fill_zero() {
        let mut s = Strip::new(0, 0);
        s.push_str("abc", red_style());
        s.fill(0, CellStyle::default());
        assert_eq!(s.width(), 0);
    }
}
