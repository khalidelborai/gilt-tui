//! CSS css! macro: parse CSS property declarations at compile time and generate Styles code.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitFloat, LitInt, LitStr, Result, Token};

// ---------------------------------------------------------------------------
// AST types
// ---------------------------------------------------------------------------

/// A single parsed CSS value token.
#[derive(Debug, Clone)]
pub(crate) enum CssValue {
    /// A bare identifier like `red`, `center`, `block`.
    Ident(String, Span),
    /// A number (integer).
    Integer(i64, Span),
    /// A number (float).
    Float(f64, Span),
    /// A number followed by `%`.
    Percent(f64, Span),
    /// A number followed by a unit identifier like `fr`, `vw`, `vh`.
    WithUnit(f64, String, Span),
    /// A string starting with `#` (color hex).
    Hash(String, Span),
    /// A quoted string literal.
    Str(String, Span),
}

impl CssValue {
    fn span(&self) -> Span {
        match self {
            CssValue::Ident(_, s)
            | CssValue::Integer(_, s)
            | CssValue::Float(_, s)
            | CssValue::Percent(_, s)
            | CssValue::WithUnit(_, _, s)
            | CssValue::Hash(_, s)
            | CssValue::Str(_, s) => *s,
        }
    }
}

/// A single CSS property declaration: `property-name: value1 value2;`
#[derive(Debug, Clone)]
pub(crate) struct CssDeclaration {
    /// The property name in kebab-case (e.g. "text-align").
    pub name: String,
    /// The span of the property name, for error reporting.
    pub name_span: Span,
    /// The parsed values after the colon.
    pub values: Vec<CssValue>,
}

/// The top-level input to the css! macro.
#[derive(Debug)]
struct CssInput {
    declarations: Vec<CssDeclaration>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl Parse for CssInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut declarations = Vec::new();
        while !input.is_empty() {
            declarations.push(parse_declaration(input)?);
        }
        Ok(CssInput { declarations })
    }
}

/// Parse a single CSS declaration: `property-name: values;`
pub(crate) fn parse_declaration(input: ParseStream) -> Result<CssDeclaration> {
    // Parse property name (kebab-case: ident - ident - ident ...).
    let first_ident: Ident = input.parse()?;
    let mut name = first_ident.to_string();
    let name_span = first_ident.span();

    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        let next: Ident = input.parse()?;
        name.push('-');
        name.push_str(&next.to_string());
    }

    // Parse `:` separator.
    input.parse::<Token![:]>()?;

    // Parse values until `;`.
    let mut values = Vec::new();
    while !input.peek(Token![;]) {
        values.push(parse_css_value(input)?);
    }
    input.parse::<Token![;]>()?;

    if values.is_empty() {
        return Err(Error::new(name_span, format!("property `{}` has no value", name)));
    }

    Ok(CssDeclaration {
        name,
        name_span,
        values,
    })
}

