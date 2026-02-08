//! Core geometry types: Offset, Size, Region, Spacing.
//!
//! These are the foundational coordinate types used throughout gilt-tui for positioning,
//! sizing, and spacing widgets in the terminal grid.

use std::ops::{Add, Neg, Sub, Mul};

// ---------------------------------------------------------------------------
// Offset
// ---------------------------------------------------------------------------

/// A 2D displacement or position delta in terminal cells.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

impl Offset {
    /// Create a new offset.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Linearly interpolate between `self` and `other` by `factor`.
    ///
    /// `factor = 0.0` returns `self`, `factor = 1.0` returns `other`.
    #[inline]
    pub fn blend(self, other: Offset, factor: f64) -> Offset {
        let inv = 1.0 - factor;
        Offset {
            x: (self.x as f64 * inv + other.x as f64 * factor).round() as i32,
            y: (self.y as f64 * inv + other.y as f64 * factor).round() as i32,
        }
    }

    /// Manhattan (taxicab) distance to `other`.
    #[inline]
    pub fn manhattan_distance(self, other: Offset) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl Add for Offset {
    type Output = Offset;
    #[inline]
    fn add(self, rhs: Offset) -> Offset {
        Offset { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Sub for Offset {
    type Output = Offset;
    #[inline]
    fn sub(self, rhs: Offset) -> Offset {
        Offset { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl Neg for Offset {
    type Output = Offset;
    #[inline]
    fn neg(self) -> Offset {
        Offset { x: -self.x, y: -self.y }
    }
}

impl Mul<i32> for Offset {
    type Output = Offset;
    #[inline]
    fn mul(self, rhs: i32) -> Offset {
        Offset { x: self.x * rhs, y: self.y * rhs }
    }
}

// ---------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------

/// A 2D size in terminal cells (width x height).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    /// A zero-sized size.
    pub const ZERO: Size = Size { width: 0, height: 0 };

    /// Create a new size.
    #[inline]
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// Total area (width * height).
    #[inline]
    pub const fn area(self) -> i32 {
        self.width * self.height
    }

    /// Whether the point (x, y) is inside `0..width` and `0..height`.
    #[inline]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }

    /// Convert to a [`Region`] positioned at the origin.
    #[inline]
    pub const fn to_region(self) -> Region {
        Region { x: 0, y: 0, width: self.width, height: self.height }
    }
}

impl Add for Size {
    type Output = Size;
    #[inline]
    fn add(self, rhs: Size) -> Size {
        Size { width: self.width + rhs.width, height: self.height + rhs.height }
    }
}

impl Sub for Size {
    type Output = Size;
    #[inline]
    fn sub(self, rhs: Size) -> Size {
        Size { width: self.width - rhs.width, height: self.height - rhs.height }
    }
}

// ---------------------------------------------------------------------------
// Region
// ---------------------------------------------------------------------------

/// A rectangular region in terminal cells defined by position and size.
///
/// This is the most heavily-used geometry type. The `intersection`, `contains`,
/// and property methods are marked `#[inline]` for performance.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Region {
    /// An empty region at the origin.
    pub const EMPTY: Region = Region { x: 0, y: 0, width: 0, height: 0 };

    /// Create a new region.
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// The right edge (exclusive): `x + width`.
    #[inline]
    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    /// The bottom edge (exclusive): `y + height`.
    #[inline]
    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    /// The top-left corner as an [`Offset`].
    #[inline]
    pub const fn offset(self) -> Offset {
        Offset { x: self.x, y: self.y }
    }

    /// The dimensions as a [`Size`].
    #[inline]
    pub const fn size(self) -> Size {
        Size { width: self.width, height: self.height }
    }

    /// Whether the point (x, y) lies inside this region.
    #[inline]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Whether `other` is entirely contained within this region.
    #[inline]
    pub const fn contains_region(self, other: Region) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Whether `other` overlaps this region (non-zero intersection area).
    #[inline]
    pub const fn overlaps(self, other: Region) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Compute the intersection of two regions.
    ///
    /// Returns [`Region::EMPTY`] if the regions do not overlap.
    /// This is performance-critical; uses inline clamping without branching.
    #[inline]
    pub const fn intersection(self, other: Region) -> Region {
        let x1 = if self.x > other.x { self.x } else { other.x };
        let y1 = if self.y > other.y { self.y } else { other.y };

        let sr = self.right();
        let or = other.right();
        let x2 = if sr < or { sr } else { or };

        let sb = self.bottom();
        let ob = other.bottom();
        let y2 = if sb < ob { sb } else { ob };

        let w = x2 - x1;
        let h = y2 - y1;

        if w <= 0 || h <= 0 {
            Region::EMPTY
        } else {
            Region { x: x1, y: y1, width: w, height: h }
        }
    }

    /// Compute the smallest region containing both `self` and `other`.
    #[inline]
    pub const fn union(self, other: Region) -> Region {
        let x1 = if self.x < other.x { self.x } else { other.x };
        let y1 = if self.y < other.y { self.y } else { other.y };

        let sr = self.right();
        let or = other.right();
        let x2 = if sr > or { sr } else { or };

        let sb = self.bottom();
        let ob = other.bottom();
        let y2 = if sb > ob { sb } else { ob };

        Region { x: x1, y: y1, width: x2 - x1, height: y2 - y1 }
    }

    /// Translate the region by an [`Offset`].
    #[inline]
    pub const fn translate(self, offset: Offset) -> Region {
        Region { x: self.x + offset.x, y: self.y + offset.y, width: self.width, height: self.height }
    }

    /// Expand the region outward by the given [`Spacing`].
    #[inline]
    pub const fn grow(self, margin: Spacing) -> Region {
        Region {
            x: self.x - margin.left,
            y: self.y - margin.top,
            width: self.width + margin.left + margin.right,
            height: self.height + margin.top + margin.bottom,
        }
    }

    /// Contract the region inward by the given [`Spacing`].
    ///
    /// Width and height are clamped to zero to avoid negative dimensions.
    #[inline]
    pub const fn shrink(self, margin: Spacing) -> Region {
        let w = self.width - margin.left - margin.right;
        let h = self.height - margin.top - margin.bottom;
        Region {
            x: self.x + margin.left,
            y: self.y + margin.top,
            width: if w > 0 { w } else { 0 },
            height: if h > 0 { h } else { 0 },
        }
    }

    /// Split vertically at `offset` cells from the left edge.
    ///
    /// Returns `(left, right)`. The offset is clamped to `[0, width]`.
    #[inline]
    pub const fn split_vertical(self, offset: i32) -> (Region, Region) {
        let clamped = if offset < 0 {
            0
        } else if offset > self.width {
            self.width
        } else {
            offset
        };
        let left = Region { x: self.x, y: self.y, width: clamped, height: self.height };
        let right = Region {
            x: self.x + clamped,
            y: self.y,
            width: self.width - clamped,
            height: self.height,
        };
        (left, right)
    }

    /// Split horizontally at `offset` cells from the top edge.
    ///
    /// Returns `(top, bottom)`. The offset is clamped to `[0, height]`.
    #[inline]
    pub const fn split_horizontal(self, offset: i32) -> (Region, Region) {
        let clamped = if offset < 0 {
            0
        } else if offset > self.height {
            self.height
        } else {
            offset
        };
        let top = Region { x: self.x, y: self.y, width: self.width, height: clamped };
        let bottom = Region {
            x: self.x,
            y: self.y + clamped,
            width: self.width,
            height: self.height - clamped,
        };
        (top, bottom)
    }

    /// Limit the region's dimensions to the given [`Size`], keeping the position.
    #[inline]
    pub const fn crop_size(self, size: Size) -> Region {
        Region {
            x: self.x,
            y: self.y,
            width: if self.width < size.width { self.width } else { size.width },
            height: if self.height < size.height { self.height } else { size.height },
        }
    }
}

// ---------------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------------

/// Spacing around the four sides of a rectangle, used for margin and padding.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Spacing {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Spacing {
    /// Zero spacing on all sides.
    pub const ZERO: Spacing = Spacing { top: 0, right: 0, bottom: 0, left: 0 };

    /// Create spacing with explicit values for each side.
    #[inline]
    pub const fn new(top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self { top, right, bottom, left }
    }

    /// All four sides set to the same value.
    #[inline]
    pub const fn all(value: i32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }

    /// Symmetric spacing: `vertical` for top/bottom, `horizontal` for left/right.
    #[inline]
    pub const fn symmetric(vertical: i32, horizontal: i32) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }

    /// Only vertical spacing (left and right are zero).
    #[inline]
    pub const fn vertical(top: i32, bottom: i32) -> Self {
        Self { top, right: 0, bottom, left: 0 }
    }

    /// Only horizontal spacing (top and bottom are zero).
    #[inline]
    pub const fn horizontal(left: i32, right: i32) -> Self {
        Self { top: 0, right, bottom: 0, left }
    }

    /// Total horizontal extent: `left + right`.
    #[inline]
    pub const fn width(self) -> i32 {
        self.left + self.right
    }

    /// Total vertical extent: `top + bottom`.
    #[inline]
    pub const fn height(self) -> i32 {
        self.top + self.bottom
    }

    /// Component-wise maximum with `other` (useful for collapsing margins).
    #[inline]
    pub const fn grow_maximum(self, other: Spacing) -> Spacing {
        Spacing {
            top: if self.top > other.top { self.top } else { other.top },
            right: if self.right > other.right { self.right } else { other.right },
            bottom: if self.bottom > other.bottom { self.bottom } else { other.bottom },
            left: if self.left > other.left { self.left } else { other.left },
        }
    }
}

impl Add for Spacing {
    type Output = Spacing;
    #[inline]
    fn add(self, rhs: Spacing) -> Spacing {
        Spacing {
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
            left: self.left + rhs.left,
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
    // Offset
    // -----------------------------------------------------------------------

    #[test]
    fn offset_new_and_default() {
        assert_eq!(Offset::new(3, -7), Offset { x: 3, y: -7 });
        assert_eq!(Offset::default(), Offset { x: 0, y: 0 });
    }

    #[test]
    fn offset_add_sub() {
        let a = Offset::new(1, 2);
        let b = Offset::new(3, 4);
        assert_eq!(a + b, Offset::new(4, 6));
        assert_eq!(b - a, Offset::new(2, 2));
    }

    #[test]
    fn offset_neg() {
        assert_eq!(-Offset::new(5, -3), Offset::new(-5, 3));
    }

    #[test]
    fn offset_mul_scalar() {
        assert_eq!(Offset::new(2, -3) * 4, Offset::new(8, -12));
        assert_eq!(Offset::new(1, 1) * 0, Offset::new(0, 0));
    }

    #[test]
    fn offset_blend() {
        let a = Offset::new(0, 0);
        let b = Offset::new(10, 20);
        assert_eq!(a.blend(b, 0.0), a);
        assert_eq!(a.blend(b, 1.0), b);
        assert_eq!(a.blend(b, 0.5), Offset::new(5, 10));
    }

    #[test]
    fn offset_blend_rounding() {
        let a = Offset::new(0, 0);
        let b = Offset::new(3, 3);
        // 0.33 * 3 = 0.99 rounds to 1
        assert_eq!(a.blend(b, 0.33), Offset::new(1, 1));
    }

    #[test]
    fn offset_manhattan_distance() {
        let a = Offset::new(1, 2);
        let b = Offset::new(4, -1);
        assert_eq!(a.manhattan_distance(b), 6); // |3| + |3|
        assert_eq!(a.manhattan_distance(a), 0);
    }

    // -----------------------------------------------------------------------
    // Size
    // -----------------------------------------------------------------------

    #[test]
    fn size_new_and_constants() {
        assert_eq!(Size::new(80, 24), Size { width: 80, height: 24 });
        assert_eq!(Size::ZERO, Size { width: 0, height: 0 });
        assert_eq!(Size::default(), Size::ZERO);
    }

    #[test]
    fn size_area() {
        assert_eq!(Size::new(10, 5).area(), 50);
        assert_eq!(Size::ZERO.area(), 0);
    }

    #[test]
    fn size_contains() {
        let s = Size::new(10, 5);
        assert!(s.contains(0, 0));
        assert!(s.contains(9, 4));
        assert!(!s.contains(10, 0));
        assert!(!s.contains(0, 5));
        assert!(!s.contains(-1, 0));
    }

    #[test]
    fn size_to_region() {
        assert_eq!(
            Size::new(80, 24).to_region(),
            Region::new(0, 0, 80, 24),
        );
    }

    #[test]
    fn size_add_sub() {
        let a = Size::new(10, 5);
        let b = Size::new(3, 2);
        assert_eq!(a + b, Size::new(13, 7));
        assert_eq!(a - b, Size::new(7, 3));
    }

    // -----------------------------------------------------------------------
    // Region — basic properties
    // -----------------------------------------------------------------------

    #[test]
    fn region_new_and_empty() {
        let r = Region::new(1, 2, 3, 4);
        assert_eq!(r.x, 1);
        assert_eq!(r.y, 2);
        assert_eq!(r.width, 3);
        assert_eq!(r.height, 4);
        assert_eq!(Region::EMPTY, Region::new(0, 0, 0, 0));
        assert_eq!(Region::default(), Region::EMPTY);
    }

    #[test]
    fn region_right_bottom() {
        let r = Region::new(5, 10, 20, 30);
        assert_eq!(r.right(), 25);
        assert_eq!(r.bottom(), 40);
    }

    #[test]
    fn region_offset_size() {
        let r = Region::new(5, 10, 20, 30);
        assert_eq!(r.offset(), Offset::new(5, 10));
        assert_eq!(r.size(), Size::new(20, 30));
    }

    // -----------------------------------------------------------------------
    // Region — containment & overlap
    // -----------------------------------------------------------------------

    #[test]
    fn region_contains_point() {
        let r = Region::new(5, 5, 10, 10);
        assert!(r.contains(5, 5));
        assert!(r.contains(14, 14));
        assert!(!r.contains(15, 5));
        assert!(!r.contains(5, 15));
        assert!(!r.contains(4, 5));
    }

    #[test]
    fn region_contains_region() {
        let outer = Region::new(0, 0, 100, 100);
        let inner = Region::new(10, 10, 20, 20);
        assert!(outer.contains_region(inner));
        assert!(!inner.contains_region(outer));
        assert!(outer.contains_region(outer)); // self-containment
    }

    #[test]
    fn region_overlaps() {
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(5, 5, 10, 10);
        assert!(a.overlaps(b));
        assert!(b.overlaps(a));

        // Adjacent but not overlapping.
        let c = Region::new(10, 0, 10, 10);
        assert!(!a.overlaps(c));
    }

    #[test]
    fn region_overlaps_zero_size() {
        let a = Region::new(0, 0, 10, 10);
        let z = Region::EMPTY;
        assert!(!a.overlaps(z));
    }

    // -----------------------------------------------------------------------
    // Region — intersection
    // -----------------------------------------------------------------------

    #[test]
    fn region_intersection_basic() {
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(5, 5, 10, 10);
        assert_eq!(a.intersection(b), Region::new(5, 5, 5, 5));
    }

    #[test]
    fn region_intersection_no_overlap() {
        let a = Region::new(0, 0, 5, 5);
        let b = Region::new(10, 10, 5, 5);
        assert_eq!(a.intersection(b), Region::EMPTY);
    }

    #[test]
    fn region_intersection_adjacent() {
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(10, 0, 10, 10);
        assert_eq!(a.intersection(b), Region::EMPTY);
    }

    #[test]
    fn region_intersection_self() {
        let r = Region::new(3, 4, 20, 15);
        assert_eq!(r.intersection(r), r);
    }

    #[test]
    fn region_intersection_contained() {
        let outer = Region::new(0, 0, 100, 100);
        let inner = Region::new(10, 10, 5, 5);
        assert_eq!(outer.intersection(inner), inner);
        assert_eq!(inner.intersection(outer), inner);
    }

    // -----------------------------------------------------------------------
    // Region — union
    // -----------------------------------------------------------------------

    #[test]
    fn region_union_basic() {
        let a = Region::new(0, 0, 5, 5);
        let b = Region::new(10, 10, 5, 5);
        assert_eq!(a.union(b), Region::new(0, 0, 15, 15));
    }

    #[test]
    fn region_union_self() {
        let r = Region::new(3, 4, 10, 10);
        assert_eq!(r.union(r), r);
    }

    // -----------------------------------------------------------------------
    // Region — translate
    // -----------------------------------------------------------------------

    #[test]
    fn region_translate() {
        let r = Region::new(5, 10, 20, 30);
        let moved = r.translate(Offset::new(-5, 3));
        assert_eq!(moved, Region::new(0, 13, 20, 30));
    }

    #[test]
    fn region_translate_zero() {
        let r = Region::new(1, 2, 3, 4);
        assert_eq!(r.translate(Offset::new(0, 0)), r);
    }

    // -----------------------------------------------------------------------
    // Region — grow / shrink
    // -----------------------------------------------------------------------

    #[test]
    fn region_grow() {
        let r = Region::new(10, 10, 20, 20);
        let s = Spacing::all(5);
        let grown = r.grow(s);
        assert_eq!(grown, Region::new(5, 5, 30, 30));
    }

    #[test]
    fn region_shrink() {
        let r = Region::new(10, 10, 20, 20);
        let s = Spacing::all(5);
        let shrunk = r.shrink(s);
        assert_eq!(shrunk, Region::new(15, 15, 10, 10));
    }

    #[test]
    fn region_grow_shrink_roundtrip() {
        let r = Region::new(10, 10, 40, 30);
        let s = Spacing::new(2, 3, 4, 5);
        assert_eq!(r.grow(s).shrink(s), r);
    }

    #[test]
    fn region_shrink_clamps_to_zero() {
        let r = Region::new(5, 5, 4, 4);
        let s = Spacing::all(10);
        let shrunk = r.shrink(s);
        assert_eq!(shrunk.width, 0);
        assert_eq!(shrunk.height, 0);
    }

    // -----------------------------------------------------------------------
    // Region — split
    // -----------------------------------------------------------------------

    #[test]
    fn region_split_vertical() {
        let r = Region::new(0, 0, 80, 24);
        let (left, right) = r.split_vertical(30);
        assert_eq!(left, Region::new(0, 0, 30, 24));
        assert_eq!(right, Region::new(30, 0, 50, 24));
    }

    #[test]
    fn region_split_vertical_at_zero() {
        let r = Region::new(5, 5, 20, 10);
        let (left, right) = r.split_vertical(0);
        assert_eq!(left.width, 0);
        assert_eq!(right, r);
    }

    #[test]
    fn region_split_vertical_at_full_width() {
        let r = Region::new(5, 5, 20, 10);
        let (left, right) = r.split_vertical(20);
        assert_eq!(left, r);
        assert_eq!(right.width, 0);
    }

    #[test]
    fn region_split_vertical_clamped() {
        let r = Region::new(0, 0, 10, 10);
        let (left, right) = r.split_vertical(100);
        assert_eq!(left, r);
        assert_eq!(right.width, 0);

        let (left2, right2) = r.split_vertical(-5);
        assert_eq!(left2.width, 0);
        assert_eq!(right2, r);
    }

    #[test]
    fn region_split_horizontal() {
        let r = Region::new(0, 0, 80, 24);
        let (top, bottom) = r.split_horizontal(10);
        assert_eq!(top, Region::new(0, 0, 80, 10));
        assert_eq!(bottom, Region::new(0, 10, 80, 14));
    }

    #[test]
    fn region_split_horizontal_clamped() {
        let r = Region::new(0, 0, 10, 10);
        let (top, bottom) = r.split_horizontal(50);
        assert_eq!(top, r);
        assert_eq!(bottom.height, 0);

        let (top2, bottom2) = r.split_horizontal(-5);
        assert_eq!(top2.height, 0);
        assert_eq!(bottom2, r);
    }

    // -----------------------------------------------------------------------
    // Region — crop_size
    // -----------------------------------------------------------------------

    #[test]
    fn region_crop_size() {
        let r = Region::new(5, 5, 100, 50);
        let cropped = r.crop_size(Size::new(20, 10));
        assert_eq!(cropped, Region::new(5, 5, 20, 10));
    }

    #[test]
    fn region_crop_size_no_change() {
        let r = Region::new(0, 0, 10, 10);
        let cropped = r.crop_size(Size::new(100, 100));
        assert_eq!(cropped, r);
    }

    // -----------------------------------------------------------------------
    // Spacing
    // -----------------------------------------------------------------------

    #[test]
    fn spacing_constructors() {
        assert_eq!(Spacing::new(1, 2, 3, 4), Spacing { top: 1, right: 2, bottom: 3, left: 4 });
        assert_eq!(Spacing::all(5), Spacing { top: 5, right: 5, bottom: 5, left: 5 });
        assert_eq!(Spacing::symmetric(3, 7), Spacing { top: 3, right: 7, bottom: 3, left: 7 });
        assert_eq!(Spacing::vertical(2, 4), Spacing { top: 2, right: 0, bottom: 4, left: 0 });
        assert_eq!(Spacing::horizontal(6, 8), Spacing { top: 0, right: 8, bottom: 0, left: 6 });
    }

    #[test]
    fn spacing_zero_and_default() {
        assert_eq!(Spacing::ZERO, Spacing::new(0, 0, 0, 0));
        assert_eq!(Spacing::default(), Spacing::ZERO);
    }

    #[test]
    fn spacing_width_height() {
        let s = Spacing::new(1, 2, 3, 4);
        assert_eq!(s.width(), 6);  // left(4) + right(2)
        assert_eq!(s.height(), 4); // top(1) + bottom(3)
    }

    #[test]
    fn spacing_add() {
        let a = Spacing::new(1, 2, 3, 4);
        let b = Spacing::new(10, 20, 30, 40);
        assert_eq!(a + b, Spacing::new(11, 22, 33, 44));
    }

    #[test]
    fn spacing_grow_maximum() {
        let a = Spacing::new(1, 20, 3, 40);
        let b = Spacing::new(10, 2, 30, 4);
        assert_eq!(a.grow_maximum(b), Spacing::new(10, 20, 30, 40));
    }

    #[test]
    fn spacing_grow_maximum_self() {
        let s = Spacing::new(5, 6, 7, 8);
        assert_eq!(s.grow_maximum(s), s);
    }

    // -----------------------------------------------------------------------
    // Trait derivation smoke tests
    // -----------------------------------------------------------------------

    #[test]
    fn types_are_copy() {
        let o = Offset::new(1, 2);
        let o2 = o; // Copy
        assert_eq!(o, o2);

        let s = Size::new(3, 4);
        let s2 = s;
        assert_eq!(s, s2);

        let r = Region::new(1, 2, 3, 4);
        let r2 = r;
        assert_eq!(r, r2);

        let sp = Spacing::all(5);
        let sp2 = sp;
        assert_eq!(sp, sp2);
    }

    #[test]
    fn types_implement_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Offset::new(1, 2));
        set.insert(Offset::new(1, 2));
        assert_eq!(set.len(), 1);

        let mut set2 = HashSet::new();
        set2.insert(Region::new(0, 0, 10, 10));
        set2.insert(Region::new(0, 0, 10, 10));
        assert_eq!(set2.len(), 1);
    }

    #[test]
    fn types_debug_format() {
        // Just ensure Debug doesn't panic.
        let _ = format!("{:?}", Offset::new(1, 2));
        let _ = format!("{:?}", Size::new(3, 4));
        let _ = format!("{:?}", Region::new(5, 6, 7, 8));
        let _ = format!("{:?}", Spacing::new(1, 2, 3, 4));
    }
}
