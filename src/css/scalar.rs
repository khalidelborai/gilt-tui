//! CSS scalar values: Scalar, Unit (cells, fr, %, vw, vh, auto).

use std::fmt;

/// A CSS unit type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    /// Cell count (default terminal unit, like "px" in CSS).
    Cells,
    /// Fraction unit (like CSS `fr` in grid).
    Fr,
    /// Percentage of parent dimension.
    Percent,
    /// Viewport width percentage.
    Vw,
    /// Viewport height percentage.
    Vh,
    /// Auto-size (content-based).
    Auto,
}

/// A scalar value with a unit, e.g. `10`, `1fr`, `50%`, `auto`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scalar {
    pub value: f32,
    pub unit: Unit,
}

impl Scalar {
    /// Create a scalar in cell units.
    pub fn cells(value: f32) -> Self {
        Self {
            value,
            unit: Unit::Cells,
        }
    }

    /// Create a scalar in fraction units.
    pub fn fr(value: f32) -> Self {
        Self {
            value,
            unit: Unit::Fr,
        }
    }

    /// Create a scalar as a percentage.
    pub fn percent(value: f32) -> Self {
        Self {
            value,
            unit: Unit::Percent,
        }
    }

    /// Create a scalar in viewport-width units.
    pub fn vw(value: f32) -> Self {
        Self {
            value,
            unit: Unit::Vw,
        }
    }

    /// Create a scalar in viewport-height units.
    pub fn vh(value: f32) -> Self {
        Self {
            value,
            unit: Unit::Vh,
        }
    }

    /// Create an auto scalar.
    pub fn auto() -> Self {
        Self {
            value: 0.0,
            unit: Unit::Auto,
        }
    }

    /// Returns `true` if this scalar is auto-sized.
    pub fn is_auto(&self) -> bool {
        self.unit == Unit::Auto
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            Unit::Auto => write!(f, "auto"),
            Unit::Cells => {
                if self.value.fract() == 0.0 {
                    write!(f, "{}", self.value as i64)
                } else {
                    write!(f, "{}", self.value)
                }
            }
            Unit::Fr => {
                if self.value.fract() == 0.0 {
                    write!(f, "{}fr", self.value as i64)
                } else {
                    write!(f, "{}fr", self.value)
                }
            }
            Unit::Percent => {
                if self.value.fract() == 0.0 {
                    write!(f, "{}%", self.value as i64)
                } else {
                    write!(f, "{}%", self.value)
                }
            }
            Unit::Vw => {
                if self.value.fract() == 0.0 {
                    write!(f, "{}vw", self.value as i64)
                } else {
                    write!(f, "{}vw", self.value)
                }
            }
            Unit::Vh => {
                if self.value.fract() == 0.0 {
                    write!(f, "{}vh", self.value as i64)
                } else {
                    write!(f, "{}vh", self.value)
                }
            }
        }
    }
}

/// Four-sided scalar values (top, right, bottom, left) like CSS margin/padding.
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarBox {
    pub top: Scalar,
    pub right: Scalar,
    pub bottom: Scalar,
    pub left: Scalar,
}

impl ScalarBox {
    /// Create a box with the same scalar on all four sides.
    pub fn all(v: Scalar) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    /// Create a box with symmetric vertical and horizontal values.
    pub fn symmetric(vertical: Scalar, horizontal: Scalar) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create a box with explicit values for all four sides.
    pub fn new(top: Scalar, right: Scalar, bottom: Scalar, left: Scalar) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_cells() {
        let s = Scalar::cells(10.0);
        assert_eq!(s.value, 10.0);
        assert_eq!(s.unit, Unit::Cells);
        assert!(!s.is_auto());
        assert_eq!(s.to_string(), "10");
    }

    #[test]
    fn test_scalar_cells_float() {
        let s = Scalar::cells(2.5);
        assert_eq!(s.to_string(), "2.5");
    }

    #[test]
    fn test_scalar_fr() {
        let s = Scalar::fr(1.0);
        assert_eq!(s.value, 1.0);
        assert_eq!(s.unit, Unit::Fr);
        assert_eq!(s.to_string(), "1fr");
    }

    #[test]
    fn test_scalar_fr_float() {
        let s = Scalar::fr(1.5);
        assert_eq!(s.to_string(), "1.5fr");
    }

    #[test]
    fn test_scalar_percent() {
        let s = Scalar::percent(50.0);
        assert_eq!(s.value, 50.0);
        assert_eq!(s.unit, Unit::Percent);
        assert_eq!(s.to_string(), "50%");
    }

    #[test]
    fn test_scalar_vw() {
        let s = Scalar::vw(100.0);
        assert_eq!(s.value, 100.0);
        assert_eq!(s.unit, Unit::Vw);
        assert_eq!(s.to_string(), "100vw");
    }

    #[test]
    fn test_scalar_vh() {
        let s = Scalar::vh(80.0);
        assert_eq!(s.value, 80.0);
        assert_eq!(s.unit, Unit::Vh);
        assert_eq!(s.to_string(), "80vh");
    }

    #[test]
    fn test_scalar_auto() {
        let s = Scalar::auto();
        assert_eq!(s.value, 0.0);
        assert_eq!(s.unit, Unit::Auto);
        assert!(s.is_auto());
        assert_eq!(s.to_string(), "auto");
    }

    #[test]
    fn test_scalar_box_all() {
        let b = ScalarBox::all(Scalar::cells(5.0));
        assert_eq!(b.top, Scalar::cells(5.0));
        assert_eq!(b.right, Scalar::cells(5.0));
        assert_eq!(b.bottom, Scalar::cells(5.0));
        assert_eq!(b.left, Scalar::cells(5.0));
    }

    #[test]
    fn test_scalar_box_symmetric() {
        let b = ScalarBox::symmetric(Scalar::cells(1.0), Scalar::cells(2.0));
        assert_eq!(b.top, Scalar::cells(1.0));
        assert_eq!(b.right, Scalar::cells(2.0));
        assert_eq!(b.bottom, Scalar::cells(1.0));
        assert_eq!(b.left, Scalar::cells(2.0));
    }

    #[test]
    fn test_scalar_box_new() {
        let b = ScalarBox::new(
            Scalar::cells(1.0),
            Scalar::percent(50.0),
            Scalar::fr(2.0),
            Scalar::auto(),
        );
        assert_eq!(b.top, Scalar::cells(1.0));
        assert_eq!(b.right, Scalar::percent(50.0));
        assert_eq!(b.bottom, Scalar::fr(2.0));
        assert!(b.left.is_auto());
    }

    #[test]
    fn test_scalar_negative() {
        let s = Scalar::cells(-3.0);
        assert_eq!(s.to_string(), "-3");
    }

    #[test]
    fn test_scalar_zero() {
        let s = Scalar::cells(0.0);
        assert_eq!(s.to_string(), "0");
    }
}