/// Parse a single CSS value token.
pub(crate) fn parse_css_value(input: ParseStream) -> Result<CssValue> {
    // Check for hash color: `#` followed by an identifier or integer.
    if input.peek(Token![#]) {
        let hash_token = input.parse::<Token![#]>()?;
        let span = hash_token.span;
        // The hex value can be an ident (like abc) or a literal int (like 1a1a2e).
        // We collect characters until we hit a `;` or whitespace.
        // Since proc-macro tokenization doesn't give us raw hex easily,
        // we parse either an ident or an int literal.
        let hex_str = if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            ident.to_string()
        } else if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            // After the int, there might be an ident suffix (e.g., 1a1a2e is tokenized as int + ident)
            let mut s = lit.to_string();
            // Check if there's an immediately following ident (no space)
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                s.push_str(&ident.to_string());
            }
            s
        } else {
            return Err(input.error("expected hex color value after `#`"));
        };
        return Ok(CssValue::Hash(format!("#{}", hex_str), span));
    }

    // Check for a string literal.
    if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        return Ok(CssValue::Str(lit.value(), lit.span()));
    }

    // Check for a float literal.
    if input.peek(LitFloat) {
        let lit: LitFloat = input.parse()?;
        let span = lit.span();
        let val: f64 = lit.base10_parse()?;

        // Check for unit suffix: `%`, `fr`, `vw`, `vh`.
        if input.peek(Token![%]) {
            input.parse::<Token![%]>()?;
            return Ok(CssValue::Percent(val, span));
        }
        if input.peek(Ident) {
            let unit: Ident = input.parse()?;
            return Ok(CssValue::WithUnit(val, unit.to_string(), span));
        }
        return Ok(CssValue::Float(val, span));
    }

    // Check for an integer literal.
    if input.peek(LitInt) {
        let lit: LitInt = input.parse()?;
        let span = lit.span();

        // LitInt may have a suffix like `1fr` — check the suffix.
        let suffix = lit.suffix();
        if !suffix.is_empty() {
            let val: f64 = lit.base10_digits().parse().map_err(|_| {
                Error::new(span, "invalid number")
            })?;
            return Ok(CssValue::WithUnit(val, suffix.to_string(), span));
        }

        let val: i64 = lit.base10_parse()?;

        // Check for unit suffix after the literal.
        if input.peek(Token![%]) {
            input.parse::<Token![%]>()?;
            return Ok(CssValue::Percent(val as f64, span));
        }
        if input.peek(Ident) {
            // Only consume if it looks like a unit (fr, vw, vh).
            let fork = input.fork();
            let ident: Ident = fork.parse()?;
            let unit_str = ident.to_string();
            if matches!(unit_str.as_str(), "fr" | "vw" | "vh" | "px") {
                input.parse::<Ident>()?; // consume from real stream
                return Ok(CssValue::WithUnit(val as f64, unit_str, span));
            }
        }
        return Ok(CssValue::Integer(val, span));
    }

    // Check for a bare identifier.
    if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        // Handle `auto` as a special scalar keyword.
        return Ok(CssValue::Ident(ident.to_string(), ident.span()));
    }

    // Check for a negative number: `-` followed by a number.
    if input.peek(Token![-]) {
        let neg_token = input.parse::<Token![-]>()?;
        let span = neg_token.span;
        if input.peek(LitFloat) {
            let lit: LitFloat = input.parse()?;
            let val: f64 = -lit.base10_parse::<f64>()?;
            if input.peek(Token![%]) {
                input.parse::<Token![%]>()?;
                return Ok(CssValue::Percent(val, span));
            }
            if input.peek(Ident) {
                let unit: Ident = input.parse()?;
                return Ok(CssValue::WithUnit(val, unit.to_string(), span));
            }
            return Ok(CssValue::Float(val, span));
        }
        if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            let val: i64 = -lit.base10_parse::<i64>()?;
            if input.peek(Token![%]) {
                input.parse::<Token![%]>()?;
                return Ok(CssValue::Percent(val as f64, span));
            }
            if input.peek(Ident) {
                let fork = input.fork();
                let ident: Ident = fork.parse()?;
                let unit_str = ident.to_string();
                if matches!(unit_str.as_str(), "fr" | "vw" | "vh" | "px") {
                    input.parse::<Ident>()?;
                    return Ok(CssValue::WithUnit(val as f64, unit_str, span));
                }
            }
            return Ok(CssValue::Integer(val, span));
        }
        return Err(Error::new(span, "expected a number after `-`"));
    }

    Err(input.error("unexpected token in CSS value"))
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

/// Convert a kebab-case property name to snake_case.
fn kebab_to_snake(name: &str) -> String {
    name.replace('-', "_")
}

/// All known CSS property names (kebab-case).
const KNOWN_PROPERTIES: &[&str] = &[
    "display",
    "visibility",
    "layout",
    "dock",
    "overflow",
    "overflow-x",
    "overflow-y",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "margin",
    "padding",
    "color",
    "background",
    "text-align",
    "text-style",
    "border",
];

