//! Styles struct with typed Option<T> fields for all CSS properties.
//!
//! This is the central style representation. Every CSS property has a typed
//! `Option<T>` field. `None` means "not set" (inherits from parent or uses default).

use crate::css::scalar::{Scalar, ScalarBox};

/// Text alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Display property options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    None,
}

/// Visibility property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

/// Overflow behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Hidden,
    Scroll,
    Auto,
}

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
    Grid,
}

/// Dock position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dock {
    Top,
    Right,
    Bottom,
    Left,
}

/// Border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderKind {
    None,
    Thin,
    Heavy,
    Double,
    Round,
    Ascii,
}

/// A border with kind and optional color.
#[derive(Debug, Clone, PartialEq)]
pub struct Border {
    pub kind: BorderKind,
    pub color: Option<String>,
}

/// Text style flags (bold, italic, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextStyleFlags {
    pub bold: Option<bool>,
    pub dim: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub reverse: Option<bool>,
}

/// All CSS properties for a node. Each field is `Option<T>` — None means unset (inherit).
///
/// Phase 1 properties: layout-critical subset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Styles {
    // Display & Layout
    pub display: Option<Display>,
    pub visibility: Option<Visibility>,
    pub layout: Option<LayoutDirection>,
    pub dock: Option<Dock>,
    pub overflow_x: Option<Overflow>,
    pub overflow_y: Option<Overflow>,

    // Sizing
    pub width: Option<Scalar>,
    pub height: Option<Scalar>,
    pub min_width: Option<Scalar>,
    pub min_height: Option<Scalar>,
    pub max_width: Option<Scalar>,
    pub max_height: Option<Scalar>,

    // Spacing
    pub margin: Option<ScalarBox>,
    pub padding: Option<ScalarBox>,

    // Colors
    pub color: Option<String>,
    pub background: Option<String>,

    // Text
    pub text_align: Option<TextAlign>,
    pub text_style: Option<TextStyleFlags>,

    // Border
    pub border: Option<Border>,
}

impl Styles {
    /// Create a new `Styles` with all fields set to `None`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge `other` on top of `self`. For each field, if `other` has a value (`Some`),
    /// use it; otherwise keep `self`'s value. This implements CSS cascade: lower-specificity
    /// styles are `self`, higher-specificity styles are `other`.
    pub fn merge(&self, other: &Styles) -> Styles {
        /// Helper: pick `other` if set, otherwise keep `base`.
        fn merge_opt<T: Clone>(base: &Option<T>, other: &Option<T>) -> Option<T> {
            if other.is_some() {
                other.clone()
            } else {
                base.clone()
            }
        }

        Styles {
            display: merge_opt(&self.display, &other.display),
            visibility: merge_opt(&self.visibility, &other.visibility),
            layout: merge_opt(&self.layout, &other.layout),
            dock: merge_opt(&self.dock, &other.dock),
            overflow_x: merge_opt(&self.overflow_x, &other.overflow_x),
            overflow_y: merge_opt(&self.overflow_y, &other.overflow_y),

            width: merge_opt(&self.width, &other.width),
            height: merge_opt(&self.height, &other.height),
            min_width: merge_opt(&self.min_width, &other.min_width),
            min_height: merge_opt(&self.min_height, &other.min_height),
            max_width: merge_opt(&self.max_width, &other.max_width),
            max_height: merge_opt(&self.max_height, &other.max_height),

            margin: merge_opt(&self.margin, &other.margin),
            padding: merge_opt(&self.padding, &other.padding),

            color: merge_opt(&self.color, &other.color),
            background: merge_opt(&self.background, &other.background),

            text_align: merge_opt(&self.text_align, &other.text_align),
            text_style: merge_opt(&self.text_style, &other.text_style),

            border: merge_opt(&self.border, &other.border),
        }
    }

