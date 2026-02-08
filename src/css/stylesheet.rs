//! Stylesheet manager: cascade resolution, apply styles to DOM.
//!
//! Compiles a parsed [`StyleSheet`] into a form ready for matching against DOM
//! nodes, computing specificity, and merging styles via the CSS cascade.

use crate::css::model::{
    Combinator, CompoundSelector, Declaration, RuleSet, Selector, SelectorComponent, SelectorPart,
    StyleSheet,
};
use crate::css::properties::apply_declaration;
use crate::css::specificity::Specificity;
use crate::css::styles::Styles;
use crate::dom::node::{NodeData, NodeId};
use crate::dom::tree::Dom;

/// A compiled stylesheet ready for matching against DOM nodes.
#[derive(Debug, Default)]
pub struct CompiledStylesheet {
    /// Rules with pre-computed specificity, ordered by source order.
    rules: Vec<CompiledRule>,
}

/// A single rule with its pre-computed specificity.
#[derive(Debug)]
struct CompiledRule {
    rule: RuleSet,
    specificity: Specificity,
    /// Source order index for stable sorting.
    source_order: usize,
}

impl CompiledStylesheet {
    /// Compile a parsed [`StyleSheet`] by computing specificity for each rule.
    ///
    /// If `is_default` is true, this is a default/user-agent stylesheet (lower priority).
    pub fn compile(stylesheet: &StyleSheet, is_default: bool) -> Self {
        let mut rules = Vec::with_capacity(stylesheet.rules.len());

        for (i, rule) in stylesheet.rules.iter().enumerate() {
            // Check if any declaration has !important
            let has_important = rule.declarations.iter().any(|d| d.important);

            // Compute the highest specificity among all selectors in this rule.
            let specificity = rule
                .selectors
                .iter()
                .map(|sel| {
                    Specificity::from_selector(sel, i as u32, is_default, has_important)
                })
                .max()
                .unwrap_or_default();

            rules.push(CompiledRule {
                rule: rule.clone(),
                specificity,
                source_order: i,
            });
        }

        CompiledStylesheet { rules }
    }

    /// Compute styles for a single node by matching all rules against it.
    ///
    /// Rules are applied in specificity order (lowest first, highest wins via merge).
    /// Within the same specificity, source order is preserved (later rules win).
    pub fn compute_styles(&self, node_id: NodeId, dom: &Dom) -> Styles {
        // Collect all matching rules with their specificity and source order.
        let mut matches: Vec<(Specificity, usize, &[Declaration])> = Vec::new();

        for compiled_rule in &self.rules {
            let rule = &compiled_rule.rule;
            let any_selector_matches = rule
                .selectors
                .iter()
                .any(|sel| matches_selector(sel, node_id, dom));

            if any_selector_matches {
                matches.push((
                    compiled_rule.specificity,
                    compiled_rule.source_order,
                    &rule.declarations,
                ));
            }
        }

        // Sort by specificity ascending, then by source order ascending.
        // Last applied wins via merge, so higher specificity / later source = wins.
        matches.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        // Merge all matching styles together.
        let mut result = Styles::new();
        for (_specificity, _order, declarations) in &matches {
            let mut rule_styles = Styles::new();
            for decl in *declarations {
                // Silently ignore errors from individual declarations.
                let _ = apply_declaration(&mut rule_styles, &decl.property, &decl.values);
            }
            result = result.merge(&rule_styles);
        }

        result
    }
}

