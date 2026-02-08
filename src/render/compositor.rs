//! Region-based dirty tracking and strip assembly.
//!
//! The `Compositor` maintains a 2D grid of `StyledCell`s representing the full
//! terminal screen. Widgets render into `Strip`s, which are placed into the screen
//! buffer via `place_strips`. The `diff` method compares two frames and produces
//! only the `CellUpdate`s needed to transition between them.

use crate::geometry::Region;
use super::strip::{Strip, StyledCell, CellStyle};

// ---------------------------------------------------------------------------
// CellUpdate
// ---------------------------------------------------------------------------

/// A single cell that changed between frames.
///
/// Used by the driver to emit minimal terminal escape sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellUpdate {
    pub x: u16,
    pub y: u16,
    pub cell: StyledCell,
}

// ---------------------------------------------------------------------------
// Compositor
// ---------------------------------------------------------------------------

/// Manages a screen buffer with dirty-region tracking.
///
/// The compositor owns the "current frame" screen buffer. During each render cycle:
/// 1. Widgets report dirty regions via `mark_dirty`.
/// 2. The app re-renders widgets whose regions overlap dirty areas.
/// 3. Rendered strips are placed via `place_strips`.
/// 4. `diff` compares against the previous frame to find changed cells.
/// 5. Changed cells are sent to the `Driver` for terminal output.
#[derive(Debug, Clone)]
pub struct Compositor {
    /// The 2D screen buffer. `screen[y][x]` is the cell at column x, row y.
    screen: Vec<Vec<StyledCell>>,
    /// Terminal width.
    pub width: u16,
    /// Terminal height.
    pub height: u16,
    /// Regions that need redrawing.
    dirty_regions: Vec<Region>,
}

