//! Proc macros for gilt-tui: `view!` RSX composition and `css!` inline CSS validation.
//!
//! This crate is not meant to be used directly — enable the `macros` feature on `gilt-tui`.

use proc_macro::TokenStream;

mod view_macro;
mod css_macro;

/// RSX-style widget composition macro.
///
/// Transforms JSX-like syntax into gilt-tui builder API calls.
///
/// # Syntax
///
/// - `<WidgetType />` — self-closing element (no children)
/// - `<WidgetType attr="val"> ... </WidgetType>` — element with children
///
/// # Attributes
///
/// - `id="value"` becomes `.with_id("value")`
/// - `class="value"` becomes `.with_class("value")`
/// - `title`, `label`, `content` — first such attribute becomes the `::new()` argument
/// - Other string attributes become `.with_attr_name("value")` builder calls
///
/// # Example
///
/// ```ignore
/// view! {
///     <Header title="My App" />
///     <Container class="main" layout="horizontal">
///         <Static content="Hello" />
///         <Button label="Click me" />
///     </Container>
///     <Footer content="Status: OK" />
/// }
/// ```
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view_macro::view_impl(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Compile-time CSS validation macro.
///
/// Parses CSS property declarations at compile time and produces a
/// `gilt_tui::css::styles::Styles` struct.
///
/// # Syntax
///
/// ```ignore
/// let styles = css! {
///     color: red;
///     background: #1a1a2e;
///     padding: 1 2;
///     text-align: center;
///     width: 50%;
///     display: block;
/// };
/// ```
///
/// Property names use kebab-case (converted to snake_case internally).
/// Values are validated at compile time.
#[proc_macro]
pub fn css(input: TokenStream) -> TokenStream {
    css_macro::css_impl(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