/// Check whether a full selector matches a given node.
///
/// Walks the selector parts from right to left, matching compound selectors
/// and navigating the DOM tree via combinators.
fn matches_selector(selector: &Selector, node_id: NodeId, dom: &Dom) -> bool {
    let parts = &selector.parts;
    if parts.is_empty() {
        return false;
    }

    // Walk parts from right to left.
    // The rightmost part must be a compound selector matching the target node.
    let mut part_idx = parts.len() - 1;

    match &parts[part_idx] {
        SelectorPart::Compound(compound) => {
            let node = match dom.get(node_id) {
                Some(n) => n,
                None => return false,
            };
            if !matches_compound(compound, node) {
                return false;
            }
        }
        SelectorPart::Combinator(_) => return false,
    }

    if part_idx == 0 {
        return true;
    }

    // Walk leftward through combinator + compound pairs.
    let mut current_node = node_id;

    loop {
        if part_idx == 0 {
            // All parts matched.
            return true;
        }

        // part_idx - 1 should be a combinator
        part_idx -= 1;
        let combinator = match &parts[part_idx] {
            SelectorPart::Combinator(c) => c,
            _ => return false,
        };

        if part_idx == 0 {
            // Combinator without a preceding compound — invalid.
            return false;
        }

        // part_idx - 1 should be a compound selector
        part_idx -= 1;
        let compound = match &parts[part_idx] {
            SelectorPart::Compound(c) => c,
            _ => return false,
        };

        match combinator {
            Combinator::Child => {
                // Immediate parent must match.
                let parent_id = match dom.parent(current_node) {
                    Some(p) => p,
                    None => return false,
                };
                let parent = match dom.get(parent_id) {
                    Some(n) => n,
                    None => return false,
                };
                if !matches_compound(compound, parent) {
                    return false;
                }
                current_node = parent_id;
            }
            Combinator::Descendant => {
                // Walk up ancestors to find a match.
                let ancestors = dom.ancestors(current_node);
                let found = ancestors.iter().find(|&&ancestor_id| {
                    dom.get(ancestor_id)
                        .is_some_and(|ancestor| matches_compound(compound, ancestor))
                });
                match found {
                    Some(&ancestor_id) => {
                        current_node = ancestor_id;
                    }
                    None => return false,
                }
            }
        }
    }
}

