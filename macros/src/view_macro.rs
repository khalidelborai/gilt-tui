//! RSX view! macro: parse JSX-like syntax and generate gilt-tui builder code.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

// ---------------------------------------------------------------------------
// AST types
// ---------------------------------------------------------------------------

/// A parsed RSX attribute: `name="value"`.
#[derive(Clone)]
pub(crate) struct Attribute {
    pub name: Ident,
    pub value: LitStr,
}

impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name.to_string())
            .field("value", &self.value.value())
            .finish()
    }
}

/// A parsed RSX element: `<Tag attrs... />` or `<Tag attrs...> children </Tag>`.
#[derive(Clone)]
pub(crate) struct Element {
    pub tag: Ident,
    pub attrs: Vec<Attribute>,
    pub children: Vec<Element>,
    pub self_closing: bool,
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("tag", &self.tag.to_string())
            .field("attrs", &self.attrs)
            .field("children", &self.children)
            .field("self_closing", &self.self_closing)
            .finish()
    }
}

/// The top-level view! input: a sequence of elements.
#[derive(Debug)]
struct ViewInput {
    elements: Vec<Element>,
}

// ---------------------------------------------------------------------------
// Constructor argument attributes
// ---------------------------------------------------------------------------

/// Attributes whose value becomes the `::new()` constructor argument.
const CONSTRUCTOR_ATTRS: &[&str] = &["title", "label", "content"];

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl Parse for ViewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut elements = Vec::new();
        while !input.is_empty() {
            elements.push(parse_element(input)?);
        }
        Ok(ViewInput { elements })
    }
}

/// Parse a single RSX element from the token stream.
pub(crate) fn parse_element(input: ParseStream) -> Result<Element> {
    // Expect `<`
    input.parse::<Token![<]>()?;

    // Parse tag name
    let tag: Ident = input.parse()?;

    // Parse attributes until we hit `/>` or `>`
    let mut attrs = Vec::new();
    loop {
        // Check for self-closing `/>`.
        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(Element {
                tag,
                attrs,
                children: Vec::new(),
                self_closing: true,
            });
        }

        // Check for open tag close `>`.
        if input.peek(Token![>]) {
            input.parse::<Token![>]>()?;
            break;
        }

        // Parse attribute: `name = "value"`
        let attr_name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let attr_value: LitStr = input.parse()?;
        attrs.push(Attribute {
            name: attr_name,
            value: attr_value,
        });
    }

    // Parse children until closing tag `</Tag>`.
    let mut children = Vec::new();
    loop {
        // Check for closing tag: `</`
        if input.peek(Token![<]) && input.peek2(Token![/]) {
            input.parse::<Token![<]>()?;
            input.parse::<Token![/]>()?;
            let closing_tag: Ident = input.parse()?;
            if closing_tag != tag {
                return Err(Error::new(
                    closing_tag.span(),
                    format!(
                        "mismatched closing tag: expected `</{}>`, found `</{}>`",
                        tag, closing_tag
                    ),
                ));
            }
            input.parse::<Token![>]>()?;
            break;
        }

        // Otherwise, parse a child element.
        if input.peek(Token![<]) {
            children.push(parse_element(input)?);
        } else {
            return Err(input.error("expected `<` to start a child element or `</` to close the parent"));
        }
    }

    Ok(Element {
        tag,
        attrs,
        children,
        self_closing: false,
    })
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

