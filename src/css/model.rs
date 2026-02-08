//! CSS AST: Selector, SelectorSet, RuleSet, Declaration.

/// A single CSS selector component.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorComponent {
    /// Type selector: matches widget type name (e.g. `Button`).
    Type(String),
    /// Universal selector: `*`.
    Universal,
    /// Class selector: `.classname`.
    Class(String),
    /// ID selector: `#id`.
    Id(String),
    /// Pseudo-class: `:hover`, `:focus`, etc.
    PseudoClass(String),
}

/// A combinator between selector components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Descendant combinator (whitespace): `A B`.
    Descendant,
    /// Child combinator: `A > B`.
    Child,
}

/// A single compound selector (sequence of components without combinators).
///
/// For example, `Button.primary:hover` is one `CompoundSelector` with three
/// components: `Type("Button")`, `Class("primary")`, `PseudoClass("hover")`.
#[derive(Debug, Clone, PartialEq)]
pub struct CompoundSelector {
    pub components: Vec<SelectorComponent>,
}

impl CompoundSelector {
    /// Create an empty compound selector.
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Append a component to this compound selector.
    pub fn push(&mut self, component: SelectorComponent) {
        self.components.push(component);
    }

    /// Returns `true` if this selector is the universal selector `*` alone.
    pub fn is_universal(&self) -> bool {
        self.components.len() == 1
            && matches!(self.components[0], SelectorComponent::Universal)
    }
}

impl Default for CompoundSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// One element in a selector chain: either a compound selector or a combinator.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorPart {
    /// A compound selector (one or more simple selectors).
    Compound(CompoundSelector),
    /// A combinator between compound selectors.
    Combinator(Combinator),
}

/// A full CSS selector: chain of compound selectors joined by combinators.
///
/// For example, `Container > Button.primary:hover` is a `Selector` with parts:
/// `[Compound(Container), Combinator(Child), Compound(Button.primary:hover)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    /// Alternating compound selectors and combinators.
    /// Always starts and ends with a `SelectorPart::Compound`.
    pub parts: Vec<SelectorPart>,
}

impl Selector {
    /// Create an empty selector.
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

/// A value token within a CSS declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum DeclarationValue {
    /// An identifier like `red`, `bold`, `center`.
    Ident(String),
    /// A bare number like `10`, `3.14`.
    Number(f32),
    /// A number with a unit suffix like `1fr`, `50%`, `80vh`.
    Dimension(f32, String),
    /// A hex color string (without the `#` prefix), e.g. `"ff00aa"`.
    Color(String),
    /// A quoted string value.
    String(String),
    /// A variable reference (without the `$` prefix), e.g. `"primary"`.
    Variable(String),
}

/// A single CSS property declaration, e.g. `color: red` or `margin: 1 2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    /// The property name, e.g. `"color"`, `"margin"`.
    pub property: String,
    /// The declaration values.
    pub values: Vec<DeclarationValue>,
    /// Whether `!important` was specified.
    pub important: bool,
}

impl Declaration {
    /// Create a new declaration.
    pub fn new(property: String, values: Vec<DeclarationValue>, important: bool) -> Self {
        Self {
            property,
            values,
            important,
        }
    }
}

/// A CSS rule: one or more selectors paired with declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSet {
    /// The selectors for this rule (comma-separated in CSS).
    pub selectors: Vec<Selector>,
    /// The property declarations inside the `{ ... }` block.
    pub declarations: Vec<Declaration>,
}

/// A parsed CSS stylesheet: a list of rule sets.
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<RuleSet>,
}

