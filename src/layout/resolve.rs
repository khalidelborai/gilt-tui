//! CSS Scalar -> taffy Style conversion.
//!
//! Maps gilt-tui's CSS types ([`Scalar`], [`ScalarBox`], [`Styles`]) to taffy's
//! layout types ([`taffy::Style`], [`LengthPercentageAuto`], etc.).

use taffy::prelude::*;

use crate::css::scalar::{Scalar, ScalarBox, Unit};
use crate::css::styles::{BorderKind, Dock, LayoutDirection, Styles};

/// Convert a [`Scalar`] to a [`LengthPercentageAuto`], resolving viewport-relative
/// units against the given viewport size.
///
/// - `Cells` -> points (length)
/// - `Percent` -> percent (0..100 range mapped to 0..1)
/// - `Vw` -> resolved to absolute length against viewport width
/// - `Vh` -> resolved to absolute length against viewport height
/// - `Auto` -> auto
/// - `Fr` -> auto (fr is handled at grid track level, not per-node)
pub fn resolve_scalar(
    scalar: &Scalar,
    viewport: taffy::geometry::Size<f32>,
) -> LengthPercentageAuto {
    match scalar.unit {
        Unit::Cells => LengthPercentageAuto::from_length(scalar.value),
        Unit::Percent => LengthPercentageAuto::from_percent(scalar.value / 100.0),
        Unit::Vw => LengthPercentageAuto::from_length(scalar.value / 100.0 * viewport.width),
        Unit::Vh => LengthPercentageAuto::from_length(scalar.value / 100.0 * viewport.height),
        Unit::Auto | Unit::Fr => LengthPercentageAuto::AUTO,
    }
}

/// Convert a [`Scalar`] to a [`LengthPercentage`] for contexts that do not allow auto
/// (e.g. min/max widths, padding, border).
///
/// `Auto` and `Fr` map to zero length since there's no auto variant.
pub fn resolve_scalar_definite(
    scalar: &Scalar,
    viewport: taffy::geometry::Size<f32>,
) -> LengthPercentage {
    match scalar.unit {
        Unit::Cells => LengthPercentage::from_length(scalar.value),
        Unit::Percent => LengthPercentage::from_percent(scalar.value / 100.0),
        Unit::Vw => LengthPercentage::from_length(scalar.value / 100.0 * viewport.width),
        Unit::Vh => LengthPercentage::from_length(scalar.value / 100.0 * viewport.height),
        Unit::Auto | Unit::Fr => LengthPercentage::ZERO,
    }
}

/// Convert a [`Scalar`] to a [`Dimension`] for sizing contexts (width, height, min/max).
///
/// - `Cells` -> length
/// - `Percent` -> percent
/// - `Vw`/`Vh` -> resolved absolute length
/// - `Auto`/`Fr` -> auto
fn resolve_scalar_dimension(
    scalar: &Scalar,
    viewport: taffy::geometry::Size<f32>,
) -> Dimension {
    match scalar.unit {
        Unit::Cells => Dimension::from_length(scalar.value),
        Unit::Percent => Dimension::from_percent(scalar.value / 100.0),
        Unit::Vw => Dimension::from_length(scalar.value / 100.0 * viewport.width),
        Unit::Vh => Dimension::from_length(scalar.value / 100.0 * viewport.height),
        Unit::Auto | Unit::Fr => Dimension::AUTO,
    }
}

/// Convert a 4-sided [`ScalarBox`] to a taffy [`Rect<LengthPercentageAuto>`].
pub fn resolve_scalar_box(
    box_: &ScalarBox,
    viewport: taffy::geometry::Size<f32>,
) -> taffy::geometry::Rect<LengthPercentageAuto> {
    taffy::geometry::Rect {
        top: resolve_scalar(&box_.top, viewport),
        right: resolve_scalar(&box_.right, viewport),
        bottom: resolve_scalar(&box_.bottom, viewport),
        left: resolve_scalar(&box_.left, viewport),
    }
}

