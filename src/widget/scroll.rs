//! Scroll state and scrollbar rendering.
//!
//! `ScrollState` tracks the current scroll position for a scrollable widget,
//! handling clamping, content/viewport size, and scroll percentages.
//! `ScrollbarState` provides the data needed to render a scrollbar indicator.

use crate::geometry::{Offset, Size, Region};

// ---------------------------------------------------------------------------
// ScrollState
// ---------------------------------------------------------------------------

/// Tracks the scroll position for a scrollable widget.
///
/// The scroll offset is always clamped to `[0, max_scroll]` where
/// `max_scroll = content_size - viewport_size` (clamped to zero).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollState {
    /// Current scroll offset (always >= 0, clamped to max).
    pub offset: Offset,
    /// Total size of the scrollable content.
    pub content_size: Size,
    /// Size of the visible viewport.
    pub viewport_size: Size,
}

impl ScrollState {
    /// Create a new scroll state with zero offset.
    pub fn new(content_size: Size, viewport_size: Size) -> Self {
        Self {
            offset: Offset::new(0, 0),
            content_size,
            viewport_size,
        }
    }

    /// The maximum scroll offset for each axis.
    ///
    /// Each component is `max(0, content_size - viewport_size)`.
    pub fn max_scroll(&self) -> Offset {
        Offset::new(
            (self.content_size.width - self.viewport_size.width).max(0),
            (self.content_size.height - self.viewport_size.height).max(0),
        )
    }

    /// Scroll to an absolute position, clamping to valid range.
    pub fn scroll_to(&mut self, x: i32, y: i32) {
        let max = self.max_scroll();
        self.offset = Offset::new(x.clamp(0, max.x), y.clamp(0, max.y));
    }

    /// Scroll by a relative delta, clamping to valid range.
    pub fn scroll_by(&mut self, dx: i32, dy: i32) {
        self.scroll_to(self.offset.x + dx, self.offset.y + dy);
    }

    /// Whether the content is wider than the viewport (horizontal scrolling possible).
    pub fn is_scrollable_x(&self) -> bool {
        self.content_size.width > self.viewport_size.width
    }

    /// Whether the content is taller than the viewport (vertical scrolling possible).
    pub fn is_scrollable_y(&self) -> bool {
        self.content_size.height > self.viewport_size.height
    }

    /// The currently visible region within the content.
    pub fn visible_region(&self) -> Region {
        Region::new(
            self.offset.x,
            self.offset.y,
            self.viewport_size.width,
            self.viewport_size.height,
        )
    }

    /// Horizontal scroll progress as a fraction in `[0.0, 1.0]`.
    ///
    /// Returns 0.0 if not scrollable.
    pub fn scroll_percent_x(&self) -> f32 {
        let max = self.max_scroll().x;
        if max <= 0 {
            0.0
        } else {
            self.offset.x as f32 / max as f32
        }
    }

    /// Vertical scroll progress as a fraction in `[0.0, 1.0]`.
    ///
    /// Returns 0.0 if not scrollable.
    pub fn scroll_percent_y(&self) -> f32 {
        let max = self.max_scroll().y;
        if max <= 0 {
            0.0
        } else {
            self.offset.y as f32 / max as f32
        }
    }

    /// Update the content size and re-clamp the offset.
    pub fn set_content_size(&mut self, size: Size) {
        self.content_size = size;
        // Re-clamp after size change.
        self.scroll_to(self.offset.x, self.offset.y);
    }

    /// Update the viewport size and re-clamp the offset.
    pub fn set_viewport_size(&mut self, size: Size) {
        self.viewport_size = size;
        // Re-clamp after size change.
        self.scroll_to(self.offset.x, self.offset.y);
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new(Size::ZERO, Size::ZERO)
    }
}

// ---------------------------------------------------------------------------
// ScrollbarState
// ---------------------------------------------------------------------------

