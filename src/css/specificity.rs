//! 6-tuple CSS specificity calculation and comparison.
//!
//! Specificity determines which CSS rule wins when multiple rules match the
//! same element. Our 6-tuple model (inspired by Textual) is:
//!
//! ```text
//! (is_user, important, id_count, class_count, type_count, source_order)
//! ```
//!
//! Fields are ordered so that `Ord` (lexicographic) gives the correct result:
//! - User rules beat default rules (`is_user`: 1 > 0)
//! - `!important` beats normal (`important`: 1 > 0)
//! - More IDs beat fewer IDs
//! - More classes/pseudo-classes beat fewer
//! - More type selectors beat fewer
//! - Later source order wins as tie-breaker

use crate::css::model::{Selector, SelectorComponent, SelectorPart};

/// CSS specificity as a 6-tuple, ordered from highest to lowest priority.
///
/// Derive `Ord` so that lexicographic comparison gives the correct cascade order:
/// higher specificity wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity {
    /// 1 for user rules (higher priority), 0 for default rules.
    pub is_user: u8,
    /// 1 if `!important`, 0 otherwise.
    pub important: u8,
    /// Number of ID selectors (`#id`).
    pub id_count: u16,
    /// Number of class + pseudo-class selectors (`.class`, `:hover`).
    pub class_count: u16,
    /// Number of type selectors (`Button`, `Container`).
    pub type_count: u16,
    /// Source order (later rules have higher values, used as tie-breaker).
    pub source_order: u32,
}

impl Specificity {
    /// Create a zero specificity (default rule, not important, no selectors).
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute specificity from a parsed selector.
    ///
    /// - `source_order`: the rule's index in the stylesheet (later = higher).
    /// - `is_default`: `true` for default/built-in rules, `false` for user rules.
    /// - `important`: `true` if the declaration has `!important`.
    pub fn from_selector(
        selector: &Selector,
        source_order: u32,
        is_default: bool,
        important: bool,
    ) -> Self {
        let mut id_count: u16 = 0;
        let mut class_count: u16 = 0;
        let mut type_count: u16 = 0;

        for part in &selector.parts {
            if let SelectorPart::Compound(compound) = part {
                for component in &compound.components {
                    match component {
                        SelectorComponent::Id(_) => id_count += 1,
                        SelectorComponent::Class(_)
                        | SelectorComponent::PseudoClass(_) => class_count += 1,
                        SelectorComponent::Type(_) => type_count += 1,
                        SelectorComponent::Universal => {
                            // Universal selector has zero specificity.
                        }
                    }
                }
            }
        }

        Self {
            is_user: u8::from(!is_default),
            important: u8::from(important),
            id_count,
            class_count,
            type_count,
            source_order,
        }
    }

