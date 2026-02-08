//! Recursive descent CSS parser.
//!
//! Parses CSS text into a [`StyleSheet`] (a vector of [`RuleSet`]s). Uses the
//! logos-based tokenizer from [`crate::css::tokenizer`].

use logos::Logos;

use crate::css::model::*;
use crate::css::tokenizer::Token;

/// Errors from CSS parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected token at position {position}: {message}")]
    UnexpectedToken { position: usize, message: String },
    #[error("unexpected end of input: {0}")]
    UnexpectedEof(String),
}

/// A positioned token with byte-level span information for whitespace detection.
#[derive(Debug, Clone)]
struct PToken {
    token: Token,
    text: String,
    /// Index in the token stream (for error reporting).
    pos: usize,
    /// Byte offset where this token starts in the source.
    byte_start: usize,
    /// Byte offset where this token ends in the source.
    byte_end: usize,
}

/// Strip CSS block comments (`/* ... */`) from the input, replacing each
/// comment with a single space.
fn strip_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Start of block comment — scan for */
            i += 2;
            let mut found_end = false;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    found_end = true;
                    break;
                }
                i += 1;
            }
            if !found_end {
                // Unterminated comment — consume the rest of the input.
                i = len;
            }
            result.push(' ');
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Tokenize input using logos with span information preserved.
fn tokenize_with_spans(input: &str) -> Vec<PToken> {
    let lexer = Token::lexer(input);
    let mut tokens = Vec::new();
    let mut idx = 0;

    for (result, span) in lexer.spanned() {
        if let Ok(token) = result {
            tokens.push(PToken {
                text: input[span.clone()].to_string(),
                token,
                pos: idx,
                byte_start: span.start,
                byte_end: span.end,
            });
            idx += 1;
        }
    }

    tokens
}

/// Parse a CSS string into a [`StyleSheet`].
pub fn parse_css(input: &str) -> Result<StyleSheet, ParseError> {
    let cleaned = strip_comments(input);
    let tokens = tokenize_with_spans(&cleaned);

    let mut parser = Parser { tokens, cursor: 0 };

    let mut rules = Vec::new();
    while !parser.is_eof() {
        rules.push(parser.parse_rule()?);
    }

    Ok(StyleSheet { rules })
}

/// Recursive descent parser state.
struct Parser {
    tokens: Vec<PToken>,
    cursor: usize,
}