/// Data needed to render a scrollbar indicator.
///
/// Both `thumb_position` and `thumb_size` are in the range `[0.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbarState {
    /// Position of the scrollbar thumb as a fraction (0.0 = top/left, 1.0 = bottom/right).
    pub thumb_position: f32,
    /// Size of the scrollbar thumb as a fraction of the track (viewport / content ratio).
    pub thumb_size: f32,
}

impl ScrollbarState {
    /// Compute scrollbar state from a `ScrollState` for the given axis.
    ///
    /// If `vertical` is true, uses the Y axis; otherwise uses the X axis.
    pub fn from_scroll_state(state: &ScrollState, vertical: bool) -> Self {
        let (content, viewport, offset) = if vertical {
            (
                state.content_size.height,
                state.viewport_size.height,
                state.offset.y,
            )
        } else {
            (
                state.content_size.width,
                state.viewport_size.width,
                state.offset.x,
            )
        };

        if content <= 0 || viewport <= 0 {
            return ScrollbarState {
                thumb_position: 0.0,
                thumb_size: 1.0,
            };
        }

        let thumb_size = (viewport as f32 / content as f32).clamp(0.0, 1.0);
        let max_scroll = (content - viewport).max(0);
        let thumb_position = if max_scroll > 0 {
            offset as f32 / max_scroll as f32
        } else {
            0.0
        };

        ScrollbarState {
            thumb_position: thumb_position.clamp(0.0, 1.0),
            thumb_size,
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ScrollState — construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_scroll_state() {
        let state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        assert_eq!(state.offset, Offset::new(0, 0));
        assert_eq!(state.content_size, Size::new(100, 200));
        assert_eq!(state.viewport_size, Size::new(40, 30));
    }

    #[test]
    fn default_scroll_state() {
        let state = ScrollState::default();
        assert_eq!(state.offset, Offset::new(0, 0));
        assert_eq!(state.content_size, Size::ZERO);
        assert_eq!(state.viewport_size, Size::ZERO);
    }

    // -----------------------------------------------------------------------
    // ScrollState — max_scroll
    // -----------------------------------------------------------------------

    #[test]
    fn max_scroll_normal() {
        let state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        assert_eq!(state.max_scroll(), Offset::new(60, 170));
    }

    #[test]
    fn max_scroll_content_smaller() {
        let state = ScrollState::new(Size::new(10, 10), Size::new(40, 30));
        assert_eq!(state.max_scroll(), Offset::new(0, 0));
    }

    #[test]
    fn max_scroll_exact_fit() {
        let state = ScrollState::new(Size::new(40, 30), Size::new(40, 30));
        assert_eq!(state.max_scroll(), Offset::new(0, 0));
    }

    // -----------------------------------------------------------------------
    // ScrollState — scroll_to
    // -----------------------------------------------------------------------

    #[test]
    fn scroll_to_within_bounds() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(20, 50);
        assert_eq!(state.offset, Offset::new(20, 50));
    }

    #[test]
    fn scroll_to_clamps_max() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(999, 999);
        assert_eq!(state.offset, Offset::new(60, 170));
    }