impl StyleSheet {
    /// Create an empty stylesheet.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compound_selector_new() {
        let cs = CompoundSelector::new();
        assert!(cs.components.is_empty());
        assert!(!cs.is_universal());
    }

    #[test]
    fn test_compound_selector_push() {
        let mut cs = CompoundSelector::new();
        cs.push(SelectorComponent::Type("Button".into()));
        cs.push(SelectorComponent::Class("primary".into()));
        assert_eq!(cs.components.len(), 2);
    }

    #[test]
    fn test_compound_selector_is_universal() {
        let mut cs = CompoundSelector::new();
        cs.push(SelectorComponent::Universal);
        assert!(cs.is_universal());
    }

    #[test]
    fn test_compound_selector_not_universal_with_extras() {
        let mut cs = CompoundSelector::new();
        cs.push(SelectorComponent::Universal);
        cs.push(SelectorComponent::Class("foo".into()));
        assert!(!cs.is_universal());
    }

    #[test]
    fn test_selector_new() {
        let s = Selector::new();
        assert!(s.parts.is_empty());
    }

    #[test]
    fn test_selector_with_parts() {
        let mut container = CompoundSelector::new();
        container.push(SelectorComponent::Type("Container".into()));

        let mut button = CompoundSelector::new();
        button.push(SelectorComponent::Type("Button".into()));
        button.push(SelectorComponent::Class("primary".into()));

        let selector = Selector {
            parts: vec![
                SelectorPart::Compound(container),
                SelectorPart::Combinator(Combinator::Child),
                SelectorPart::Compound(button),
            ],
        };

        assert_eq!(selector.parts.len(), 3);
        assert!(matches!(&selector.parts[0], SelectorPart::Compound(cs) if cs.components.len() == 1));
        assert!(matches!(&selector.parts[1], SelectorPart::Combinator(Combinator::Child)));
        assert!(matches!(&selector.parts[2], SelectorPart::Compound(cs) if cs.components.len() == 2));
    }

    #[test]
    fn test_declaration_new() {
        let decl = Declaration::new(
            "color".into(),
            vec![DeclarationValue::Ident("red".into())],
            false,
        );
        assert_eq!(decl.property, "color");
        assert_eq!(decl.values.len(), 1);
        assert!(!decl.important);
    }

    #[test]
    fn test_declaration_important() {
        let decl = Declaration::new(
            "color".into(),
            vec![DeclarationValue::Color("ff0000".into())],
            true,
        );
        assert!(decl.important);
    }

    #[test]
    fn test_declaration_multiple_values() {
        let decl = Declaration::new(
            "margin".into(),
            vec![
                DeclarationValue::Number(1.0),
                DeclarationValue::Number(2.0),
                DeclarationValue::Dimension(50.0, "%".into()),
                DeclarationValue::Dimension(1.0, "fr".into()),
            ],
            false,
        );
        assert_eq!(decl.values.len(), 4);
    }

    #[test]
    fn test_declaration_value_variants() {
        // Verify all variants can be constructed
        let _ident = DeclarationValue::Ident("bold".into());
        let _num = DeclarationValue::Number(42.0);
        let _dim = DeclarationValue::Dimension(100.0, "vw".into());
        let _color = DeclarationValue::Color("aabbcc".into());
        let _string = DeclarationValue::String("hello world".into());
        let _var = DeclarationValue::Variable("primary".into());
    }

    #[test]
    fn test_stylesheet_new() {
        let ss = StyleSheet::new();
        assert!(ss.rules.is_empty());
    }

    #[test]
    fn test_stylesheet_default() {
        let ss = StyleSheet::default();
        assert!(ss.rules.is_empty());
    }

    #[test]
    fn test_ruleset_construction() {
        let mut sel = CompoundSelector::new();
        sel.push(SelectorComponent::Type("Button".into()));

        let rule = RuleSet {
            selectors: vec![Selector {
                parts: vec![SelectorPart::Compound(sel)],
            }],
            declarations: vec![Declaration::new(
                "color".into(),
                vec![DeclarationValue::Ident("red".into())],
                false,
            )],
        };

        assert_eq!(rule.selectors.len(), 1);
        assert_eq!(rule.declarations.len(), 1);
    }

    #[test]
    fn test_selector_component_variants() {
        let type_sel = SelectorComponent::Type("Button".into());
        let universal = SelectorComponent::Universal;
        let class = SelectorComponent::Class("primary".into());
        let id = SelectorComponent::Id("main".into());
        let pseudo = SelectorComponent::PseudoClass("hover".into());

        // Verify equality
        assert_eq!(type_sel, SelectorComponent::Type("Button".into()));
        assert_ne!(type_sel, universal);
        assert_eq!(class, SelectorComponent::Class("primary".into()));
        assert_eq!(id, SelectorComponent::Id("main".into()));
        assert_eq!(pseudo, SelectorComponent::PseudoClass("hover".into()));
    }

    #[test]
    fn test_combinator_variants() {
        assert_ne!(Combinator::Descendant, Combinator::Child);
        assert_eq!(Combinator::Child, Combinator::Child);
    }
}