/// Generate a scalar token stream from a CssValue.
fn scalar_from_value(val: &CssValue) -> Result<TokenStream> {
    match val {
        CssValue::Integer(n, _) => {
            let f = *n as f64;
            Ok(quote! { gilt_tui::css::scalar::Scalar::cells(#f as f32) })
        }
        CssValue::Float(f, _) => {
            Ok(quote! { gilt_tui::css::scalar::Scalar::cells(#f as f32) })
        }
        CssValue::Percent(f, _) => {
            Ok(quote! { gilt_tui::css::scalar::Scalar::percent(#f as f32) })
        }
        CssValue::WithUnit(f, unit, span) => match unit.as_str() {
            "fr" => Ok(quote! { gilt_tui::css::scalar::Scalar::fr(#f as f32) }),
            "vw" => Ok(quote! { gilt_tui::css::scalar::Scalar::vw(#f as f32) }),
            "vh" => Ok(quote! { gilt_tui::css::scalar::Scalar::vh(#f as f32) }),
            "px" => Ok(quote! { gilt_tui::css::scalar::Scalar::cells(#f as f32) }),
            _ => Err(Error::new(*span, format!("unknown unit `{}`", unit))),
        },
        CssValue::Ident(s, span) if s == "auto" => {
            Ok(quote! { gilt_tui::css::scalar::Scalar::auto() })
        }
        other => Err(Error::new(other.span(), "expected a scalar value (number, percentage, or `auto`)")),
    }
}

/// Generate code for a single CSS declaration.
fn generate_declaration(decl: &CssDeclaration) -> Result<TokenStream> {
    // Validate the property name.
    if !KNOWN_PROPERTIES.contains(&decl.name.as_str()) {
        return Err(Error::new(
            decl.name_span,
            format!("unknown CSS property `{}`", decl.name),
        ));
    }

    match decl.name.as_str() {
        // --- Color / Background ---
        "color" | "background" => {
            let field = Ident::new(&kebab_to_snake(&decl.name), decl.name_span);
            let val_str = value_to_string(&decl.values[0])?;
            Ok(quote! { __styles.#field = Some(#val_str.to_string()); })
        }

        // --- Display ---
        "display" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = match val.as_str() {
                "block" => quote! { gilt_tui::css::styles::Display::Block },
                "none" => quote! { gilt_tui::css::styles::Display::None },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!("invalid display value `{}`; expected `block` or `none`", val),
                    ))
                }
            };
            Ok(quote! { __styles.display = Some(#variant); })
        }

        // --- Visibility ---
        "visibility" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = match val.as_str() {
                "visible" => quote! { gilt_tui::css::styles::Visibility::Visible },
                "hidden" => quote! { gilt_tui::css::styles::Visibility::Hidden },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!("invalid visibility value `{}`; expected `visible` or `hidden`", val),
                    ))
                }
            };
            Ok(quote! { __styles.visibility = Some(#variant); })
        }

        // --- Layout ---
        "layout" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = match val.as_str() {
                "vertical" => quote! { gilt_tui::css::styles::LayoutDirection::Vertical },
                "horizontal" => quote! { gilt_tui::css::styles::LayoutDirection::Horizontal },
                "grid" => quote! { gilt_tui::css::styles::LayoutDirection::Grid },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!(
                            "invalid layout value `{}`; expected `vertical`, `horizontal`, or `grid`",
                            val
                        ),
                    ))
                }
            };
            Ok(quote! { __styles.layout = Some(#variant); })
        }

        // --- Dock ---
        "dock" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = match val.as_str() {
                "top" => quote! { gilt_tui::css::styles::Dock::Top },
                "right" => quote! { gilt_tui::css::styles::Dock::Right },
                "bottom" => quote! { gilt_tui::css::styles::Dock::Bottom },
                "left" => quote! { gilt_tui::css::styles::Dock::Left },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!(
                            "invalid dock value `{}`; expected `top`, `right`, `bottom`, or `left`",
                            val
                        ),
                    ))
                }
            };
            Ok(quote! { __styles.dock = Some(#variant); })
        }

        // --- Text align ---
        "text-align" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = match val.as_str() {
                "left" => quote! { gilt_tui::css::styles::TextAlign::Left },
                "center" => quote! { gilt_tui::css::styles::TextAlign::Center },
                "right" => quote! { gilt_tui::css::styles::TextAlign::Right },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!(
                            "invalid text-align value `{}`; expected `left`, `center`, or `right`",
                            val
                        ),
                    ))
                }
            };
            Ok(quote! { __styles.text_align = Some(#variant); })
        }

        // --- Overflow (shorthand and directional) ---
        "overflow" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = overflow_variant(&val, &decl.values[0])?;
            Ok(quote! {
                __styles.overflow_x = Some(#variant);
                __styles.overflow_y = Some(#variant);
            })
        }
        "overflow-x" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = overflow_variant(&val, &decl.values[0])?;
            Ok(quote! { __styles.overflow_x = Some(#variant); })
        }
        "overflow-y" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let variant = overflow_variant(&val, &decl.values[0])?;
            Ok(quote! { __styles.overflow_y = Some(#variant); })
        }

        // --- Sizing scalars ---
        "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height" => {
            let field = Ident::new(&kebab_to_snake(&decl.name), decl.name_span);
            if decl.values.len() != 1 {
                return Err(Error::new(
                    decl.name_span,
                    format!("`{}` expects exactly one value", decl.name),
                ));
            }
            let scalar = scalar_from_value(&decl.values[0])?;
            Ok(quote! { __styles.#field = Some(#scalar); })
        }

        // --- Padding / Margin (1-4 value shorthand) ---
        "padding" | "margin" => {
            let field = Ident::new(&kebab_to_snake(&decl.name), decl.name_span);
            let box_expr = scalar_box_from_values(&decl.values, &decl.name, decl.name_span)?;
            Ok(quote! { __styles.#field = Some(#box_expr); })
        }

        // --- Text style ---
        "text-style" => {
            let val = single_ident(&decl.values, &decl.name)?;
            let flag = match val.as_str() {
                "bold" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        bold: Some(true),
                        ..Default::default()
                    }
                },
                "italic" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        italic: Some(true),
                        ..Default::default()
                    }
                },
                "dim" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        dim: Some(true),
                        ..Default::default()
                    }
                },
                "underline" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        underline: Some(true),
                        ..Default::default()
                    }
                },
                "strikethrough" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        strikethrough: Some(true),
                        ..Default::default()
                    }
                },
                "reverse" => quote! {
                    gilt_tui::css::styles::TextStyleFlags {
                        reverse: Some(true),
                        ..Default::default()
                    }
                },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!(
                            "invalid text-style value `{}`; expected `bold`, `italic`, `dim`, `underline`, `strikethrough`, or `reverse`",
                            val
                        ),
                    ))
                }
            };
            Ok(quote! { __styles.text_style = Some(#flag); })
        }

        // --- Border ---
        "border" => {
            if decl.values.is_empty() {
                return Err(Error::new(decl.name_span, "border requires at least a border kind"));
            }
            let kind_str = ident_value(&decl.values[0])?;
            let kind = match kind_str.as_str() {
                "none" => quote! { gilt_tui::css::styles::BorderKind::None },
                "thin" => quote! { gilt_tui::css::styles::BorderKind::Thin },
                "heavy" => quote! { gilt_tui::css::styles::BorderKind::Heavy },
                "double" => quote! { gilt_tui::css::styles::BorderKind::Double },
                "round" => quote! { gilt_tui::css::styles::BorderKind::Round },
                "ascii" => quote! { gilt_tui::css::styles::BorderKind::Ascii },
                _ => {
                    return Err(Error::new(
                        decl.values[0].span(),
                        format!(
                            "invalid border kind `{}`; expected `none`, `thin`, `heavy`, `double`, `round`, or `ascii`",
                            kind_str
                        ),
                    ))
                }
            };
            let color = if decl.values.len() > 1 {
                let color_str = value_to_string(&decl.values[1])?;
                quote! { Some(#color_str.to_string()) }
            } else {
                quote! { None }
            };
            Ok(quote! {
                __styles.border = Some(gilt_tui::css::styles::Border {
                    kind: #kind,
                    color: #color,
                });
            })
        }

        _ => Err(Error::new(
            decl.name_span,
            format!("unknown CSS property `{}`", decl.name),
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a single identifier value from the declaration values.
fn single_ident(values: &[CssValue], prop: &str) -> Result<String> {
    if values.len() != 1 {
        return Err(Error::new(
            values.first().map_or(Span::call_site(), |v| v.span()),
            format!("`{}` expects exactly one value", prop),
        ));
    }
    ident_value(&values[0])
}

/// Get the string representation of an ident CssValue.
fn ident_value(val: &CssValue) -> Result<String> {
    match val {
        CssValue::Ident(s, _) => Ok(s.clone()),
        other => Err(Error::new(other.span(), "expected an identifier")),
    }
}

/// Convert any CssValue to a string representation (for color/background).
fn value_to_string(val: &CssValue) -> Result<String> {
    match val {
        CssValue::Ident(s, _) => Ok(s.clone()),
        CssValue::Hash(s, _) => Ok(s.clone()),
        CssValue::Str(s, _) => Ok(s.clone()),
        CssValue::Integer(n, _) => Ok(n.to_string()),
        CssValue::Float(f, _) => Ok(f.to_string()),
        other => Err(Error::new(other.span(), "cannot convert value to string")),
    }
}

/// Generate an overflow variant token stream.
fn overflow_variant(val: &str, css_val: &CssValue) -> Result<TokenStream> {
    match val {
        "hidden" => Ok(quote! { gilt_tui::css::styles::Overflow::Hidden }),
        "scroll" => Ok(quote! { gilt_tui::css::styles::Overflow::Scroll }),
        "auto" => Ok(quote! { gilt_tui::css::styles::Overflow::Auto }),
        _ => Err(Error::new(
            css_val.span(),
            format!("invalid overflow value `{}`; expected `hidden`, `scroll`, or `auto`", val),
        )),
    }
}

/// Generate a ScalarBox from 1-4 CSS values (shorthand expansion).
fn scalar_box_from_values(values: &[CssValue], prop: &str, span: Span) -> Result<TokenStream> {
    match values.len() {
        1 => {
            // All four sides.
            let s = scalar_from_value(&values[0])?;
            Ok(quote! { gilt_tui::css::scalar::ScalarBox::all(#s) })
        }
        2 => {
            // vertical horizontal.
            let vert = scalar_from_value(&values[0])?;
            let horiz = scalar_from_value(&values[1])?;
            Ok(quote! { gilt_tui::css::scalar::ScalarBox::symmetric(#vert, #horiz) })
        }
        3 => {
            // top horizontal bottom.
            let top = scalar_from_value(&values[0])?;
            let horiz = scalar_from_value(&values[1])?;
            let bottom = scalar_from_value(&values[2])?;
            Ok(quote! {
                gilt_tui::css::scalar::ScalarBox::new(#top, #horiz, #bottom, #horiz)
            })
        }
        4 => {
            // top right bottom left.
            let top = scalar_from_value(&values[0])?;
            let right = scalar_from_value(&values[1])?;
            let bottom = scalar_from_value(&values[2])?;
            let left = scalar_from_value(&values[3])?;
            Ok(quote! {
                gilt_tui::css::scalar::ScalarBox::new(#top, #right, #bottom, #left)
            })
        }
        _ => Err(Error::new(
            span,
            format!("`{}` expects 1 to 4 values, got {}", prop, values.len()),
        )),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point: generate code for the entire css! macro.
pub(crate) fn css_impl(input: TokenStream) -> Result<TokenStream> {
    let parsed: CssInput = syn::parse2(input)?;

    if parsed.declarations.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "css! macro requires at least one property declaration",
        ));
    }

    let mut field_assignments = Vec::new();
    for decl in &parsed.declarations {
        field_assignments.push(generate_declaration(decl)?);
    }

    Ok(quote! {
        {
            let mut __styles = gilt_tui::css::styles::Styles::new();
            #(#field_assignments)*
            __styles
        }
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    // Helper: parse a CSS input.
    fn parse_css(tokens: TokenStream) -> Result<CssInput> {
        syn::parse2(tokens)
    }

    // Helper: parse and generate.
    fn gen(tokens: TokenStream) -> Result<TokenStream> {
        css_impl(tokens)
    }

    // -----------------------------------------------------------------------
    // Parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_simple_color() {
        let input = parse_css(quote! { color: red; }).unwrap();
        assert_eq!(input.declarations.len(), 1);
        assert_eq!(input.declarations[0].name, "color");
        assert_eq!(input.declarations[0].values.len(), 1);
        match &input.declarations[0].values[0] {
            CssValue::Ident(s, _) => assert_eq!(s, "red"),
            _ => panic!("expected ident"),
        }
    }

    #[test]
    fn parse_kebab_case_property() {
        let input = parse_css(quote! { text-align: center; }).unwrap();
        assert_eq!(input.declarations[0].name, "text-align");
    }

    #[test]
    fn parse_multiple_declarations() {
        let input = parse_css(quote! {
            color: red;
            background: blue;
            display: block;
        })
        .unwrap();
        assert_eq!(input.declarations.len(), 3);
    }

    #[test]
    fn parse_integer_value() {
        let input = parse_css(quote! { width: 50; }).unwrap();
        match &input.declarations[0].values[0] {
            CssValue::Integer(n, _) => assert_eq!(*n, 50),
            _ => panic!("expected integer"),
        }
    }

    #[test]
    fn parse_percent_value() {
        let input = parse_css(quote! { width: 50%; }).unwrap();
        match &input.declarations[0].values[0] {
            CssValue::Percent(f, _) => assert!((f - 50.0).abs() < f64::EPSILON),
            _ => panic!("expected percent"),
        }
    }

    #[test]
    fn parse_hash_color() {
        // Use syn::parse_str to avoid quote! treating # as interpolation.
        let tokens: TokenStream = syn::parse_str("background: #ff0000;").unwrap();
        let input: CssInput = syn::parse2(tokens).unwrap();
        match &input.declarations[0].values[0] {
            CssValue::Hash(s, _) => assert!(s.starts_with('#')),
            _ => panic!("expected hash color, got {:?}", input.declarations[0].values[0]),
        }
    }

    #[test]
    fn parse_padding_shorthand_two_values() {
        let input = parse_css(quote! { padding: 1 2; }).unwrap();
        assert_eq!(input.declarations[0].values.len(), 2);
        match &input.declarations[0].values[0] {
            CssValue::Integer(n, _) => assert_eq!(*n, 1),
            _ => panic!("expected integer"),
        }
        match &input.declarations[0].values[1] {
            CssValue::Integer(n, _) => assert_eq!(*n, 2),
            _ => panic!("expected integer"),
        }
    }

    #[test]
    fn parse_auto_value() {
        let input = parse_css(quote! { width: auto; }).unwrap();
        match &input.declarations[0].values[0] {
            CssValue::Ident(s, _) => assert_eq!(s, "auto"),
            _ => panic!("expected ident auto"),
        }
    }

    // -----------------------------------------------------------------------
    // Code generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn codegen_color() {
        let result = gen(quote! { color: red; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("__styles . color = Some (\"red\" . to_string ())"));
    }

    #[test]
    fn codegen_display_block() {
        let result = gen(quote! { display: block; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("Display :: Block"));
    }

    #[test]
    fn codegen_text_align_center() {
        let result = gen(quote! { text-align: center; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("TextAlign :: Center"));
    }

    #[test]
    fn codegen_width_percent() {
        let result = gen(quote! { width: 50%; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("Scalar :: percent"));
    }

    #[test]
    fn codegen_padding_two_values() {
        let result = gen(quote! { padding: 1 2; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("ScalarBox :: symmetric"));
    }

    #[test]
    fn codegen_padding_one_value() {
        let result = gen(quote! { padding: 1; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("ScalarBox :: all"));
    }

    #[test]
    fn codegen_padding_four_values() {
        let result = gen(quote! { padding: 1 2 3 4; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("ScalarBox :: new"));
    }

    #[test]
    fn codegen_overflow() {
        let result = gen(quote! { overflow: scroll; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("overflow_x"));
        assert!(s.contains("overflow_y"));
        assert!(s.contains("Overflow :: Scroll"));
    }

    #[test]
    fn codegen_border_with_color() {
        let result = gen(quote! { border: thin red; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("BorderKind :: Thin"));
        assert!(s.contains("\"red\""));
    }

    #[test]
    fn codegen_text_style_bold() {
        let result = gen(quote! { text-style: bold; }).unwrap();
        let s = result.to_string();
        assert!(s.contains("bold : Some (true)"));
    }

    // -----------------------------------------------------------------------
    // Error tests
    // -----------------------------------------------------------------------

    #[test]
    fn error_unknown_property() {
        let result = gen(quote! { foo-bar: baz; });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown CSS property"));
    }

    #[test]
    fn error_invalid_display_value() {
        let result = gen(quote! { display: flex; });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid display value"));
    }

    #[test]
    fn error_empty_css() {
        let result = gen(quote! {});
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one property"));
    }

    #[test]
    fn error_too_many_padding_values() {
        let result = gen(quote! { padding: 1 2 3 4 5; });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1 to 4 values"));
    }

    #[test]
    fn codegen_multiple_declarations() {
        let result = gen(quote! {
            color: red;
            background: blue;
            display: block;
            text-align: center;
            width: 50%;
            padding: 1 2;
        })
        .unwrap();
        let s = result.to_string();
        assert!(s.contains("color"));
        assert!(s.contains("background"));
        assert!(s.contains("Display :: Block"));
        assert!(s.contains("TextAlign :: Center"));
        assert!(s.contains("Scalar :: percent"));
        assert!(s.contains("ScalarBox :: symmetric"));
    }
}