impl Parser {
    fn is_eof(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn peek(&self) -> Option<&PToken> {
        self.tokens.get(self.cursor)
    }

    fn advance(&mut self) -> Option<&PToken> {
        if self.cursor < self.tokens.len() {
            let tok = &self.tokens[self.cursor];
            self.cursor += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<PToken, ParseError> {
        match self.advance() {
            Some(tok) if &tok.token == expected => Ok(tok.clone()),
            Some(tok) => Err(ParseError::UnexpectedToken {
                position: tok.pos,
                message: format!(
                    "expected {:?}, got {:?} '{}'",
                    expected, tok.token, tok.text
                ),
            }),
            None => Err(ParseError::UnexpectedEof(format!(
                "expected {:?}",
                expected
            ))),
        }
    }

    fn current_pos(&self) -> usize {
        self.peek().map(|t| t.pos).unwrap_or(self.tokens.len())
    }

    /// Returns `true` if the current token is immediately adjacent (no whitespace)
    /// to the previous token.
    fn is_adjacent(&self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let prev = &self.tokens[self.cursor - 1];
        match self.peek() {
            Some(curr) => curr.byte_start == prev.byte_end,
            None => false,
        }
    }

    /// Parse a single CSS rule: selector(s) `{` declarations `}`.
    fn parse_rule(&mut self) -> Result<RuleSet, ParseError> {
        let selectors = self.parse_selector_list()?;
        self.expect(&Token::BraceOpen)?;
        let declarations = self.parse_declarations()?;
        self.expect(&Token::BraceClose)?;

        Ok(RuleSet {
            selectors,
            declarations,
        })
    }

    /// Parse a comma-separated list of selectors (before `{`).
    fn parse_selector_list(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();

        selectors.push(self.parse_selector()?);

        while self.peek().is_some_and(|t| t.token == Token::Comma) {
            self.advance(); // consume comma
            selectors.push(self.parse_selector()?);
        }

        Ok(selectors)
    }

    /// Parse a single selector: a sequence of compound selectors with combinators.
    ///
    /// A selector like `Container > Button.primary:hover` becomes parts:
    /// - SelectorPart::Compound(CompoundSelector [Type("Container")])
    /// - SelectorPart::Combinator(Child)
    /// - SelectorPart::Compound(CompoundSelector [Type("Button"), Class("primary"), PseudoClass("hover")])
    fn parse_selector(&mut self) -> Result<Selector, ParseError> {
        let mut parts = Vec::new();

        // Parse first compound selector
        parts.push(SelectorPart::Compound(self.parse_compound_selector()?));

        // Parse additional combinator + compound pairs
        loop {
            match self.peek() {
                // `>` means child combinator
                Some(t) if t.token == Token::GreaterThan => {
                    self.advance();
                    parts.push(SelectorPart::Combinator(Combinator::Child));
                    parts.push(SelectorPart::Compound(self.parse_compound_selector()?));
                }
                // If we see a selector-starting token that is NOT adjacent to the
                // previous token (i.e., there was whitespace), it's a descendant
                // combinator. If it IS adjacent, the parse_compound_selector would
                // have already consumed it.
                Some(t)
                    if matches!(
                        t.token,
                        Token::Ident
                            | Token::Hash
                            | Token::Dot
                            | Token::Star
                            | Token::PseudoClass
                    ) =>
                {
                    parts.push(SelectorPart::Combinator(Combinator::Descendant));
                    parts.push(SelectorPart::Compound(self.parse_compound_selector()?));
                }
                // Anything else ends this selector
                _ => break,
            }
        }

        Ok(Selector { parts })
    }

    /// Parse a compound selector: a sequence of simple selector components with
    /// no whitespace between them, e.g. `Button.primary:hover`.
    ///
    /// Uses span-based adjacency detection: `.class`, `#id`, and `:pseudo` are
    /// only appended to the current compound if they appear immediately after the
    /// previous token (no whitespace gap).
    fn parse_compound_selector(&mut self) -> Result<CompoundSelector, ParseError> {
        let mut components = Vec::new();

        // Parse the first part of the compound (type, universal, class, id, or pseudo-class)
        match self.peek() {
            Some(t) if t.token == Token::Ident => {
                let name = t.text.clone();
                self.advance();
                components.push(SelectorComponent::Type(name));
            }
            Some(t) if t.token == Token::Star => {
                self.advance();
                components.push(SelectorComponent::Universal);
            }
            Some(t) if t.token == Token::Dot => {
                self.advance();
                let name_tok = self.advance().ok_or_else(|| {
                    ParseError::UnexpectedEof("expected class name after '.'".into())
                })?;
                if name_tok.token != Token::Ident {
                    return Err(ParseError::UnexpectedToken {
                        position: name_tok.pos,
                        message: format!(
                            "expected class name, got {:?} '{}'",
                            name_tok.token, name_tok.text
                        ),
                    });
                }
                components.push(SelectorComponent::Class(name_tok.text.clone()));
            }
            Some(t) if t.token == Token::Hash => {
                self.advance();
                let name_tok = self.advance().ok_or_else(|| {
                    ParseError::UnexpectedEof("expected id name after '#'".into())
                })?;
                if name_tok.token != Token::Ident {
                    return Err(ParseError::UnexpectedToken {
                        position: name_tok.pos,
                        message: format!(
                            "expected id name, got {:?} '{}'",
                            name_tok.token, name_tok.text
                        ),
                    });
                }
                components.push(SelectorComponent::Id(name_tok.text.clone()));
            }
            Some(t) if t.token == Token::PseudoClass => {
                let name = t.text[1..].to_string();
                self.advance();
                components.push(SelectorComponent::PseudoClass(name));
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    position: self.current_pos(),
                    message: "expected selector part".into(),
                });
            }
        }

        // Continue appending to this compound only if the next token is adjacent
        // (no whitespace gap).
        loop {
            if !self.is_adjacent() {
                break;
            }

            match self.peek() {
                Some(t) if t.token == Token::Dot => {
                    self.advance();
                    let name_tok = self.advance().ok_or_else(|| {
                        ParseError::UnexpectedEof("expected class name after '.'".into())
                    })?;
                    if name_tok.token != Token::Ident {
                        return Err(ParseError::UnexpectedToken {
                            position: name_tok.pos,
                            message: format!(
                                "expected class name, got {:?} '{}'",
                                name_tok.token, name_tok.text
                            ),
                        });
                    }
                    components.push(SelectorComponent::Class(name_tok.text.clone()));
                }
                Some(t) if t.token == Token::Hash => {
                    self.advance();
                    let name_tok = self.advance().ok_or_else(|| {
                        ParseError::UnexpectedEof("expected id name after '#'".into())
                    })?;
                    if name_tok.token != Token::Ident {
                        return Err(ParseError::UnexpectedToken {
                            position: name_tok.pos,
                            message: format!(
                                "expected id name, got {:?} '{}'",
                                name_tok.token, name_tok.text
                            ),
                        });
                    }
                    components.push(SelectorComponent::Id(name_tok.text.clone()));
                }
                Some(t) if t.token == Token::PseudoClass => {
                    let name = t.text[1..].to_string();
                    self.advance();
                    components.push(SelectorComponent::PseudoClass(name));
                }
                _ => break,
            }
        }

        if components.is_empty() {
            return Err(ParseError::UnexpectedToken {
                position: self.current_pos(),
                message: "expected selector part".into(),
            });
        }

        Ok(CompoundSelector { components })
    }

    /// Parse declarations between `{` and `}`.
    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut declarations = Vec::new();

        while self.peek().is_some_and(|t| t.token != Token::BraceClose) {
            declarations.push(self.parse_declaration()?);
        }

        Ok(declarations)
    }