/// Convert a 4-sided [`ScalarBox`] to a taffy [`Rect<LengthPercentage>`] (no auto).
fn resolve_scalar_box_definite(
    box_: &ScalarBox,
    viewport: taffy::geometry::Size<f32>,
) -> taffy::geometry::Rect<LengthPercentage> {
    taffy::geometry::Rect {
        top: resolve_scalar_definite(&box_.top, viewport),
        right: resolve_scalar_definite(&box_.right, viewport),
        bottom: resolve_scalar_definite(&box_.bottom, viewport),
        left: resolve_scalar_definite(&box_.left, viewport),
    }
}

/// Convert an [`Overflow`] value to taffy's [`taffy::style::Overflow`].
fn resolve_overflow(overflow: &crate::css::styles::Overflow) -> taffy::style::Overflow {
    match overflow {
        crate::css::styles::Overflow::Hidden => taffy::style::Overflow::Hidden,
        crate::css::styles::Overflow::Scroll | crate::css::styles::Overflow::Auto => {
            taffy::style::Overflow::Scroll
        }
    }
}

/// Convert a full [`Styles`] into a [`taffy::Style`].
///
/// The `viewport_size` is `(columns, rows)` representing the terminal dimensions.
/// This is used to resolve `vw` and `vh` units.
///
/// Mapping summary:
/// - `display: Block` -> `Display::Flex`, `display: None` -> `Display::None`
/// - `layout: Vertical` -> `FlexDirection::Column`, `Horizontal` -> `Row`, `Grid` -> `Display::Grid`
/// - `width/height` -> `size`
/// - `min_width/min_height` -> `min_size`
/// - `max_width/max_height` -> `max_size`
/// - `margin` -> `margin`
/// - `padding` -> `padding`
/// - `overflow_x/overflow_y` -> `overflow`
/// - `dock` -> `position: absolute` with inset
/// - `border` with non-None kind -> 1 cell border on each side
pub fn resolve_styles(styles: &Styles, viewport_size: (u16, u16)) -> taffy::Style {
    let viewport = taffy::geometry::Size {
        width: viewport_size.0 as f32,
        height: viewport_size.1 as f32,
    };

    let mut style = taffy::Style::default();

    // Display and layout direction
    let is_grid = matches!(styles.layout, Some(LayoutDirection::Grid));
    match styles.display {
        Some(crate::css::styles::Display::None) => {
            style.display = Display::None;
        }
        _ => {
            // Default or Block
            if is_grid {
                style.display = Display::Grid;
            } else {
                style.display = Display::Flex;
            }
        }
    }

    // Flex direction (only relevant when display is Flex)
    match styles.layout {
        Some(LayoutDirection::Horizontal) => {
            style.flex_direction = FlexDirection::Row;
        }
        Some(LayoutDirection::Vertical) | None => {
            style.flex_direction = FlexDirection::Column;
        }
        Some(LayoutDirection::Grid) => {
            // Grid direction is handled by display, flex_direction is ignored
        }
    }

    // Size
    if let Some(ref w) = styles.width {
        style.size.width = resolve_scalar_dimension(w, viewport);
    }
    if let Some(ref h) = styles.height {
        style.size.height = resolve_scalar_dimension(h, viewport);
    }

    // Min size
    if let Some(ref w) = styles.min_width {
        style.min_size.width = resolve_scalar_dimension(w, viewport);
    }
    if let Some(ref h) = styles.min_height {
        style.min_size.height = resolve_scalar_dimension(h, viewport);
    }

    // Max size
    if let Some(ref w) = styles.max_width {
        style.max_size.width = resolve_scalar_dimension(w, viewport);
    }
    if let Some(ref h) = styles.max_height {
        style.max_size.height = resolve_scalar_dimension(h, viewport);
    }

    // Margin
    if let Some(ref m) = styles.margin {
        style.margin = resolve_scalar_box(m, viewport);
    }

    // Padding
    if let Some(ref p) = styles.padding {
        style.padding = resolve_scalar_box_definite(p, viewport);
    }

    // Overflow
    let ox = styles
        .overflow_x
        .as_ref()
        .map(resolve_overflow)
        .unwrap_or(taffy::style::Overflow::Visible);
    let oy = styles
        .overflow_y
        .as_ref()
        .map(resolve_overflow)
        .unwrap_or(taffy::style::Overflow::Visible);
    style.overflow = taffy::geometry::Point { x: ox, y: oy };

    // Border: if styles.border is Some with a non-None kind, add 1 cell on each side
    if let Some(ref border) = styles.border {
        if border.kind != BorderKind::None {
            style.border = taffy::geometry::Rect {
                top: LengthPercentage::from_length(1.0),
                right: LengthPercentage::from_length(1.0),
                bottom: LengthPercentage::from_length(1.0),
                left: LengthPercentage::from_length(1.0),
            };
        }
    }

    // Dock -> position: absolute with inset
    if let Some(ref dock) = styles.dock {
        style.position = Position::Absolute;
        match dock {
            Dock::Top => {
                style.inset = taffy::geometry::Rect {
                    top: LengthPercentageAuto::from_length(0.0),
                    left: LengthPercentageAuto::from_length(0.0),
                    right: LengthPercentageAuto::from_length(0.0),
                    bottom: LengthPercentageAuto::AUTO,
                };
            }
            Dock::Bottom => {
                style.inset = taffy::geometry::Rect {
                    top: LengthPercentageAuto::AUTO,
                    left: LengthPercentageAuto::from_length(0.0),
                    right: LengthPercentageAuto::from_length(0.0),
                    bottom: LengthPercentageAuto::from_length(0.0),
                };
            }
            Dock::Left => {
                style.inset = taffy::geometry::Rect {
                    top: LengthPercentageAuto::from_length(0.0),
                    left: LengthPercentageAuto::from_length(0.0),
                    right: LengthPercentageAuto::AUTO,
                    bottom: LengthPercentageAuto::from_length(0.0),
                };
            }
            Dock::Right => {
                style.inset = taffy::geometry::Rect {
                    top: LengthPercentageAuto::from_length(0.0),
                    left: LengthPercentageAuto::AUTO,
                    right: LengthPercentageAuto::from_length(0.0),
                    bottom: LengthPercentageAuto::from_length(0.0),
                };
            }
        }
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::scalar::{Scalar, ScalarBox};
    use crate::css::styles::{Border, BorderKind, Dock, LayoutDirection, Overflow, Styles};

    const VIEWPORT: taffy::geometry::Size<f32> = taffy::geometry::Size {
        width: 80.0,
        height: 24.0,
    };

    const VP_TUPLE: (u16, u16) = (80, 24);

    // -----------------------------------------------------------------------
    // resolve_scalar
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_cells() {
        let s = Scalar::cells(10.0);
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::from_length(10.0));
    }

    #[test]
    fn resolve_percent() {
        let s = Scalar::percent(50.0);
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::from_percent(0.5));
    }

    #[test]
    fn resolve_vw() {
        let s = Scalar::vw(50.0);
        let result = resolve_scalar(&s, VIEWPORT);
        // 50% of 80 = 40
        assert_eq!(result, LengthPercentageAuto::from_length(40.0));
    }

    #[test]
    fn resolve_vh() {
        let s = Scalar::vh(100.0);
        let result = resolve_scalar(&s, VIEWPORT);
        // 100% of 24 = 24
        assert_eq!(result, LengthPercentageAuto::from_length(24.0));
    }

    #[test]
    fn resolve_auto() {
        let s = Scalar::auto();
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::AUTO);
    }

    #[test]
    fn resolve_fr_as_auto() {
        let s = Scalar::fr(2.0);
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::AUTO);
    }

    // -----------------------------------------------------------------------
    // resolve_scalar_definite
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_definite_cells() {
        let s = Scalar::cells(5.0);
        let result = resolve_scalar_definite(&s, VIEWPORT);
        assert_eq!(result, LengthPercentage::from_length(5.0));
    }

    #[test]
    fn resolve_definite_percent() {
        let s = Scalar::percent(25.0);
        let result = resolve_scalar_definite(&s, VIEWPORT);
        assert_eq!(result, LengthPercentage::from_percent(0.25));
    }

    #[test]
    fn resolve_definite_auto_becomes_zero() {
        let s = Scalar::auto();
        let result = resolve_scalar_definite(&s, VIEWPORT);
        assert_eq!(result, LengthPercentage::ZERO);
    }

    #[test]
    fn resolve_definite_vw() {
        let s = Scalar::vw(10.0);
        let result = resolve_scalar_definite(&s, VIEWPORT);
        assert_eq!(result, LengthPercentage::from_length(8.0)); // 10% of 80
    }

    #[test]
    fn resolve_definite_vh() {
        let s = Scalar::vh(50.0);
        let result = resolve_scalar_definite(&s, VIEWPORT);
        assert_eq!(result, LengthPercentage::from_length(12.0)); // 50% of 24
    }

    // -----------------------------------------------------------------------
    // resolve_scalar_box
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_box_uniform() {
        let b = ScalarBox::all(Scalar::cells(2.0));
        let result = resolve_scalar_box(&b, VIEWPORT);
        let expected = LengthPercentageAuto::from_length(2.0);
        assert_eq!(result.top, expected);
        assert_eq!(result.right, expected);
        assert_eq!(result.bottom, expected);
        assert_eq!(result.left, expected);
    }

    #[test]
    fn resolve_box_mixed_units() {
        let b = ScalarBox::new(
            Scalar::cells(1.0),
            Scalar::percent(50.0),
            Scalar::vw(10.0),
            Scalar::auto(),
        );
        let result = resolve_scalar_box(&b, VIEWPORT);
        assert_eq!(result.top, LengthPercentageAuto::from_length(1.0));
        assert_eq!(result.right, LengthPercentageAuto::from_percent(0.5));
        assert_eq!(result.bottom, LengthPercentageAuto::from_length(8.0)); // 10% of 80
        assert_eq!(result.left, LengthPercentageAuto::AUTO);
    }

    // -----------------------------------------------------------------------
    // resolve_styles
    // -----------------------------------------------------------------------

    #[test]
    fn styles_default_is_flex_column() {
        let styles = Styles::new();
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.display, Display::Flex);
        assert_eq!(taffy_style.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn styles_display_none() {
        let mut styles = Styles::new();
        styles.display = Some(crate::css::styles::Display::None);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.display, Display::None);
    }

    #[test]
    fn styles_horizontal_layout() {
        let mut styles = Styles::new();
        styles.layout = Some(LayoutDirection::Horizontal);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.display, Display::Flex);
        assert_eq!(taffy_style.flex_direction, FlexDirection::Row);
    }

    #[test]
    fn styles_grid_layout() {
        let mut styles = Styles::new();
        styles.layout = Some(LayoutDirection::Grid);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.display, Display::Grid);
    }

    #[test]
    fn styles_sizing() {
        let mut styles = Styles::new();
        styles.width = Some(Scalar::cells(40.0));
        styles.height = Some(Scalar::percent(50.0));
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.size.width, Dimension::from_length(40.0));
        assert_eq!(taffy_style.size.height, Dimension::from_percent(0.5));
    }

    #[test]
    fn styles_min_max_sizing() {
        let mut styles = Styles::new();
        styles.min_width = Some(Scalar::cells(10.0));
        styles.max_height = Some(Scalar::vh(100.0));
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.min_size.width, Dimension::from_length(10.0));
        assert_eq!(
            taffy_style.max_size.height,
            Dimension::from_length(24.0) // 100% of 24
        );
    }

    #[test]
    fn styles_margin() {
        let mut styles = Styles::new();
        styles.margin = Some(ScalarBox::all(Scalar::cells(2.0)));
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(
            taffy_style.margin.top,
            LengthPercentageAuto::from_length(2.0)
        );
        assert_eq!(
            taffy_style.margin.left,
            LengthPercentageAuto::from_length(2.0)
        );
    }

    #[test]
    fn styles_padding() {
        let mut styles = Styles::new();
        styles.padding = Some(ScalarBox::symmetric(Scalar::cells(1.0), Scalar::cells(3.0)));
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(
            taffy_style.padding.top,
            LengthPercentage::from_length(1.0)
        );
        assert_eq!(
            taffy_style.padding.left,
            LengthPercentage::from_length(3.0)
        );
    }

    #[test]
    fn styles_overflow() {
        let mut styles = Styles::new();
        styles.overflow_x = Some(Overflow::Scroll);
        styles.overflow_y = Some(Overflow::Hidden);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.overflow.x, taffy::style::Overflow::Scroll);
        assert_eq!(taffy_style.overflow.y, taffy::style::Overflow::Hidden);
    }

    #[test]
    fn styles_overflow_auto_becomes_scroll() {
        let mut styles = Styles::new();
        styles.overflow_x = Some(Overflow::Auto);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.overflow.x, taffy::style::Overflow::Scroll);
    }

    #[test]
    fn styles_overflow_default_visible() {
        let styles = Styles::new();
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.overflow.x, taffy::style::Overflow::Visible);
        assert_eq!(taffy_style.overflow.y, taffy::style::Overflow::Visible);
    }

    #[test]
    fn styles_border_thin() {
        let mut styles = Styles::new();
        styles.border = Some(Border {
            kind: BorderKind::Thin,
            color: None,
        });
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(
            taffy_style.border.top,
            LengthPercentage::from_length(1.0)
        );
        assert_eq!(
            taffy_style.border.left,
            LengthPercentage::from_length(1.0)
        );
    }

    #[test]
    fn styles_border_none_kind_no_border() {
        let mut styles = Styles::new();
        styles.border = Some(Border {
            kind: BorderKind::None,
            color: None,
        });
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.border.top, LengthPercentage::ZERO);
    }

    #[test]
    fn styles_dock_top() {
        let mut styles = Styles::new();
        styles.dock = Some(Dock::Top);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.position, Position::Absolute);
        assert_eq!(
            taffy_style.inset.top,
            LengthPercentageAuto::from_length(0.0)
        );
        assert_eq!(taffy_style.inset.bottom, LengthPercentageAuto::AUTO);
    }

    #[test]
    fn styles_dock_bottom() {
        let mut styles = Styles::new();
        styles.dock = Some(Dock::Bottom);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.position, Position::Absolute);
        assert_eq!(taffy_style.inset.top, LengthPercentageAuto::AUTO);
        assert_eq!(
            taffy_style.inset.bottom,
            LengthPercentageAuto::from_length(0.0)
        );
    }

    #[test]
    fn styles_dock_left() {
        let mut styles = Styles::new();
        styles.dock = Some(Dock::Left);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.position, Position::Absolute);
        assert_eq!(
            taffy_style.inset.left,
            LengthPercentageAuto::from_length(0.0)
        );
        assert_eq!(taffy_style.inset.right, LengthPercentageAuto::AUTO);
    }

    #[test]
    fn styles_dock_right() {
        let mut styles = Styles::new();
        styles.dock = Some(Dock::Right);
        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.position, Position::Absolute);
        assert_eq!(taffy_style.inset.left, LengthPercentageAuto::AUTO);
        assert_eq!(
            taffy_style.inset.right,
            LengthPercentageAuto::from_length(0.0)
        );
    }

    #[test]
    fn styles_full_combination() {
        let mut styles = Styles::new();
        styles.display = Some(crate::css::styles::Display::Block);
        styles.layout = Some(LayoutDirection::Horizontal);
        styles.width = Some(Scalar::percent(100.0));
        styles.height = Some(Scalar::cells(3.0));
        styles.padding = Some(ScalarBox::all(Scalar::cells(1.0)));
        styles.border = Some(Border {
            kind: BorderKind::Heavy,
            color: Some("red".into()),
        });

        let taffy_style = resolve_styles(&styles, VP_TUPLE);
        assert_eq!(taffy_style.display, Display::Flex);
        assert_eq!(taffy_style.flex_direction, FlexDirection::Row);
        assert_eq!(taffy_style.size.width, Dimension::from_percent(1.0));
        assert_eq!(taffy_style.size.height, Dimension::from_length(3.0));
        assert_eq!(
            taffy_style.padding.top,
            LengthPercentage::from_length(1.0)
        );
        assert_eq!(
            taffy_style.border.top,
            LengthPercentage::from_length(1.0)
        );
    }

    #[test]
    fn resolve_zero_cells() {
        let s = Scalar::cells(0.0);
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::from_length(0.0));
    }

    #[test]
    fn resolve_negative_cells() {
        let s = Scalar::cells(-5.0);
        let result = resolve_scalar(&s, VIEWPORT);
        assert_eq!(result, LengthPercentageAuto::from_length(-5.0));
    }

    #[test]
    fn resolve_vw_zero_viewport() {
        let viewport = taffy::geometry::Size {
            width: 0.0,
            height: 0.0,
        };
        let s = Scalar::vw(50.0);
        let result = resolve_scalar(&s, viewport);
        assert_eq!(result, LengthPercentageAuto::from_length(0.0));
    }
}