/// Check whether a compound selector matches a single node's data.
fn matches_compound(compound: &CompoundSelector, node: &NodeData) -> bool {
    compound.components.iter().all(|component| match component {
        SelectorComponent::Type(name) => node.widget_type == *name,
        SelectorComponent::Class(name) => node.has_class(name),
        SelectorComponent::Id(name) => node.id.as_deref() == Some(name.as_str()),
        SelectorComponent::Universal => true,
        SelectorComponent::PseudoClass(_) => {
            // Pseudo-classes need runtime state; skip for Phase 1.
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parser::parse_css;
    use crate::css::styles::{Display, TextAlign};
    use crate::dom::node::NodeData;
    use crate::dom::tree::Dom;

    /// Build a test tree:
    /// ```text
    ///       root (Container #root)
    ///      /    \
    ///    panel   sidebar
    ///  (Panel    (Panel
    ///   #main     #sidebar
    ///   .content)  .nav)
    ///   / \
    ///  btn  lbl
    /// (Button  (Label
    ///  .primary  #title)
    ///  .btn)
    /// ```
    fn build_test_dom() -> (Dom, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut dom = Dom::new();
        let root = dom.insert(NodeData::new("Container").with_id("root"));
        let panel = dom.insert_child(
            root,
            NodeData::new("Panel")
                .with_id("main")
                .with_class("content"),
        );
        let sidebar = dom.insert_child(
            root,
            NodeData::new("Panel")
                .with_id("sidebar")
                .with_class("nav"),
        );
        let btn = dom.insert_child(
            panel,
            NodeData::new("Button")
                .with_class("primary")
                .with_class("btn"),
        );
        let lbl = dom.insert_child(panel, NodeData::new("Label").with_id("title"));
        (dom, root, panel, sidebar, btn, lbl)
    }

    // ── Selector matching ────────────────────────────────────────────

    #[test]
    fn match_type_selector() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css("Button { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("red".into()));
    }

    #[test]
    fn match_class_selector() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css(".primary { color: blue; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("blue".into()));
    }

    #[test]
    fn match_id_selector() {
        let (dom, _, _, _, _, lbl) = build_test_dom();
        let sheet = parse_css("#title { color: green; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(lbl, &dom);
        assert_eq!(styles.color, Some("green".into()));
    }

    #[test]
    fn match_universal_selector() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css("* { color: white; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("white".into()));
    }

    #[test]
    fn no_match_wrong_type() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css("Label { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert!(styles.color.is_none());
    }

    // ── Descendant combinator matching ───────────────────────────────

    #[test]
    fn match_descendant_combinator() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css("Container Button { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("red".into()));
    }

    #[test]
    fn match_descendant_skips_intermediate() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        // Button is grandchild of Container (Container > Panel > Button)
        let sheet = parse_css("Container Button { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("red".into()));
    }

    #[test]
    fn no_match_wrong_ancestor() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        // sidebar is Panel, not the parent of btn
        let sheet = parse_css("#sidebar Button { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert!(styles.color.is_none());
    }

    // ── Child combinator matching ────────────────────────────────────

    #[test]
    fn match_child_combinator() {
        let (dom, _, panel, _, _, _) = build_test_dom();
        let sheet = parse_css("Container > Panel { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(panel, &dom);
        assert_eq!(styles.color, Some("red".into()));
    }

    #[test]
    fn no_match_child_combinator_grandchild() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        // Button is grandchild of Container, not direct child
        let sheet = parse_css("Container > Button { color: red; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert!(styles.color.is_none());
    }

    // ── Cascade order ────────────────────────────────────────────────

    #[test]
    fn cascade_higher_specificity_wins() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        // Type selector (specificity 0,0,1) vs class selector (specificity 0,1,0)
        let sheet =
            parse_css("Button { color: red; } .primary { color: blue; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("blue".into()));
    }

    #[test]
    fn cascade_later_rule_wins_at_same_specificity() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet =
            parse_css("Button { color: red; } Button { color: blue; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("blue".into()));
    }

    #[test]
    fn cascade_merge_different_properties() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet =
            parse_css("Button { color: red; } .primary { background: blue; }").unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("red".into()));
        assert_eq!(styles.background, Some("blue".into()));
    }

    // ── Multiple matching rules merge ────────────────────────────────

    #[test]
    fn merge_multiple_rules() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let sheet = parse_css(
            r#"
            Button { display: block; color: red; }
            .btn { text-align: center; }
            .primary { color: blue; background: white; }
            "#,
        )
        .unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);

        assert_eq!(styles.display, Some(Display::Block));
        assert_eq!(styles.color, Some("blue".into())); // .primary overrides Button
        assert_eq!(styles.text_align, Some(TextAlign::Center));
        assert_eq!(styles.background, Some("white".into()));
    }

    // ── Empty stylesheet ─────────────────────────────────────────────

    #[test]
    fn empty_stylesheet_produces_empty_styles() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let compiled = CompiledStylesheet::default();
        let styles = compiled.compute_styles(btn, &dom);
        assert!(styles.is_empty());
    }

    // ── Compound selector in cascade ─────────────────────────────────

    #[test]
    fn compound_selector_higher_specificity() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        // Button.primary has specificity (0,1,1) > .primary (0,1,0) > Button (0,0,1)
        let sheet = parse_css(
            r#"
            Button { color: red; }
            .primary { color: green; }
            Button.primary { color: blue; }
            "#,
        )
        .unwrap();
        let compiled = CompiledStylesheet::compile(&sheet, false);
        let styles = compiled.compute_styles(btn, &dom);
        assert_eq!(styles.color, Some("blue".into()));
    }

    // ── matches_compound direct tests ────────────────────────────────

    #[test]
    fn matches_compound_type() {
        let node = NodeData::new("Button");
        let compound = CompoundSelector {
            components: vec![SelectorComponent::Type("Button".into())],
        };
        assert!(matches_compound(&compound, &node));

        let compound_wrong = CompoundSelector {
            components: vec![SelectorComponent::Type("Label".into())],
        };
        assert!(!matches_compound(&compound_wrong, &node));
    }

    #[test]
    fn matches_compound_class() {
        let node = NodeData::new("Button").with_class("primary");
        let compound = CompoundSelector {
            components: vec![SelectorComponent::Class("primary".into())],
        };
        assert!(matches_compound(&compound, &node));

        let compound_wrong = CompoundSelector {
            components: vec![SelectorComponent::Class("secondary".into())],
        };
        assert!(!matches_compound(&compound_wrong, &node));
    }

    #[test]
    fn matches_compound_id() {
        let node = NodeData::new("Button").with_id("save");
        let compound = CompoundSelector {
            components: vec![SelectorComponent::Id("save".into())],
        };
        assert!(matches_compound(&compound, &node));
    }

    #[test]
    fn matches_compound_universal() {
        let node = NodeData::new("Button");
        let compound = CompoundSelector {
            components: vec![SelectorComponent::Universal],
        };
        assert!(matches_compound(&compound, &node));
    }

    #[test]
    fn matches_compound_pseudo_class_returns_false() {
        let node = NodeData::new("Button");
        let compound = CompoundSelector {
            components: vec![
                SelectorComponent::Type("Button".into()),
                SelectorComponent::PseudoClass("hover".into()),
            ],
        };
        // Pseudo-classes are skipped (return false) in Phase 1
        assert!(!matches_compound(&compound, &node));
    }

    #[test]
    fn matches_compound_multiple_parts() {
        let node = NodeData::new("Button")
            .with_class("primary")
            .with_class("btn");
        let compound = CompoundSelector {
            components: vec![
                SelectorComponent::Type("Button".into()),
                SelectorComponent::Class("primary".into()),
            ],
        };
        assert!(matches_compound(&compound, &node));

        // Fails if any part doesn't match
        let compound_fail = CompoundSelector {
            components: vec![
                SelectorComponent::Type("Button".into()),
                SelectorComponent::Class("secondary".into()),
            ],
        };
        assert!(!matches_compound(&compound_fail, &node));
    }

    // ── matches_selector direct tests ────────────────────────────────

    #[test]
    fn matches_selector_simple() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let selector = Selector {
            parts: vec![SelectorPart::Compound(CompoundSelector {
                components: vec![SelectorComponent::Type("Button".into())],
            })],
        };
        assert!(matches_selector(&selector, btn, &dom));
    }

    #[test]
    fn matches_selector_child_combinator() {
        let (dom, _, panel, _, btn, _) = build_test_dom();
        let selector = Selector {
            parts: vec![
                SelectorPart::Compound(CompoundSelector {
                    components: vec![SelectorComponent::Type("Panel".into())],
                }),
                SelectorPart::Combinator(Combinator::Child),
                SelectorPart::Compound(CompoundSelector {
                    components: vec![SelectorComponent::Type("Button".into())],
                }),
            ],
        };
        assert!(matches_selector(&selector, btn, &dom));

        // panel's parent is Container, not Panel
        assert!(!matches_selector(&selector, panel, &dom));
    }

    #[test]
    fn matches_selector_descendant_combinator() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let selector = Selector {
            parts: vec![
                SelectorPart::Compound(CompoundSelector {
                    components: vec![SelectorComponent::Type("Container".into())],
                }),
                SelectorPart::Combinator(Combinator::Descendant),
                SelectorPart::Compound(CompoundSelector {
                    components: vec![SelectorComponent::Type("Button".into())],
                }),
            ],
        };
        assert!(matches_selector(&selector, btn, &dom));
    }

    #[test]
    fn matches_selector_empty_returns_false() {
        let (dom, _, _, _, btn, _) = build_test_dom();
        let selector = Selector { parts: vec![] };
        assert!(!matches_selector(&selector, btn, &dom));
    }
}
