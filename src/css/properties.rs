//! Property parsing: token values → typed property values.
//!
//! Parses string/token-based CSS declaration values into the typed fields
//! on [`crate::css::styles::Styles`].

use crate::css::model::DeclarationValue;
use crate::css::scalar::{Scalar, ScalarBox};
use crate::css::styles::*;

/// Errors from property parsing.
#[derive(Debug, thiserror::Error)]
pub enum PropertyError {
    #[error("unknown property: {0}")]
    UnknownProperty(String),
    #[error("invalid value for {property}: {message}")]
    InvalidValue { property: String, message: String },
}

/// Parse a single declaration value into a [`Scalar`].
pub fn parse_scalar(value: &DeclarationValue) -> Result<Scalar, PropertyError> {
    match value {
        DeclarationValue::Number(n) => Ok(Scalar::cells(*n)),
        DeclarationValue::Dimension(n, unit) => match unit.as_str() {
            "fr" => Ok(Scalar::fr(*n)),
            "%" => Ok(Scalar::percent(*n)),
            "vw" => Ok(Scalar::vw(*n)),
            "vh" => Ok(Scalar::vh(*n)),
            other => Err(PropertyError::InvalidValue {
                property: "scalar".into(),
                message: format!("unknown unit: {other}"),
            }),
        },
        DeclarationValue::Ident(name) if name.eq_ignore_ascii_case("auto") => {
            Ok(Scalar::auto())
        }
        other => Err(PropertyError::InvalidValue {
            property: "scalar".into(),
            message: format!("expected number, dimension, or 'auto', got: {other:?}"),
        }),
    }
}

/// Parse 1-4 scalar values into a [`ScalarBox`] (CSS shorthand).
///
/// - 1 value: all sides
/// - 2 values: vertical, horizontal
/// - 3 values: top, horizontal, bottom
/// - 4 values: top, right, bottom, left
pub fn parse_scalar_box(values: &[DeclarationValue]) -> Result<ScalarBox, PropertyError> {
    match values.len() {
        1 => {
            let v = parse_scalar(&values[0])?;
            Ok(ScalarBox::all(v))
        }
        2 => {
            let vertical = parse_scalar(&values[0])?;
            let horizontal = parse_scalar(&values[1])?;
            Ok(ScalarBox::symmetric(vertical, horizontal))
        }
        3 => {
            let top = parse_scalar(&values[0])?;
            let horizontal = parse_scalar(&values[1])?;
            let bottom = parse_scalar(&values[2])?;
            Ok(ScalarBox::new(top, horizontal, bottom, horizontal))
        }
        4 => {
            let top = parse_scalar(&values[0])?;
            let right = parse_scalar(&values[1])?;
            let bottom = parse_scalar(&values[2])?;
            let left = parse_scalar(&values[3])?;
            Ok(ScalarBox::new(top, right, bottom, left))
        }
        n => Err(PropertyError::InvalidValue {
            property: "margin/padding".into(),
            message: format!("expected 1-4 values, got {n}"),
        }),
    }
}

/// Extract a single identifier from values, returning an error using the given property name.
fn require_single_ident<'a>(
    values: &'a [DeclarationValue],
    property: &str,
) -> Result<&'a str, PropertyError> {
    if values.len() != 1 {
        return Err(PropertyError::InvalidValue {
            property: property.into(),
            message: format!("expected 1 value, got {}", values.len()),
        });
    }
    match &values[0] {
        DeclarationValue::Ident(name) => Ok(name.as_str()),
        other => Err(PropertyError::InvalidValue {
            property: property.into(),
            message: format!("expected identifier, got: {other:?}"),
        }),
    }
}

/// Extract a color value (ident or hex color) from values.
fn require_color_value(
    values: &[DeclarationValue],
    property: &str,
) -> Result<String, PropertyError> {
    if values.len() != 1 {
        return Err(PropertyError::InvalidValue {
            property: property.into(),
            message: format!("expected 1 color value, got {}", values.len()),
        });
    }
    match &values[0] {
        DeclarationValue::Ident(name) => Ok(name.clone()),
        DeclarationValue::Color(hex) => Ok(format!("#{hex}")),
        other => Err(PropertyError::InvalidValue {
            property: property.into(),
            message: format!("expected color name or hex color, got: {other:?}"),
        }),
    }
}