    #[test]
    fn scroll_to_clamps_negative() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(-10, -5);
        assert_eq!(state.offset, Offset::new(0, 0));
    }

    // -----------------------------------------------------------------------
    // ScrollState — scroll_by
    // -----------------------------------------------------------------------

    #[test]
    fn scroll_by_positive() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_by(10, 20);
        assert_eq!(state.offset, Offset::new(10, 20));
    }

    #[test]
    fn scroll_by_negative() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(30, 50);
        state.scroll_by(-10, -20);
        assert_eq!(state.offset, Offset::new(20, 30));
    }

    #[test]
    fn scroll_by_clamps() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_by(-5, -5);
        assert_eq!(state.offset, Offset::new(0, 0));
    }

    // -----------------------------------------------------------------------
    // ScrollState — scrollable queries
    // -----------------------------------------------------------------------

    #[test]
    fn is_scrollable() {
        let state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        assert!(state.is_scrollable_x());
        assert!(state.is_scrollable_y());
    }

    #[test]
    fn not_scrollable_when_fits() {
        let state = ScrollState::new(Size::new(40, 30), Size::new(40, 30));
        assert!(!state.is_scrollable_x());
        assert!(!state.is_scrollable_y());
    }

    #[test]
    fn not_scrollable_content_smaller() {
        let state = ScrollState::new(Size::new(10, 5), Size::new(40, 30));
        assert!(!state.is_scrollable_x());
        assert!(!state.is_scrollable_y());
    }

    // -----------------------------------------------------------------------
    // ScrollState — visible_region
    // -----------------------------------------------------------------------

    #[test]
    fn visible_region_at_zero() {
        let state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        assert_eq!(state.visible_region(), Region::new(0, 0, 40, 30));
    }

    #[test]
    fn visible_region_scrolled() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(10, 25);
        assert_eq!(state.visible_region(), Region::new(10, 25, 40, 30));
    }

    // -----------------------------------------------------------------------
    // ScrollState — scroll percentages
    // -----------------------------------------------------------------------

    #[test]
    fn scroll_percent_at_start() {
        let state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        assert_eq!(state.scroll_percent_x(), 0.0);
        assert_eq!(state.scroll_percent_y(), 0.0);
    }

    #[test]
    fn scroll_percent_at_end() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(60, 170); // max scroll
        assert!((state.scroll_percent_x() - 1.0).abs() < f32::EPSILON);
        assert!((state.scroll_percent_y() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_percent_midpoint() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(30, 85); // half of max scroll (60, 170)
        assert!((state.scroll_percent_x() - 0.5).abs() < f32::EPSILON);
        assert!((state.scroll_percent_y() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_percent_not_scrollable() {
        let state = ScrollState::new(Size::new(10, 10), Size::new(40, 30));
        assert_eq!(state.scroll_percent_x(), 0.0);
        assert_eq!(state.scroll_percent_y(), 0.0);
    }

    // -----------------------------------------------------------------------
    // ScrollState — set_content_size / set_viewport_size
    // -----------------------------------------------------------------------

    #[test]
    fn set_content_size_reclamps() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(50, 150);
        // Shrink content so max scroll becomes (10, 20)
        state.set_content_size(Size::new(50, 50));
        assert_eq!(state.offset, Offset::new(10, 20));
    }

    #[test]
    fn set_viewport_size_reclamps() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(50, 150);
        // Grow viewport so max scroll becomes (20, 100)
        state.set_viewport_size(Size::new(80, 100));
        assert_eq!(state.offset, Offset::new(20, 100));
    }

    // -----------------------------------------------------------------------
    // ScrollbarState
    // -----------------------------------------------------------------------

    #[test]
    fn scrollbar_from_scroll_state_vertical() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(0, 85); // half of max_y (170)

        let bar = ScrollbarState::from_scroll_state(&state, true);
        assert!((bar.thumb_position - 0.5).abs() < f32::EPSILON);
        assert!((bar.thumb_size - 30.0 / 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scrollbar_from_scroll_state_horizontal() {
        let mut state = ScrollState::new(Size::new(100, 200), Size::new(40, 30));
        state.scroll_to(30, 0); // half of max_x (60)

        let bar = ScrollbarState::from_scroll_state(&state, false);
        assert!((bar.thumb_position - 0.5).abs() < f32::EPSILON);
        assert!((bar.thumb_size - 40.0 / 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scrollbar_content_fits() {
        let state = ScrollState::new(Size::new(40, 30), Size::new(40, 30));
        let bar = ScrollbarState::from_scroll_state(&state, true);
        assert_eq!(bar.thumb_position, 0.0);
        assert!((bar.thumb_size - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scrollbar_zero_content() {
        let state = ScrollState::new(Size::ZERO, Size::new(40, 30));
        let bar = ScrollbarState::from_scroll_state(&state, true);
        assert_eq!(bar.thumb_position, 0.0);
        assert_eq!(bar.thumb_size, 1.0);
    }
}