    /// Parse a single declaration: `property: value1 value2 [!important];`
    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        // Property name
        let prop_tok = self.advance().ok_or_else(|| {
            ParseError::UnexpectedEof("expected property name".into())
        })?;
        if prop_tok.token != Token::Ident {
            return Err(ParseError::UnexpectedToken {
                position: prop_tok.pos,
                message: format!(
                    "expected property name, got {:?} '{}'",
                    prop_tok.token, prop_tok.text
                ),
            });
        }
        let property = prop_tok.text.clone();

        // Colon
        self.expect(&Token::Colon)?;

        // Values (until `;` or `}` or `!important`)
        let mut values = Vec::new();
        let mut important = false;

        loop {
            match self.peek() {
                None
                | Some(PToken {
                    token: Token::Semicolon,
                    ..
                })
                | Some(PToken {
                    token: Token::BraceClose,
                    ..
                }) => break,
                Some(PToken {
                    token: Token::Important,
                    ..
                }) => {
                    self.advance();
                    important = true;
                    break;
                }
                Some(_) => {
                    values.push(self.parse_declaration_value()?);
                }
            }
        }

        // Consume optional semicolon
        if self.peek().is_some_and(|t| t.token == Token::Semicolon) {
            self.advance();
        }

        Ok(Declaration {
            property,
            values,
            important,
        })
    }

    /// Parse a single declaration value token into a [`DeclarationValue`].
    fn parse_declaration_value(&mut self) -> Result<DeclarationValue, ParseError> {
        let tok = self.advance().ok_or_else(|| {
            ParseError::UnexpectedEof("expected declaration value".into())
        })?;

        match &tok.token {
            Token::Number => {
                let n: f32 = tok.text.parse().map_err(|_| ParseError::UnexpectedToken {
                    position: tok.pos,
                    message: format!("invalid number: {}", tok.text),
                })?;
                Ok(DeclarationValue::Number(n))
            }
            Token::Dimension => {
                let text = &tok.text;
                let (num_str, unit_str) =
                    split_dimension(text).ok_or_else(|| ParseError::UnexpectedToken {
                        position: tok.pos,
                        message: format!("invalid dimension: {text}"),
                    })?;
                let n: f32 =
                    num_str
                        .parse()
                        .map_err(|_| ParseError::UnexpectedToken {
                            position: tok.pos,
                            message: format!("invalid number in dimension: {num_str}"),
                        })?;
                Ok(DeclarationValue::Dimension(n, unit_str.to_string()))
            }
            Token::Ident => Ok(DeclarationValue::Ident(tok.text.clone())),
            Token::HexColor => {
                // Strip the leading '#' for DeclarationValue::Color
                let hex = tok.text.strip_prefix('#').unwrap_or(&tok.text);
                Ok(DeclarationValue::Color(hex.to_string()))
            }
            Token::StringLiteral | Token::StringLiteralSingle => {
                // Strip surrounding quotes
                let inner = &tok.text[1..tok.text.len() - 1];
                Ok(DeclarationValue::String(inner.to_string()))
            }
            Token::Variable => {
                // Strip the leading `$`
                let name = tok.text.strip_prefix('$').unwrap_or(&tok.text);
                Ok(DeclarationValue::Variable(name.to_string()))
            }
            other => Err(ParseError::UnexpectedToken {
                position: tok.pos,
                message: format!(
                    "unexpected token in declaration value: {:?} '{}'",
                    other, tok.text
                ),
            }),
        }
    }
}