    /// Returns `true` if this specificity came from a default/built-in rule.
    pub fn is_default(&self) -> bool {
        self.is_user == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::model::{CompoundSelector, SelectorPart};

    /// Build a selector from a list of components (single compound, no combinators).
    fn simple_selector(components: Vec<SelectorComponent>) -> Selector {
        Selector {
            parts: vec![SelectorPart::Compound(CompoundSelector { components })],
        }
    }

    #[test]
    fn test_specificity_new() {
        let s = Specificity::new();
        assert_eq!(s.is_user, 0);
        assert_eq!(s.important, 0);
        assert_eq!(s.id_count, 0);
        assert_eq!(s.class_count, 0);
        assert_eq!(s.type_count, 0);
        assert_eq!(s.source_order, 0);
        assert!(s.is_default());
    }

    #[test]
    fn test_from_selector_type_only() {
        // Button
        let sel = simple_selector(vec![SelectorComponent::Type("Button".into())]);
        let spec = Specificity::from_selector(&sel, 0, false, false);

        assert_eq!(spec.is_user, 1);
        assert_eq!(spec.type_count, 1);
        assert_eq!(spec.class_count, 0);
        assert_eq!(spec.id_count, 0);
    }

    #[test]
    fn test_from_selector_class() {
        // .primary
        let sel = simple_selector(vec![SelectorComponent::Class("primary".into())]);
        let spec = Specificity::from_selector(&sel, 0, false, false);

        assert_eq!(spec.class_count, 1);
        assert_eq!(spec.type_count, 0);
    }

    #[test]
    fn test_from_selector_id() {
        // #main
        let sel = simple_selector(vec![SelectorComponent::Id("main".into())]);
        let spec = Specificity::from_selector(&sel, 0, false, false);

        assert_eq!(spec.id_count, 1);
        assert_eq!(spec.class_count, 0);
        assert_eq!(spec.type_count, 0);
    }

    #[test]
    fn test_from_selector_compound() {
        // Button.primary:hover
        let sel = simple_selector(vec![
            SelectorComponent::Type("Button".into()),
            SelectorComponent::Class("primary".into()),
            SelectorComponent::PseudoClass("hover".into()),
        ]);
        let spec = Specificity::from_selector(&sel, 0, false, false);

        assert_eq!(spec.type_count, 1);
        assert_eq!(spec.class_count, 2); // .primary + :hover
        assert_eq!(spec.id_count, 0);
    }

    #[test]
    fn test_from_selector_universal_zero_specificity() {
        // *
        let sel = simple_selector(vec![SelectorComponent::Universal]);
        let spec = Specificity::from_selector(&sel, 0, false, false);

        assert_eq!(spec.type_count, 0);
        assert_eq!(spec.class_count, 0);
        assert_eq!(spec.id_count, 0);
    }

    #[test]
    fn test_user_beats_default() {
        let sel = simple_selector(vec![SelectorComponent::Type("Button".into())]);
        let user = Specificity::from_selector(&sel, 0, false, false);
        let default = Specificity::from_selector(&sel, 0, true, false);

        assert!(user > default, "user rule should beat default rule");
        assert!(!user.is_default());
        assert!(default.is_default());
    }

    #[test]
    fn test_important_beats_normal() {
        let sel = simple_selector(vec![SelectorComponent::Type("Button".into())]);
        let important = Specificity::from_selector(&sel, 0, false, true);
        let normal = Specificity::from_selector(&sel, 0, false, false);

        assert!(
            important > normal,
            "!important should beat normal"
        );
    }

    #[test]
    fn test_id_beats_class() {
        let id_sel = simple_selector(vec![SelectorComponent::Id("main".into())]);
        let class_sel = simple_selector(vec![SelectorComponent::Class("primary".into())]);

        let id_spec = Specificity::from_selector(&id_sel, 0, false, false);
        let class_spec = Specificity::from_selector(&class_sel, 0, false, false);

        assert!(id_spec > class_spec, "ID selector should beat class selector");
    }

    #[test]
    fn test_class_beats_type() {
        let class_sel = simple_selector(vec![SelectorComponent::Class("primary".into())]);
        let type_sel = simple_selector(vec![SelectorComponent::Type("Button".into())]);

        let class_spec = Specificity::from_selector(&class_sel, 0, false, false);
        let type_spec = Specificity::from_selector(&type_sel, 0, false, false);

        assert!(
            class_spec > type_spec,
            "class selector should beat type selector"
        );
    }

    #[test]
    fn test_source_order_tiebreak() {
        let sel = simple_selector(vec![SelectorComponent::Type("Button".into())]);
        let earlier = Specificity::from_selector(&sel, 0, false, false);
        let later = Specificity::from_selector(&sel, 1, false, false);

        assert!(later > earlier, "later source order should win as tiebreaker");
    }

    #[test]
    fn test_multiple_ids_beat_fewer() {
        let two_ids = simple_selector(vec![
            SelectorComponent::Id("a".into()),
            SelectorComponent::Id("b".into()),
        ]);
        let one_id = simple_selector(vec![SelectorComponent::Id("a".into())]);

        let two_spec = Specificity::from_selector(&two_ids, 0, false, false);
        let one_spec = Specificity::from_selector(&one_id, 0, false, false);

        assert!(two_spec > one_spec);
    }

    #[test]
    fn test_user_important_highest_priority() {
        let sel = simple_selector(vec![SelectorComponent::Universal]);
        let user_important = Specificity::from_selector(&sel, 0, false, true);

        // Even a default rule with an ID selector should lose
        let id_sel = simple_selector(vec![SelectorComponent::Id("x".into())]);
        let default_normal = Specificity::from_selector(&id_sel, 100, true, false);

        assert!(
            user_important > default_normal,
            "user !important should beat default normal with high specificity"
        );
    }

    #[test]
    fn test_ordering_chain() {
        // Verify the full ordering: user important > user normal > default important > default normal
        let sel = simple_selector(vec![SelectorComponent::Type("X".into())]);

        let user_important = Specificity::from_selector(&sel, 0, false, true);
        let user_normal = Specificity::from_selector(&sel, 0, false, false);
        let default_important = Specificity::from_selector(&sel, 0, true, true);
        let default_normal = Specificity::from_selector(&sel, 0, true, false);

        assert!(user_important > user_normal);
        assert!(user_normal > default_important);
        assert!(default_important > default_normal);
    }

    #[test]
    fn test_complex_selector_with_combinators() {
        // Container > Button.primary:hover
        use crate::css::model::Combinator;
        let selector = Selector {
            parts: vec![
                SelectorPart::Compound(CompoundSelector {
                    components: vec![SelectorComponent::Type("Container".into())],
                }),
                SelectorPart::Combinator(Combinator::Child),
                SelectorPart::Compound(CompoundSelector {
                    components: vec![
                        SelectorComponent::Type("Button".into()),
                        SelectorComponent::Class("primary".into()),
                        SelectorComponent::PseudoClass("hover".into()),
                    ],
                }),
            ],
        };

        let spec = Specificity::from_selector(&selector, 5, false, false);
        // 2 types (Container, Button), 2 classes (.primary, :hover), 0 IDs
        assert_eq!(spec.type_count, 2);
        assert_eq!(spec.class_count, 2);
        assert_eq!(spec.id_count, 0);
        assert_eq!(spec.source_order, 5);
        assert_eq!(spec.is_user, 1);
    }
}