/// Parse an overflow ident.
fn parse_overflow(name: &str, property: &str) -> Result<Overflow, PropertyError> {
    match name {
        "hidden" => Ok(Overflow::Hidden),
        "scroll" => Ok(Overflow::Scroll),
        "auto" => Ok(Overflow::Auto),
        other => Err(PropertyError::InvalidValue {
            property: property.into(),
            message: format!("expected hidden|scroll|auto, got: {other}"),
        }),
    }
}

/// Parse border values: `<kind>` or `<kind> <color>`.
fn parse_border(values: &[DeclarationValue]) -> Result<Border, PropertyError> {
    if values.is_empty() {
        return Err(PropertyError::InvalidValue {
            property: "border".into(),
            message: "expected at least 1 value for border".into(),
        });
    }

    let kind_str = match &values[0] {
        DeclarationValue::Ident(name) => name.as_str(),
        other => {
            return Err(PropertyError::InvalidValue {
                property: "border".into(),
                message: format!("expected border kind identifier, got: {other:?}"),
            });
        }
    };

    let kind = match kind_str {
        "none" => BorderKind::None,
        "thin" => BorderKind::Thin,
        "heavy" => BorderKind::Heavy,
        "double" => BorderKind::Double,
        "round" => BorderKind::Round,
        "ascii" => BorderKind::Ascii,
        other => {
            return Err(PropertyError::InvalidValue {
                property: "border".into(),
                message: format!("unknown border kind: {other}"),
            });
        }
    };

    let color = if values.len() > 1 {
        match &values[1] {
            DeclarationValue::Ident(name) => Some(name.clone()),
            DeclarationValue::Color(hex) => Some(format!("#{hex}")),
            other => {
                return Err(PropertyError::InvalidValue {
                    property: "border".into(),
                    message: format!("expected color for border, got: {other:?}"),
                });
            }
        }
    } else {
        None
    };

    Ok(Border { kind, color })
}

/// Parse text-style values: one or more of bold, dim, italic, underline, strikethrough, reverse.
fn parse_text_style(values: &[DeclarationValue]) -> Result<TextStyleFlags, PropertyError> {
    let mut flags = TextStyleFlags::default();

    for value in values {
        let name = match value {
            DeclarationValue::Ident(name) => name.as_str(),
            other => {
                return Err(PropertyError::InvalidValue {
                    property: "text-style".into(),
                    message: format!("expected text style identifier, got: {other:?}"),
                });
            }
        };
        match name {
            "bold" => flags.bold = Some(true),
            "dim" => flags.dim = Some(true),
            "italic" => flags.italic = Some(true),
            "underline" => flags.underline = Some(true),
            "strikethrough" => flags.strikethrough = Some(true),
            "reverse" => flags.reverse = Some(true),
            "none" => {
                // Reset all flags
                flags.bold = Some(false);
                flags.dim = Some(false);
                flags.italic = Some(false);
                flags.underline = Some(false);
                flags.strikethrough = Some(false);
                flags.reverse = Some(false);
            }
            other => {
                return Err(PropertyError::InvalidValue {
                    property: "text-style".into(),
                    message: format!("unknown text style: {other}"),
                });
            }
        }
    }

    Ok(flags)
}