    /// Returns `true` if all fields are `None` (no properties set).
    pub fn is_empty(&self) -> bool {
        self.display.is_none()
            && self.visibility.is_none()
            && self.layout.is_none()
            && self.dock.is_none()
            && self.overflow_x.is_none()
            && self.overflow_y.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.min_width.is_none()
            && self.min_height.is_none()
            && self.max_width.is_none()
            && self.max_height.is_none()
            && self.margin.is_none()
            && self.padding.is_none()
            && self.color.is_none()
            && self.background.is_none()
            && self.text_align.is_none()
            && self.text_style.is_none()
            && self.border.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::scalar::{Scalar, ScalarBox};

    #[test]
    fn new_is_empty() {
        let s = Styles::new();
        assert!(s.is_empty());
    }

    #[test]
    fn default_is_empty() {
        let s = Styles::default();
        assert!(s.is_empty());
    }

    #[test]
    fn not_empty_when_field_set() {
        let mut s = Styles::new();
        s.color = Some("red".into());
        assert!(!s.is_empty());
    }

    #[test]
    fn merge_empty_with_empty() {
        let a = Styles::new();
        let b = Styles::new();
        let merged = a.merge(&b);
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_keeps_base_when_other_empty() {
        let mut base = Styles::new();
        base.color = Some("red".into());
        base.display = Some(Display::Block);

        let other = Styles::new();
        let merged = base.merge(&other);

        assert_eq!(merged.color, Some("red".into()));
        assert_eq!(merged.display, Some(Display::Block));
    }

    #[test]
    fn merge_other_overrides_base() {
        let mut base = Styles::new();
        base.color = Some("red".into());
        base.background = Some("white".into());

        let mut other = Styles::new();
        other.color = Some("blue".into());

        let merged = base.merge(&other);
        assert_eq!(merged.color, Some("blue".into()));
        assert_eq!(merged.background, Some("white".into()));
    }

    #[test]
    fn merge_partial_override() {
        let mut base = Styles::new();
        base.display = Some(Display::Block);
        base.width = Some(Scalar::percent(50.0));
        base.color = Some("red".into());
        base.text_align = Some(TextAlign::Left);

        let mut other = Styles::new();
        other.color = Some("green".into());
        other.text_align = Some(TextAlign::Center);
        other.height = Some(Scalar::cells(10.0));

        let merged = base.merge(&other);

        // Kept from base (not overridden)
        assert_eq!(merged.display, Some(Display::Block));
        assert_eq!(merged.width, Some(Scalar::percent(50.0)));

        // Overridden by other
        assert_eq!(merged.color, Some("green".into()));
        assert_eq!(merged.text_align, Some(TextAlign::Center));

        // New from other
        assert_eq!(merged.height, Some(Scalar::cells(10.0)));

        // Still unset
        assert!(merged.background.is_none());
    }

    #[test]
    fn merge_all_fields() {
        let mut base = Styles::new();
        base.display = Some(Display::Block);
        base.visibility = Some(Visibility::Visible);
        base.layout = Some(LayoutDirection::Vertical);
        base.dock = Some(Dock::Top);
        base.overflow_x = Some(Overflow::Hidden);
        base.overflow_y = Some(Overflow::Scroll);
        base.width = Some(Scalar::cells(10.0));
        base.height = Some(Scalar::cells(20.0));
        base.min_width = Some(Scalar::cells(5.0));
        base.min_height = Some(Scalar::cells(5.0));
        base.max_width = Some(Scalar::cells(100.0));
        base.max_height = Some(Scalar::cells(100.0));
        base.margin = Some(ScalarBox::all(Scalar::cells(1.0)));
        base.padding = Some(ScalarBox::all(Scalar::cells(2.0)));
        base.color = Some("red".into());
        base.background = Some("blue".into());
        base.text_align = Some(TextAlign::Left);
        base.text_style = Some(TextStyleFlags {
            bold: Some(true),
            ..Default::default()
        });
        base.border = Some(Border {
            kind: BorderKind::Thin,
            color: None,
        });

        // Override everything with other
        let mut other = Styles::new();
        other.display = Some(Display::None);
        other.visibility = Some(Visibility::Hidden);
        other.layout = Some(LayoutDirection::Horizontal);
        other.dock = Some(Dock::Bottom);
        other.overflow_x = Some(Overflow::Auto);
        other.overflow_y = Some(Overflow::Auto);
        other.width = Some(Scalar::percent(50.0));
        other.height = Some(Scalar::percent(50.0));
        other.min_width = Some(Scalar::cells(0.0));
        other.min_height = Some(Scalar::cells(0.0));
        other.max_width = Some(Scalar::auto());
        other.max_height = Some(Scalar::auto());
        other.margin = Some(ScalarBox::all(Scalar::cells(0.0)));
        other.padding = Some(ScalarBox::all(Scalar::cells(0.0)));
        other.color = Some("green".into());
        other.background = Some("yellow".into());
        other.text_align = Some(TextAlign::Right);
        other.text_style = Some(TextStyleFlags {
            italic: Some(true),
            ..Default::default()
        });
        other.border = Some(Border {
            kind: BorderKind::Heavy,
            color: Some("red".into()),
        });

        let merged = base.merge(&other);

        assert_eq!(merged.display, Some(Display::None));
        assert_eq!(merged.visibility, Some(Visibility::Hidden));
        assert_eq!(merged.layout, Some(LayoutDirection::Horizontal));
        assert_eq!(merged.dock, Some(Dock::Bottom));
        assert_eq!(merged.overflow_x, Some(Overflow::Auto));
        assert_eq!(merged.overflow_y, Some(Overflow::Auto));
        assert_eq!(merged.width, Some(Scalar::percent(50.0)));
        assert_eq!(merged.height, Some(Scalar::percent(50.0)));
        assert_eq!(merged.min_width, Some(Scalar::cells(0.0)));
        assert_eq!(merged.min_height, Some(Scalar::cells(0.0)));
        assert_eq!(merged.max_width, Some(Scalar::auto()));
        assert_eq!(merged.max_height, Some(Scalar::auto()));
        assert_eq!(merged.margin, Some(ScalarBox::all(Scalar::cells(0.0))));
        assert_eq!(merged.padding, Some(ScalarBox::all(Scalar::cells(0.0))));
        assert_eq!(merged.color, Some("green".into()));
        assert_eq!(merged.background, Some("yellow".into()));
        assert_eq!(merged.text_align, Some(TextAlign::Right));
        assert_eq!(
            merged.text_style,
            Some(TextStyleFlags {
                italic: Some(true),
                ..Default::default()
            })
        );
        assert_eq!(
            merged.border,
            Some(Border {
                kind: BorderKind::Heavy,
                color: Some("red".into()),
            })
        );
    }

    #[test]
    fn merge_is_not_commutative() {
        let mut a = Styles::new();
        a.color = Some("red".into());

        let mut b = Styles::new();
        b.color = Some("blue".into());

        // a.merge(&b) => blue wins (other overrides)
        assert_eq!(a.merge(&b).color, Some("blue".into()));
        // b.merge(&a) => red wins (other overrides)
        assert_eq!(b.merge(&a).color, Some("red".into()));
    }

    #[test]
    fn merge_chained_cascade() {
        // Simulate three layers of cascade: default -> widget -> user
        let mut default_styles = Styles::new();
        default_styles.display = Some(Display::Block);
        default_styles.color = Some("white".into());
        default_styles.background = Some("black".into());

        let mut widget_styles = Styles::new();
        widget_styles.color = Some("gray".into());
        widget_styles.padding = Some(ScalarBox::all(Scalar::cells(1.0)));

        let mut user_styles = Styles::new();
        user_styles.color = Some("red".into());

        let result = default_styles.merge(&widget_styles).merge(&user_styles);

        assert_eq!(result.display, Some(Display::Block)); // from default
        assert_eq!(result.color, Some("red".into())); // from user (highest)
        assert_eq!(result.background, Some("black".into())); // from default
        assert_eq!(result.padding, Some(ScalarBox::all(Scalar::cells(1.0)))); // from widget
    }

    #[test]
    fn text_style_flags_default() {
        let flags = TextStyleFlags::default();
        assert!(flags.bold.is_none());
        assert!(flags.dim.is_none());
        assert!(flags.italic.is_none());
        assert!(flags.underline.is_none());
        assert!(flags.strikethrough.is_none());
        assert!(flags.reverse.is_none());
    }

    #[test]
    fn border_equality() {
        let a = Border {
            kind: BorderKind::Thin,
            color: Some("red".into()),
        };
        let b = Border {
            kind: BorderKind::Thin,
            color: Some("red".into()),
        };
        let c = Border {
            kind: BorderKind::Heavy,
            color: Some("red".into()),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