impl Compositor {
    /// Create a new compositor with a blank screen of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let screen = Self::blank_screen(width, height);
        Self {
            screen,
            width,
            height,
            dirty_regions: Vec::new(),
        }
    }

    /// Resize the screen buffer. All cells are reset to blank.
    ///
    /// After resize, the entire screen is marked dirty.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.screen = Self::blank_screen(width, height);
        self.mark_all_dirty();
    }

    /// Mark a region as dirty (needs redraw).
    pub fn mark_dirty(&mut self, region: Region) {
        self.dirty_regions.push(region);
    }

    /// Mark the entire screen as dirty.
    pub fn mark_all_dirty(&mut self) {
        self.dirty_regions.clear();
        self.dirty_regions.push(Region::new(
            0,
            0,
            self.width as i32,
            self.height as i32,
        ));
    }

    /// Whether any regions need redrawing.
    pub fn is_dirty(&self) -> bool {
        !self.dirty_regions.is_empty()
    }

    /// Get the current dirty regions.
    pub fn dirty_regions(&self) -> &[Region] {
        &self.dirty_regions
    }

    /// Place rendered strips into the screen buffer, clipped to the given region.
    ///
    /// Each strip's cells are written into the screen at their (x_offset + i, y) position,
    /// but only if that position falls within both the clip `region` and the screen bounds.
    pub fn place_strips(&mut self, strips: &[Strip], region: &Region) {
        let screen_region = Region::new(0, 0, self.width as i32, self.height as i32);
        let clip = region.intersection(screen_region);

        if clip.width <= 0 || clip.height <= 0 {
            return;
        }

        for strip in strips {
            let y = strip.y;
            if y < clip.y || y >= clip.bottom() {
                continue;
            }

            let row = y as usize;
            if row >= self.screen.len() {
                continue;
            }

            for (i, cell) in strip.cells.iter().enumerate() {
                let x = strip.x_offset + i as i32;
                if x < clip.x || x >= clip.right() {
                    continue;
                }
                let col = x as usize;
                if col < self.screen[row].len() {
                    self.screen[row][col] = cell.clone();
                }
            }
        }
    }

    /// Compare this frame against a previous frame and return only the changed cells.
    ///
    /// This is the core of the differential rendering optimization: instead of
    /// redrawing the entire screen, only cells that differ between frames are sent
    /// to the terminal.
    pub fn diff(&self, previous: &Compositor) -> Vec<CellUpdate> {
        let mut updates = Vec::new();
        let h = self.height.min(previous.height) as usize;
        let w = self.width.min(previous.width) as usize;

        for y in 0..h {
            for x in 0..w {
                if self.screen[y][x] != previous.screen[y][x] {
                    updates.push(CellUpdate {
                        x: x as u16,
                        y: y as u16,
                        cell: self.screen[y][x].clone(),
                    });
                }
            }
        }

        // If the new frame is larger, all new cells are updates.
        if self.height > previous.height || self.width > previous.width {
            for y in 0..self.height as usize {
                for x in 0..self.width as usize {
                    if y >= previous.height as usize || x >= previous.width as usize {
                        updates.push(CellUpdate {
                            x: x as u16,
                            y: y as u16,
                            cell: self.screen[y][x].clone(),
                        });
                    }
                }
            }
        }

        updates
    }

    /// Clear dirty regions after a render cycle.
    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }

    /// Get a reference to the screen buffer cell at (x, y).
    ///
    /// Returns `None` if coordinates are out of bounds.
    pub fn get_cell(&self, x: u16, y: u16) -> Option<&StyledCell> {
        self.screen
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
    }

    /// Fill the entire screen with a given style (useful for background).
    pub fn fill(&mut self, style: CellStyle) {
        for row in &mut self.screen {
            for cell in row.iter_mut() {
                *cell = StyledCell::blank_styled(style.clone());
            }
        }
    }

    /// Create a blank screen buffer.
    fn blank_screen(width: u16, height: u16) -> Vec<Vec<StyledCell>> {
        (0..height as usize)
            .map(|_| {
                (0..width as usize)
                    .map(|_| StyledCell::blank())
                    .collect()
            })
            .collect()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_strip(y: i32, x_offset: i32, text: &str, style: CellStyle) -> Strip {
        let mut strip = Strip::new(y, x_offset);
        strip.push_str(text, style);
        strip
    }

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_compositor() {
        let c = Compositor::new(80, 24);
        assert_eq!(c.width, 80);
        assert_eq!(c.height, 24);
        assert!(!c.is_dirty());
    }

    #[test]
    fn new_compositor_blank_cells() {
        let c = Compositor::new(10, 5);
        for y in 0..5u16 {
            for x in 0..10u16 {
                let cell = c.get_cell(x, y).unwrap();
                assert_eq!(cell.ch, ' ');
                assert_eq!(cell.style, CellStyle::default());
            }
        }
    }

    #[test]
    fn get_cell_out_of_bounds() {
        let c = Compositor::new(10, 5);
        assert!(c.get_cell(10, 0).is_none());
        assert!(c.get_cell(0, 5).is_none());
        assert!(c.get_cell(100, 100).is_none());
    }

    // -----------------------------------------------------------------------
    // Dirty regions
    // -----------------------------------------------------------------------

    #[test]
    fn mark_dirty() {
        let mut c = Compositor::new(80, 24);
        assert!(!c.is_dirty());
        c.mark_dirty(Region::new(0, 0, 10, 10));
        assert!(c.is_dirty());
        assert_eq!(c.dirty_regions().len(), 1);
    }

    #[test]
    fn mark_all_dirty() {
        let mut c = Compositor::new(80, 24);
        c.mark_all_dirty();
        assert!(c.is_dirty());
        assert_eq!(c.dirty_regions()[0], Region::new(0, 0, 80, 24));
    }

    #[test]
    fn clear_dirty() {
        let mut c = Compositor::new(80, 24);
        c.mark_dirty(Region::new(0, 0, 10, 10));
        c.clear_dirty();
        assert!(!c.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Resize
    // -----------------------------------------------------------------------

    #[test]
    fn resize_resets_screen() {
        let mut c = Compositor::new(10, 5);
        // Place some content
        let strip = make_strip(0, 0, "Hello", CellStyle::default());
        c.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        // Resize — screen should be blank again
        c.resize(20, 10);
        assert_eq!(c.width, 20);
        assert_eq!(c.height, 10);
        let cell = c.get_cell(0, 0).unwrap();
        assert_eq!(cell.ch, ' ');
        // Should be dirty after resize
        assert!(c.is_dirty());
    }

    // -----------------------------------------------------------------------
    // place_strips
    // -----------------------------------------------------------------------

    #[test]
    fn place_strips_basic() {
        let mut c = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "Hi", CellStyle::default());
        c.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        assert_eq!(c.get_cell(0, 0).unwrap().ch, 'H');
        assert_eq!(c.get_cell(1, 0).unwrap().ch, 'i');
        assert_eq!(c.get_cell(2, 0).unwrap().ch, ' ');
    }

    #[test]
    fn place_strips_with_offset() {
        let mut c = Compositor::new(10, 5);
        let strip = make_strip(2, 3, "AB", CellStyle::default());
        c.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        assert_eq!(c.get_cell(3, 2).unwrap().ch, 'A');
        assert_eq!(c.get_cell(4, 2).unwrap().ch, 'B');
        // Adjacent cells unchanged
        assert_eq!(c.get_cell(2, 2).unwrap().ch, ' ');
        assert_eq!(c.get_cell(5, 2).unwrap().ch, ' ');
    }

    #[test]
    fn place_strips_clipped() {
        let mut c = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "Hello World!", CellStyle::default());
        // Clip region is only 5 wide
        c.place_strips(&[strip], &Region::new(0, 0, 5, 1));

        assert_eq!(c.get_cell(4, 0).unwrap().ch, 'o');
        // Position 5 should still be blank (clipped)
        assert_eq!(c.get_cell(5, 0).unwrap().ch, ' ');
    }

    #[test]
    fn place_strips_outside_screen() {
        let mut c = Compositor::new(10, 5);
        // Strip at y=10, way outside the 5-row screen
        let strip = make_strip(10, 0, "Ghost", CellStyle::default());
        c.place_strips(&[strip], &Region::new(0, 0, 10, 20));
        // Should not crash, and screen should be unchanged
        assert_eq!(c.get_cell(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn place_strips_multiple() {
        let mut c = Compositor::new(10, 3);
        let strips = vec![
            make_strip(0, 0, "Row0", CellStyle::default()),
            make_strip(1, 0, "Row1", CellStyle::default()),
            make_strip(2, 0, "Row2", CellStyle::default()),
        ];
        c.place_strips(&strips, &Region::new(0, 0, 10, 3));

        assert_eq!(c.get_cell(0, 0).unwrap().ch, 'R');
        assert_eq!(c.get_cell(3, 0).unwrap().ch, '0');
        assert_eq!(c.get_cell(3, 1).unwrap().ch, '1');
        assert_eq!(c.get_cell(3, 2).unwrap().ch, '2');
    }

    #[test]
    fn place_strips_with_style() {
        let style = CellStyle {
            fg: Some("red".into()),
            bold: true,
            ..CellStyle::default()
        };
        let mut c = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "X", style.clone());
        c.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        let cell = c.get_cell(0, 0).unwrap();
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.style, style);
    }

    // -----------------------------------------------------------------------
    // diff
    // -----------------------------------------------------------------------

    #[test]
    fn diff_identical_frames() {
        let a = Compositor::new(10, 5);
        let b = Compositor::new(10, 5);
        let updates = a.diff(&b);
        assert!(updates.is_empty());
    }

    #[test]
    fn diff_single_change() {
        let prev = Compositor::new(10, 5);
        let mut curr = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "A", CellStyle::default());
        curr.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        let updates = curr.diff(&prev);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].x, 0);
        assert_eq!(updates[0].y, 0);
        assert_eq!(updates[0].cell.ch, 'A');
    }

    #[test]
    fn diff_multiple_changes() {
        let prev = Compositor::new(10, 5);
        let mut curr = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "ABC", CellStyle::default());
        curr.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        let updates = curr.diff(&prev);
        assert_eq!(updates.len(), 3);
    }

    #[test]
    fn diff_style_change() {
        let mut prev = Compositor::new(10, 5);
        let strip = make_strip(0, 0, "X", CellStyle::default());
        prev.place_strips(&[strip], &Region::new(0, 0, 10, 5));

        let mut curr = Compositor::new(10, 5);
        let style = CellStyle {
            fg: Some("red".into()),
            ..CellStyle::default()
        };
        let strip2 = make_strip(0, 0, "X", style);
        curr.place_strips(&[strip2], &Region::new(0, 0, 10, 5));

        let updates = curr.diff(&prev);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].cell.style.fg, Some("red".into()));
    }

    // -----------------------------------------------------------------------
    // fill
    // -----------------------------------------------------------------------

    #[test]
    fn fill_screen() {
        let mut c = Compositor::new(5, 3);
        let style = CellStyle {
            bg: Some("blue".into()),
            ..CellStyle::default()
        };
        c.fill(style.clone());

        for y in 0..3u16 {
            for x in 0..5u16 {
                let cell = c.get_cell(x, y).unwrap();
                assert_eq!(cell.ch, ' ');
                assert_eq!(cell.style, style);
            }
        }
    }
}