/// Apply a CSS declaration (property name + values) to a mutable [`Styles`].
///
/// Handles all Phase 1 properties. Returns an error for unknown properties
/// or invalid values.
pub fn apply_declaration(
    styles: &mut Styles,
    property: &str,
    values: &[DeclarationValue],
) -> Result<(), PropertyError> {
    match property {
        // Display & Layout
        "display" => {
            let name = require_single_ident(values, "display")?;
            styles.display = Some(match name {
                "block" => Display::Block,
                "none" => Display::None,
                other => {
                    return Err(PropertyError::InvalidValue {
                        property: "display".into(),
                        message: format!("expected block|none, got: {other}"),
                    });
                }
            });
        }
        "visibility" => {
            let name = require_single_ident(values, "visibility")?;
            styles.visibility = Some(match name {
                "visible" => Visibility::Visible,
                "hidden" => Visibility::Hidden,
                other => {
                    return Err(PropertyError::InvalidValue {
                        property: "visibility".into(),
                        message: format!("expected visible|hidden, got: {other}"),
                    });
                }
            });
        }
        "layout" => {
            let name = require_single_ident(values, "layout")?;
            styles.layout = Some(match name {
                "vertical" => LayoutDirection::Vertical,
                "horizontal" => LayoutDirection::Horizontal,
                "grid" => LayoutDirection::Grid,
                other => {
                    return Err(PropertyError::InvalidValue {
                        property: "layout".into(),
                        message: format!("expected vertical|horizontal|grid, got: {other}"),
                    });
                }
            });
        }
        "dock" => {
            let name = require_single_ident(values, "dock")?;
            styles.dock = Some(match name {
                "top" => Dock::Top,
                "right" => Dock::Right,
                "bottom" => Dock::Bottom,
                "left" => Dock::Left,
                other => {
                    return Err(PropertyError::InvalidValue {
                        property: "dock".into(),
                        message: format!("expected top|right|bottom|left, got: {other}"),
                    });
                }
            });
        }
        "overflow" => {
            let name = require_single_ident(values, "overflow")?;
            let overflow = parse_overflow(name, "overflow")?;
            styles.overflow_x = Some(overflow);
            styles.overflow_y = Some(overflow);
        }
        "overflow-x" => {
            let name = require_single_ident(values, "overflow-x")?;
            styles.overflow_x = Some(parse_overflow(name, "overflow-x")?);
        }
        "overflow-y" => {
            let name = require_single_ident(values, "overflow-y")?;
            styles.overflow_y = Some(parse_overflow(name, "overflow-y")?);
        }

        // Sizing
        "width" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "width".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.width = Some(parse_scalar(&values[0])?);
        }
        "height" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "height".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.height = Some(parse_scalar(&values[0])?);
        }
        "min-width" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "min-width".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.min_width = Some(parse_scalar(&values[0])?);
        }
        "min-height" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "min-height".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.min_height = Some(parse_scalar(&values[0])?);
        }
        "max-width" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "max-width".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.max_width = Some(parse_scalar(&values[0])?);
        }
        "max-height" => {
            if values.len() != 1 {
                return Err(PropertyError::InvalidValue {
                    property: "max-height".into(),
                    message: format!("expected 1 value, got {}", values.len()),
                });
            }
            styles.max_height = Some(parse_scalar(&values[0])?);
        }

        // Spacing
        "margin" => {
            styles.margin = Some(parse_scalar_box(values)?);
        }
        "padding" => {
            styles.padding = Some(parse_scalar_box(values)?);
        }

        // Colors
        "color" => {
            styles.color = Some(require_color_value(values, "color")?);
        }
        "background" => {
            styles.background = Some(require_color_value(values, "background")?);
        }

        // Text
        "text-align" => {
            let name = require_single_ident(values, "text-align")?;
            styles.text_align = Some(match name {
                "left" => TextAlign::Left,
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                other => {
                    return Err(PropertyError::InvalidValue {
                        property: "text-align".into(),
                        message: format!("expected left|center|right, got: {other}"),
                    });
                }
            });
        }
        "text-style" => {
            styles.text_style = Some(parse_text_style(values)?);
        }

        // Border
        "border" => {
            styles.border = Some(parse_border(values)?);
        }

        // Unknown
        other => {
            return Err(PropertyError::UnknownProperty(other.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::model::DeclarationValue;
    use crate::css::scalar::{Scalar, ScalarBox};

    // ── parse_scalar ─────────────────────────────────────────────────

    #[test]
    fn parse_scalar_number() {
        let v = DeclarationValue::Number(10.0);
        let s = parse_scalar(&v).unwrap();
        assert_eq!(s, Scalar::cells(10.0));
    }

    #[test]
    fn parse_scalar_fr() {
        let v = DeclarationValue::Dimension(1.0, "fr".into());
        let s = parse_scalar(&v).unwrap();
        assert_eq!(s, Scalar::fr(1.0));
    }

    #[test]
    fn parse_scalar_percent() {
        let v = DeclarationValue::Dimension(50.0, "%".into());
        let s = parse_scalar(&v).unwrap();
        assert_eq!(s, Scalar::percent(50.0));
    }

    #[test]
    fn parse_scalar_vw() {
        let v = DeclarationValue::Dimension(100.0, "vw".into());
        let s = parse_scalar(&v).unwrap();
        assert_eq!(s, Scalar::vw(100.0));
    }

    #[test]
    fn parse_scalar_vh() {
        let v = DeclarationValue::Dimension(80.0, "vh".into());
        let s = parse_scalar(&v).unwrap();
        assert_eq!(s, Scalar::vh(80.0));
    }

    #[test]
    fn parse_scalar_auto() {
        let v = DeclarationValue::Ident("auto".into());
        let s = parse_scalar(&v).unwrap();
        assert!(s.is_auto());
    }

    #[test]
    fn parse_scalar_unknown_unit_err() {
        let v = DeclarationValue::Dimension(10.0, "em".into());
        assert!(parse_scalar(&v).is_err());
    }

    #[test]
    fn parse_scalar_color_err() {
        let v = DeclarationValue::Color("fff".into());
        assert!(parse_scalar(&v).is_err());
    }

    // ── parse_scalar_box ─────────────────────────────────────────────

    #[test]
    fn parse_scalar_box_one_value() {
        let values = vec![DeclarationValue::Number(5.0)];
        let b = parse_scalar_box(&values).unwrap();
        assert_eq!(b, ScalarBox::all(Scalar::cells(5.0)));
    }

    #[test]
    fn parse_scalar_box_two_values() {
        let values = vec![
            DeclarationValue::Number(1.0),
            DeclarationValue::Number(2.0),
        ];
        let b = parse_scalar_box(&values).unwrap();
        assert_eq!(
            b,
            ScalarBox::symmetric(Scalar::cells(1.0), Scalar::cells(2.0))
        );
    }

    #[test]
    fn parse_scalar_box_three_values() {
        let values = vec![
            DeclarationValue::Number(1.0),
            DeclarationValue::Number(2.0),
            DeclarationValue::Number(3.0),
        ];
        let b = parse_scalar_box(&values).unwrap();
        assert_eq!(
            b,
            ScalarBox::new(
                Scalar::cells(1.0),
                Scalar::cells(2.0),
                Scalar::cells(3.0),
                Scalar::cells(2.0),
            )
        );
    }

    #[test]
    fn parse_scalar_box_four_values() {
        let values = vec![
            DeclarationValue::Number(1.0),
            DeclarationValue::Number(2.0),
            DeclarationValue::Number(3.0),
            DeclarationValue::Number(4.0),
        ];
        let b = parse_scalar_box(&values).unwrap();
        assert_eq!(
            b,
            ScalarBox::new(
                Scalar::cells(1.0),
                Scalar::cells(2.0),
                Scalar::cells(3.0),
                Scalar::cells(4.0),
            )
        );
    }

    #[test]
    fn parse_scalar_box_zero_values_err() {
        let values: Vec<DeclarationValue> = vec![];
        assert!(parse_scalar_box(&values).is_err());
    }

    #[test]
    fn parse_scalar_box_five_values_err() {
        let values = vec![
            DeclarationValue::Number(1.0),
            DeclarationValue::Number(2.0),
            DeclarationValue::Number(3.0),
            DeclarationValue::Number(4.0),
            DeclarationValue::Number(5.0),
        ];
        assert!(parse_scalar_box(&values).is_err());
    }

    #[test]
    fn parse_scalar_box_mixed_units() {
        let values = vec![
            DeclarationValue::Number(1.0),
            DeclarationValue::Dimension(50.0, "%".into()),
        ];
        let b = parse_scalar_box(&values).unwrap();
        assert_eq!(b.top, Scalar::cells(1.0));
        assert_eq!(b.right, Scalar::percent(50.0));
        assert_eq!(b.bottom, Scalar::cells(1.0));
        assert_eq!(b.left, Scalar::percent(50.0));
    }

    // ── apply_declaration: display ───────────────────────────────────

    #[test]
    fn apply_display_block() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "display", &[DeclarationValue::Ident("block".into())]).unwrap();
        assert_eq!(s.display, Some(Display::Block));
    }

    #[test]
    fn apply_display_none() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "display", &[DeclarationValue::Ident("none".into())]).unwrap();
        assert_eq!(s.display, Some(Display::None));
    }

    #[test]
    fn apply_display_invalid() {
        let mut s = Styles::new();
        let result =
            apply_declaration(&mut s, "display", &[DeclarationValue::Ident("flex".into())]);
        assert!(result.is_err());
    }

    // ── apply_declaration: visibility ────────────────────────────────

    #[test]
    fn apply_visibility() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "visibility",
            &[DeclarationValue::Ident("hidden".into())],
        )
        .unwrap();
        assert_eq!(s.visibility, Some(Visibility::Hidden));
    }

    // ── apply_declaration: layout ────────────────────────────────────

    #[test]
    fn apply_layout_horizontal() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "layout",
            &[DeclarationValue::Ident("horizontal".into())],
        )
        .unwrap();
        assert_eq!(s.layout, Some(LayoutDirection::Horizontal));
    }

    #[test]
    fn apply_layout_grid() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "layout", &[DeclarationValue::Ident("grid".into())]).unwrap();
        assert_eq!(s.layout, Some(LayoutDirection::Grid));
    }

    // ── apply_declaration: dock ──────────────────────────────────────

    #[test]
    fn apply_dock() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "dock", &[DeclarationValue::Ident("bottom".into())]).unwrap();
        assert_eq!(s.dock, Some(Dock::Bottom));
    }

    // ── apply_declaration: overflow ──────────────────────────────────

    #[test]
    fn apply_overflow_shorthand() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "overflow",
            &[DeclarationValue::Ident("scroll".into())],
        )
        .unwrap();
        assert_eq!(s.overflow_x, Some(Overflow::Scroll));
        assert_eq!(s.overflow_y, Some(Overflow::Scroll));
    }

    #[test]
    fn apply_overflow_x() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "overflow-x",
            &[DeclarationValue::Ident("hidden".into())],
        )
        .unwrap();
        assert_eq!(s.overflow_x, Some(Overflow::Hidden));
        assert!(s.overflow_y.is_none());
    }

    #[test]
    fn apply_overflow_y() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "overflow-y",
            &[DeclarationValue::Ident("auto".into())],
        )
        .unwrap();
        assert_eq!(s.overflow_y, Some(Overflow::Auto));
        assert!(s.overflow_x.is_none());
    }

    // ── apply_declaration: sizing ────────────────────────────────────

    #[test]
    fn apply_width_cells() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "width", &[DeclarationValue::Number(80.0)]).unwrap();
        assert_eq!(s.width, Some(Scalar::cells(80.0)));
    }

    #[test]
    fn apply_width_percent() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "width",
            &[DeclarationValue::Dimension(50.0, "%".into())],
        )
        .unwrap();
        assert_eq!(s.width, Some(Scalar::percent(50.0)));
    }

    #[test]
    fn apply_height_fr() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "height",
            &[DeclarationValue::Dimension(1.0, "fr".into())],
        )
        .unwrap();
        assert_eq!(s.height, Some(Scalar::fr(1.0)));
    }

    #[test]
    fn apply_min_width() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "min-width", &[DeclarationValue::Number(10.0)]).unwrap();
        assert_eq!(s.min_width, Some(Scalar::cells(10.0)));
    }

    #[test]
    fn apply_max_height_auto() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "max-height",
            &[DeclarationValue::Ident("auto".into())],
        )
        .unwrap();
        assert!(s.max_height.unwrap().is_auto());
    }

    // ── apply_declaration: margin/padding ────────────────────────────

    #[test]
    fn apply_margin_shorthand() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "margin",
            &[DeclarationValue::Number(1.0), DeclarationValue::Number(2.0)],
        )
        .unwrap();
        assert_eq!(
            s.margin,
            Some(ScalarBox::symmetric(Scalar::cells(1.0), Scalar::cells(2.0)))
        );
    }

    #[test]
    fn apply_padding_single() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "padding", &[DeclarationValue::Number(3.0)]).unwrap();
        assert_eq!(s.padding, Some(ScalarBox::all(Scalar::cells(3.0))));
    }

    // ── apply_declaration: colors ────────────────────────────────────

    #[test]
    fn apply_color_ident() {
        let mut s = Styles::new();
        apply_declaration(&mut s, "color", &[DeclarationValue::Ident("red".into())]).unwrap();
        assert_eq!(s.color, Some("red".into()));
    }

    #[test]
    fn apply_color_hex() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "color",
            &[DeclarationValue::Color("ff0000".into())],
        )
        .unwrap();
        assert_eq!(s.color, Some("#ff0000".into()));
    }

    #[test]
    fn apply_background_hex() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "background",
            &[DeclarationValue::Color("fff".into())],
        )
        .unwrap();
        assert_eq!(s.background, Some("#fff".into()));
    }

    // ── apply_declaration: text ──────────────────────────────────────

    #[test]
    fn apply_text_align() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "text-align",
            &[DeclarationValue::Ident("center".into())],
        )
        .unwrap();
        assert_eq!(s.text_align, Some(TextAlign::Center));
    }

    #[test]
    fn apply_text_style_multiple() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "text-style",
            &[
                DeclarationValue::Ident("bold".into()),
                DeclarationValue::Ident("italic".into()),
            ],
        )
        .unwrap();
        let flags = s.text_style.unwrap();
        assert_eq!(flags.bold, Some(true));
        assert_eq!(flags.italic, Some(true));
        assert!(flags.dim.is_none());
        assert!(flags.underline.is_none());
    }

    #[test]
    fn apply_text_style_none() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "text-style",
            &[DeclarationValue::Ident("none".into())],
        )
        .unwrap();
        let flags = s.text_style.unwrap();
        assert_eq!(flags.bold, Some(false));
        assert_eq!(flags.italic, Some(false));
        assert_eq!(flags.underline, Some(false));
    }

    // ── apply_declaration: border ────────────────────────────────────

    #[test]
    fn apply_border_kind_only() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "border",
            &[DeclarationValue::Ident("heavy".into())],
        )
        .unwrap();
        let border = s.border.unwrap();
        assert_eq!(border.kind, BorderKind::Heavy);
        assert!(border.color.is_none());
    }

    #[test]
    fn apply_border_kind_and_color() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "border",
            &[
                DeclarationValue::Ident("thin".into()),
                DeclarationValue::Ident("red".into()),
            ],
        )
        .unwrap();
        let border = s.border.unwrap();
        assert_eq!(border.kind, BorderKind::Thin);
        assert_eq!(border.color, Some("red".into()));
    }

    #[test]
    fn apply_border_kind_and_hex_color() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "border",
            &[
                DeclarationValue::Ident("double".into()),
                DeclarationValue::Color("ff0000".into()),
            ],
        )
        .unwrap();
        let border = s.border.unwrap();
        assert_eq!(border.kind, BorderKind::Double);
        assert_eq!(border.color, Some("#ff0000".into()));
    }

    #[test]
    fn apply_border_none() {
        let mut s = Styles::new();
        apply_declaration(
            &mut s,
            "border",
            &[DeclarationValue::Ident("none".into())],
        )
        .unwrap();
        let border = s.border.unwrap();
        assert_eq!(border.kind, BorderKind::None);
        assert!(border.color.is_none());
    }

    // ── apply_declaration: unknown ───────────────────────────────────

    #[test]
    fn apply_unknown_property() {
        let mut s = Styles::new();
        let result = apply_declaration(
            &mut s,
            "font-family",
            &[DeclarationValue::Ident("monospace".into())],
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            PropertyError::UnknownProperty(name) => assert_eq!(name, "font-family"),
            other => panic!("expected UnknownProperty, got: {other:?}"),
        }
    }

    #[test]
    fn apply_width_multiple_values_err() {
        let mut s = Styles::new();
        let result = apply_declaration(
            &mut s,
            "width",
            &[DeclarationValue::Number(10.0), DeclarationValue::Number(20.0)],
        );
        assert!(result.is_err());
    }
}