/// Generate code for a single element.
fn generate_element(elem: &Element) -> TokenStream {
    let tag = &elem.tag;

    // Find the constructor argument (first constructor-type attribute).
    let ctor_attr = elem.attrs.iter().find(|a| {
        CONSTRUCTOR_ATTRS.contains(&a.name.to_string().as_str())
    });

    // Determine if this is a Container (has its own with_id/with_class/with_child).
    let is_container = tag == "Container";

    // Generate the initial constructor call.
    let constructor = if let Some(attr) = ctor_attr {
        let val = &attr.value;
        quote! { gilt_tui::widgets::#tag::new(#val) }
    } else {
        quote! { gilt_tui::widgets::#tag::new() }
    };

    // Build up the chain of builder calls.
    let mut builder_calls = Vec::new();

    for attr in &elem.attrs {
        let attr_name_str = attr.name.to_string();
        let val = &attr.value;

        // Skip the attribute we already used as the constructor argument.
        if let Some(ctor) = ctor_attr {
            if attr.name == ctor.name {
                continue;
            }
        }

        match attr_name_str.as_str() {
            "id" => {
                builder_calls.push(quote! { .with_id(#val) });
            }
            "class" => {
                builder_calls.push(quote! { .with_class(#val) });
            }
            _ => {
                // Convert attribute name to `with_<name>` method.
                let method_name = Ident::new(
                    &format!("with_{}", attr_name_str),
                    attr.name.span(),
                );
                builder_calls.push(quote! { .#method_name(#val) });
            }
        }
    }

    // If there are children and this is a Container, add with_child calls.
    if !elem.children.is_empty() && is_container {
        for child in &elem.children {
            let child_code = generate_element(child);
            builder_calls.push(quote! { .with_child(#child_code) });
        }
    }

    // For non-Container types with children, we still try with_child
    // (the user might have custom container types).
    if !elem.children.is_empty() && !is_container {
        for child in &elem.children {
            let child_code = generate_element(child);
            builder_calls.push(quote! { .with_child(#child_code) });
        }
    }

    quote! {
        #constructor #(#builder_calls)*
    }
}

/// Entry point: generate code for the entire view! macro.
pub(crate) fn view_impl(input: TokenStream) -> Result<TokenStream> {
    let parsed: ViewInput = syn::parse2(input)?;

    if parsed.elements.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "view! macro requires at least one element",
        ));
    }

    let element_exprs: Vec<TokenStream> = parsed
        .elements
        .iter()
        .map(|elem| {
            let code = generate_element(elem);
            quote! {
                __children.push(Box::new(#code));
            }
        })
        .collect();

    Ok(quote! {
        {
            let mut __children: Vec<Box<dyn gilt_tui::widget::Widget>> = Vec::new();
            #(#element_exprs)*
            __children
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

    // Helper: parse a token stream into a ViewInput.
    fn parse_view(tokens: TokenStream) -> Result<ViewInput> {
        syn::parse2(tokens)
    }

    // Helper: parse a single element.
    fn parse_single_element(tokens: TokenStream) -> Result<Element> {
        syn::parse2::<ViewInput>(tokens).map(|v| v.elements.into_iter().next().unwrap())
    }

    // -----------------------------------------------------------------------
    // Parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_self_closing_element() {
        let elem = parse_single_element(quote! { <Header title="My App" /> }).unwrap();
        assert_eq!(elem.tag.to_string(), "Header");
        assert!(elem.self_closing);
        assert!(elem.children.is_empty());
        assert_eq!(elem.attrs.len(), 1);
        assert_eq!(elem.attrs[0].name.to_string(), "title");
        assert_eq!(elem.attrs[0].value.value(), "My App");
    }

    #[test]
    fn parse_self_closing_no_attrs() {
        let elem = parse_single_element(quote! { <Sidebar /> }).unwrap();
        assert_eq!(elem.tag.to_string(), "Sidebar");
        assert!(elem.self_closing);
        assert!(elem.attrs.is_empty());
    }

    #[test]
    fn parse_element_with_children() {
        let elem = parse_single_element(quote! {
            <Container>
                <Static content="Hello" />
                <Button label="Click" />
            </Container>
        })
        .unwrap();
        assert_eq!(elem.tag.to_string(), "Container");
        assert!(!elem.self_closing);
        assert_eq!(elem.children.len(), 2);
        assert_eq!(elem.children[0].tag.to_string(), "Static");
        assert_eq!(elem.children[1].tag.to_string(), "Button");
    }

    #[test]
    fn parse_nested_containers() {
        let elem = parse_single_element(quote! {
            <Container>
                <Container>
                    <Static content="Deep" />
                </Container>
            </Container>
        })
        .unwrap();
        assert_eq!(elem.children.len(), 1);
        assert_eq!(elem.children[0].children.len(), 1);
        assert_eq!(elem.children[0].children[0].tag.to_string(), "Static");
    }

    #[test]
    fn parse_multiple_attributes() {
        let elem = parse_single_element(quote! {
            <Container id="main" class="primary" />
        })
        .unwrap();
        assert_eq!(elem.attrs.len(), 2);
        assert_eq!(elem.attrs[0].name.to_string(), "id");
        assert_eq!(elem.attrs[0].value.value(), "main");
        assert_eq!(elem.attrs[1].name.to_string(), "class");
        assert_eq!(elem.attrs[1].value.value(), "primary");
    }

    #[test]
    fn parse_multiple_root_elements() {
        let view = parse_view(quote! {
            <Header title="App" />
            <Static content="Body" />
            <Footer content="End" />
        })
        .unwrap();
        assert_eq!(view.elements.len(), 3);
    }

    #[test]
    fn parse_error_mismatched_closing_tag() {
        let result = parse_single_element(quote! {
            <Container>
                <Static content="x" />
            </Header>
        });
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("mismatched closing tag"));
    }

    #[test]
    fn parse_container_with_attrs_and_children() {
        let elem = parse_single_element(quote! {
            <Container class="sidebar" id="nav">
                <Button label="Home" />
            </Container>
        })
        .unwrap();
        assert_eq!(elem.attrs.len(), 2);
        assert_eq!(elem.children.len(), 1);
        assert_eq!(elem.children[0].tag.to_string(), "Button");
    }

    // -----------------------------------------------------------------------
    // Code generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn codegen_self_closing_with_title() {
        let elem = parse_single_element(quote! { <Header title="App" /> }).unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("gilt_tui :: widgets :: Header :: new (\"App\")"));
    }

    #[test]
    fn codegen_self_closing_with_label() {
        let elem = parse_single_element(quote! { <Button label="OK" /> }).unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("gilt_tui :: widgets :: Button :: new (\"OK\")"));
    }

    #[test]
    fn codegen_with_id_and_class() {
        let elem = parse_single_element(quote! {
            <Container id="main" class="primary" />
        })
        .unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("with_id"), "expected with_id in: {}", code_str);
        assert!(code_str.contains("\"main\""), "expected \"main\" in: {}", code_str);
        assert!(code_str.contains("with_class"), "expected with_class in: {}", code_str);
        assert!(code_str.contains("\"primary\""), "expected \"primary\" in: {}", code_str);
    }

    #[test]
    fn codegen_container_with_children() {
        let elem = parse_single_element(quote! {
            <Container>
                <Static content="Hello" />
            </Container>
        })
        .unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("with_child"));
        assert!(code_str.contains("Static :: new (\"Hello\")"));
    }

    #[test]
    fn codegen_no_ctor_attr() {
        let elem = parse_single_element(quote! { <Input /> }).unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("gilt_tui :: widgets :: Input :: new ()"));
    }

    #[test]
    fn codegen_custom_attribute() {
        let elem = parse_single_element(quote! {
            <Input placeholder="Type here..." />
        })
        .unwrap();
        let code = generate_element(&elem);
        let code_str = code.to_string();
        assert!(code_str.contains("with_placeholder"));
    }

    #[test]
    fn codegen_full_view() {
        let result = view_impl(quote! {
            <Header title="My App" />
            <Container class="main">
                <Static content="Hello" />
            </Container>
            <Footer content="Done" />
        });
        assert!(result.is_ok());
        let code_str = result.unwrap().to_string();
        assert!(code_str.contains("__children"));
        assert!(code_str.contains("Header :: new (\"My App\")"));
        assert!(code_str.contains("Footer :: new (\"Done\")"));
    }

    #[test]
    fn codegen_empty_view_is_error() {
        let result = view_impl(quote! {});
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one element"));
    }
}