/// Split a dimension string like "50%" or "1fr" into (number_part, unit_part).
fn split_dimension(s: &str) -> Option<(&str, &str)> {
    let unit_start = s
        .char_indices()
        .find(|(i, c)| !c.is_ascii_digit() && *c != '.' && !(*c == '-' && *i == 0))
        .map(|(i, _)| i)?;

    if unit_start == 0 || unit_start >= s.len() {
        return None;
    }

    Some((&s[..unit_start], &s[unit_start..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ───────────────────────────────────────────────────────

    fn parse(input: &str) -> StyleSheet {
        parse_css(input).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    fn first_rule(input: &str) -> RuleSet {
        let sheet = parse(input);
        assert!(!sheet.rules.is_empty(), "expected at least one rule");
        sheet.rules.into_iter().next().unwrap()
    }

    /// Extract the first compound selector's components from a selector.
    fn first_compound(sel: &Selector) -> &[SelectorComponent] {
        match &sel.parts[0] {
            SelectorPart::Compound(c) => &c.components,
            _ => panic!("expected compound selector at index 0"),
        }
    }

    // ── Simple rule ──────────────────────────────────────────────────

    #[test]
    fn parse_simple_rule() {
        let rule = first_rule("Button { color: red; }");
        assert_eq!(rule.selectors.len(), 1);
        assert_eq!(rule.declarations.len(), 1);

        let comps = first_compound(&rule.selectors[0]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0], SelectorComponent::Type("Button".into()));

        let decl = &rule.declarations[0];
        assert_eq!(decl.property, "color");
        assert!(!decl.important);
        assert_eq!(decl.values.len(), 1);
        assert_eq!(decl.values[0], DeclarationValue::Ident("red".into()));
    }

    // ── Compound selector (no whitespace between parts) ──────────────

    #[test]
    fn parse_compound_selector() {
        let rule = first_rule("Button.primary:hover { color: blue; }");
        let comps = first_compound(&rule.selectors[0]);
        assert_eq!(comps.len(), 3);
        assert_eq!(comps[0], SelectorComponent::Type("Button".into()));
        assert_eq!(comps[1], SelectorComponent::Class("primary".into()));
        assert_eq!(comps[2], SelectorComponent::PseudoClass("hover".into()));
    }

    // ── Descendant combinator ────────────────────────────────────────

    #[test]
    fn parse_descendant_combinator() {
        let rule = first_rule("Container Panel { margin: 1; }");
        let sel = &rule.selectors[0];
        assert_eq!(sel.parts.len(), 3);

        match &sel.parts[0] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components[0], SelectorComponent::Type("Container".into()));
            }
            _ => panic!("expected compound"),
        }
        assert_eq!(
            sel.parts[1],
            SelectorPart::Combinator(Combinator::Descendant)
        );
        match &sel.parts[2] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components[0], SelectorComponent::Type("Panel".into()));
            }
            _ => panic!("expected compound"),
        }
    }

    // ── Child combinator ─────────────────────────────────────────────

    #[test]
    fn parse_child_combinator() {
        let rule = first_rule("Container > Button { padding: 1 2; }");
        let sel = &rule.selectors[0];
        assert_eq!(sel.parts.len(), 3);
        assert_eq!(
            sel.parts[1],
            SelectorPart::Combinator(Combinator::Child)
        );
    }

    // ── Multiple selectors ───────────────────────────────────────────

    #[test]
    fn parse_multiple_selectors() {
        let rule = first_rule("Button, Label { color: green; }");
        assert_eq!(rule.selectors.len(), 2);

        let comps0 = first_compound(&rule.selectors[0]);
        assert_eq!(comps0[0], SelectorComponent::Type("Button".into()));

        let comps1 = first_compound(&rule.selectors[1]);
        assert_eq!(comps1[0], SelectorComponent::Type("Label".into()));
    }

    // ── Multiple declarations ────────────────────────────────────────

    #[test]
    fn parse_multiple_declarations() {
        let rule = first_rule("Button { color: red; background: blue; text-align: center; }");
        assert_eq!(rule.declarations.len(), 3);
        assert_eq!(rule.declarations[0].property, "color");
        assert_eq!(rule.declarations[1].property, "background");
        assert_eq!(rule.declarations[2].property, "text-align");
    }

    // ── Dimensions ───────────────────────────────────────────────────

    #[test]
    fn parse_dimensions() {
        let rule = first_rule("Panel { width: 50%; height: 1fr; min-width: 10; }");
        assert_eq!(rule.declarations.len(), 3);

        assert_eq!(rule.declarations[0].property, "width");
        assert_eq!(
            rule.declarations[0].values[0],
            DeclarationValue::Dimension(50.0, "%".into())
        );

        assert_eq!(rule.declarations[1].property, "height");
        assert_eq!(
            rule.declarations[1].values[0],
            DeclarationValue::Dimension(1.0, "fr".into())
        );

        assert_eq!(rule.declarations[2].property, "min-width");
        assert_eq!(
            rule.declarations[2].values[0],
            DeclarationValue::Number(10.0)
        );
    }

    // ── !important ───────────────────────────────────────────────────

    #[test]
    fn parse_important() {
        let rule = first_rule("Button { color: red !important; }");
        assert_eq!(rule.declarations.len(), 1);
        assert!(rule.declarations[0].important);
        assert_eq!(
            rule.declarations[0].values[0],
            DeclarationValue::Ident("red".into())
        );
    }

    // ── Hex colors ───────────────────────────────────────────────────

    #[test]
    fn parse_hex_colors() {
        let rule = first_rule("Label { color: #ff0000; background: #fff; }");
        assert_eq!(rule.declarations.len(), 2);
        assert_eq!(
            rule.declarations[0].values[0],
            DeclarationValue::Color("ff0000".into())
        );
        assert_eq!(
            rule.declarations[1].values[0],
            DeclarationValue::Color("fff".into())
        );
    }

    // ── Comments ─────────────────────────────────────────────────────

    #[test]
    fn parse_with_comments() {
        let input = "/* comment */ Button { color: red; /* inline */ background: blue; }";
        let rule = first_rule(input);
        assert_eq!(rule.declarations.len(), 2);
        assert_eq!(rule.declarations[0].property, "color");
        assert_eq!(rule.declarations[1].property, "background");
    }

    #[test]
    fn parse_comment_between_rules() {
        let input = "Button { color: red; } /* between */ Label { color: blue; }";
        let sheet = parse(input);
        assert_eq!(sheet.rules.len(), 2);
    }

    // ── Error handling ───────────────────────────────────────────────

    #[test]
    fn parse_unclosed_brace() {
        let result = parse_css("Button { color: red;");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_input() {
        let sheet = parse("");
        assert!(sheet.rules.is_empty());
    }

    // ── Multiple rules ───────────────────────────────────────────────

    #[test]
    fn parse_multiple_rules() {
        let sheet = parse("Button { color: red; } Label { color: blue; }");
        assert_eq!(sheet.rules.len(), 2);
    }

    // ── Universal selector ───────────────────────────────────────────

    #[test]
    fn parse_universal_selector() {
        let rule = first_rule("* { color: white; }");
        let comps = first_compound(&rule.selectors[0]);
        assert_eq!(comps[0], SelectorComponent::Universal);
    }

    // ── ID selector ──────────────────────────────────────────────────

    #[test]
    fn parse_id_selector() {
        let rule = first_rule("#sidebar { color: gray; }");
        let comps = first_compound(&rule.selectors[0]);
        assert_eq!(comps[0], SelectorComponent::Id("sidebar".into()));
    }

    // ── Class-only selector ──────────────────────────────────────────

    #[test]
    fn parse_class_only_selector() {
        let rule = first_rule(".primary { color: blue; }");
        let comps = first_compound(&rule.selectors[0]);
        assert_eq!(comps[0], SelectorComponent::Class("primary".into()));
    }

    // ── Margin shorthand ─────────────────────────────────────────────

    #[test]
    fn parse_margin_shorthand() {
        let rule = first_rule("Panel { margin: 1 2 3 4; }");
        let decl = &rule.declarations[0];
        assert_eq!(decl.property, "margin");
        assert_eq!(decl.values.len(), 4);
        assert_eq!(decl.values[0], DeclarationValue::Number(1.0));
        assert_eq!(decl.values[1], DeclarationValue::Number(2.0));
        assert_eq!(decl.values[2], DeclarationValue::Number(3.0));
        assert_eq!(decl.values[3], DeclarationValue::Number(4.0));
    }

    // ── Declaration without semicolon (last before }) ────────────────

    #[test]
    fn parse_declaration_without_trailing_semicolon() {
        let rule = first_rule("Button { color: red }");
        assert_eq!(rule.declarations.len(), 1);
        assert_eq!(rule.declarations[0].property, "color");
    }

    // ── Complex selectors ────────────────────────────────────────────

    #[test]
    fn parse_complex_selector_chain() {
        let rule = first_rule("Container > Panel .item:hover { color: red; }");
        let sel = &rule.selectors[0];
        // Container > Panel <descendant> .item:hover
        // = [Container] > [Panel] descendant [.item:hover]
        assert_eq!(sel.parts.len(), 5);

        match &sel.parts[0] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components[0], SelectorComponent::Type("Container".into()));
            }
            _ => panic!("expected compound"),
        }
        assert_eq!(
            sel.parts[1],
            SelectorPart::Combinator(Combinator::Child)
        );
        match &sel.parts[2] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components.len(), 1);
                assert_eq!(c.components[0], SelectorComponent::Type("Panel".into()));
            }
            _ => panic!("expected compound"),
        }
        assert_eq!(
            sel.parts[3],
            SelectorPart::Combinator(Combinator::Descendant)
        );
        match &sel.parts[4] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components.len(), 2);
                assert_eq!(c.components[0], SelectorComponent::Class("item".into()));
                assert_eq!(
                    c.components[1],
                    SelectorComponent::PseudoClass("hover".into())
                );
            }
            _ => panic!("expected compound"),
        }
    }

    /// Verify that `Panel.item` (no space) is a single compound selector,
    /// while `Panel .item` (space) produces two compounds with a descendant combinator.
    #[test]
    fn whitespace_distinguishes_compound_from_descendant() {
        // No space: single compound
        let rule = first_rule("Panel.item { color: red; }");
        let sel = &rule.selectors[0];
        assert_eq!(sel.parts.len(), 1);
        match &sel.parts[0] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components.len(), 2);
                assert_eq!(c.components[0], SelectorComponent::Type("Panel".into()));
                assert_eq!(c.components[1], SelectorComponent::Class("item".into()));
            }
            _ => panic!("expected compound"),
        }

        // With space: descendant combinator
        let rule = first_rule("Panel .item { color: red; }");
        let sel = &rule.selectors[0];
        assert_eq!(sel.parts.len(), 3);
        match &sel.parts[0] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components.len(), 1);
                assert_eq!(c.components[0], SelectorComponent::Type("Panel".into()));
            }
            _ => panic!("expected compound"),
        }
        assert_eq!(
            sel.parts[1],
            SelectorPart::Combinator(Combinator::Descendant)
        );
        match &sel.parts[2] {
            SelectorPart::Compound(c) => {
                assert_eq!(c.components.len(), 1);
                assert_eq!(c.components[0], SelectorComponent::Class("item".into()));
            }
            _ => panic!("expected compound"),
        }
    }

    // ── strip_comments ───────────────────────────────────────────────

    #[test]
    fn strip_comments_basic() {
        let result = strip_comments("a /* comment */ b");
        // The space before /* + replacement space + space after */ = 3 spaces
        assert_eq!(result, "a   b");
    }

    #[test]
    fn strip_comments_multiple() {
        let result = strip_comments("/* c1 */ a /* c2 */ b /* c3 */");
        // Each comment becomes one space; surrounding spaces preserved
        assert_eq!(result, "  a   b  ");
    }

    #[test]
    fn strip_comments_no_comments() {
        assert_eq!(strip_comments("hello world"), "hello world");
    }

    #[test]
    fn strip_comments_unterminated() {
        let result = strip_comments("a /* unterminated");
        // Unterminated comment consumes rest of input, replaced by single space
        assert_eq!(result, "a  ");
    }

    // ── split_dimension ──────────────────────────────────────────────

    #[test]
    fn split_dimension_percent() {
        assert_eq!(split_dimension("50%"), Some(("50", "%")));
    }

    #[test]
    fn split_dimension_fr() {
        assert_eq!(split_dimension("1fr"), Some(("1", "fr")));
    }

    #[test]
    fn split_dimension_vw() {
        assert_eq!(split_dimension("100vw"), Some(("100", "vw")));
    }

    #[test]
    fn split_dimension_negative() {
        assert_eq!(split_dimension("-10%"), Some(("-10", "%")));
    }

    #[test]
    fn split_dimension_float() {
        assert_eq!(split_dimension("1.5fr"), Some(("1.5", "fr")));
    }
}
