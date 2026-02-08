//! logos-based CSS tokenizer.
//!
//! Token priority in logos is determined by:
//! 1. Longest match wins (e.g. `#fff` as HexColor beats `#` as Hash)
//! 2. For equal length matches, earlier-defined variants win
//!
//! Our ordering ensures:
//! - `#ff00aa` matches [`Token::HexColor`], not `Hash` + `Ident`
//! - `1fr` matches [`Token::Dimension`], not `Number` + `Ident`
//! - `:hover` matches [`Token::PseudoClass`], not `Colon` + `Ident`

use logos::Logos;

/// CSS token produced by the lexer.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r\f]+")]
pub enum Token {
    // ── Compound tokens (longer matches, defined first) ──────────────

    /// `!important` flag.
    #[token("!important")]
    Important,

    /// CSS hex color: `#fff`, `#ff00aa`, `#ff00aa80` (3-8 hex digits).
    #[regex(r"#[0-9a-fA-F]{3,8}")]
    HexColor,

    /// Dimension: number with unit suffix like `1fr`, `50%`, `10vw`, `80vh`.
    #[regex(r"-?[0-9]+(\.[0-9]+)?(fr|%|vw|vh)")]
    Dimension,

    /// Pseudo-class: `:hover`, `:focus`, `:disabled`, etc.
    #[regex(r":[a-zA-Z][a-zA-Z0-9_-]*")]
    PseudoClass,

    /// Double-quoted string literal.
    #[regex(r#""[^"]*""#)]
    StringLiteral,

    /// Single-quoted string literal.
    #[regex(r"'[^']*'")]
    StringLiteralSingle,

    /// CSS variable reference: `$primary`, `$bg-color`.
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_-]*")]
    Variable,

    /// Number: integer or float, possibly negative.
    #[regex(r"-?[0-9]+(\.[0-9]+)?")]
    Number,

    /// Identifier: property names, selector names, color names, etc.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_-]*")]
    Ident,

    // ── Single-character punctuation ─────────────────────────────────

    /// `{`
    #[token("{")]
    BraceOpen,

    /// `}`
    #[token("}")]
    BraceClose,

    /// `:`
    #[token(":")]
    Colon,

    /// `;`
    #[token(";")]
    Semicolon,

    /// `,`
    #[token(",")]
    Comma,

    /// `.`
    #[token(".")]
    Dot,

    /// `#`
    #[token("#")]
    Hash,

    /// `*`
    #[token("*")]
    Star,

    /// `>`
    #[token(">")]
    GreaterThan,
}

/// Tokenize a CSS string into a vector of `(Token, &str)` pairs.
///
/// Returns `None` for any token that fails to lex (logos error tokens are skipped).
pub fn tokenize(input: &str) -> Vec<(Token, String)> {
    let lexer = Token::lexer(input);
    lexer
        .spanned()
        .filter_map(|(result, span)| {
            result.ok().map(|token| (token, input[span].to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tokenize and return just the token variants.
    fn tokens(input: &str) -> Vec<Token> {
        tokenize(input).into_iter().map(|(t, _)| t).collect()
    }

    /// Helper: tokenize and return (token, slice) pairs.
    fn tokens_with_text(input: &str) -> Vec<(Token, String)> {
        tokenize(input)
    }

    // ── Basic punctuation ────────────────────────────────────────────

    #[test]
    fn test_punctuation() {
        assert_eq!(
            tokens("{ } : ; , . # * >"),
            vec![
                Token::BraceOpen,
                Token::BraceClose,
                Token::Colon,
                Token::Semicolon,
                Token::Comma,
                Token::Dot,
                Token::Hash,
                Token::Star,
                Token::GreaterThan,
            ]
        );
    }

    // ── Identifiers ──────────────────────────────────────────────────

    #[test]
    fn test_idents() {
        let result = tokens_with_text("color background my-widget _private");
        assert_eq!(result[0], (Token::Ident, "color".into()));
        assert_eq!(result[1], (Token::Ident, "background".into()));
        assert_eq!(result[2], (Token::Ident, "my-widget".into()));
        assert_eq!(result[3], (Token::Ident, "_private".into()));
    }

    // ── Numbers ──────────────────────────────────────────────────────

    #[test]
    fn test_numbers() {
        let result = tokens_with_text("10 -5 3.14 0");
        assert_eq!(result[0], (Token::Number, "10".into()));
        assert_eq!(result[1], (Token::Number, "-5".into()));
        assert_eq!(result[2], (Token::Number, "3.14".into()));
        assert_eq!(result[3], (Token::Number, "0".into()));
    }

    // ── Dimensions ───────────────────────────────────────────────────

    #[test]
    fn test_dimensions() {
        let result = tokens_with_text("1fr 50% 100vw 80vh");
        assert_eq!(result[0], (Token::Dimension, "1fr".into()));
        assert_eq!(result[1], (Token::Dimension, "50%".into()));
        assert_eq!(result[2], (Token::Dimension, "100vw".into()));
        assert_eq!(result[3], (Token::Dimension, "80vh".into()));
    }

    #[test]
    fn test_negative_dimension() {
        let result = tokens_with_text("-10%");
        assert_eq!(result[0], (Token::Dimension, "-10%".into()));
    }

    #[test]
    fn test_float_dimension() {
        let result = tokens_with_text("1.5fr");
        assert_eq!(result[0], (Token::Dimension, "1.5fr".into()));
    }

    // ── Hex colors ───────────────────────────────────────────────────

    #[test]
    fn test_hex_colors() {
        let result = tokens_with_text("#fff #ff00aa #ff00aa80");
        assert_eq!(result[0], (Token::HexColor, "#fff".into()));
        assert_eq!(result[1], (Token::HexColor, "#ff00aa".into()));
        assert_eq!(result[2], (Token::HexColor, "#ff00aa80".into()));
    }

    #[test]
    fn test_hex_color_priority_over_hash() {
        // #fff should be a single HexColor token, not Hash + Ident
        let result = tokens("#fff");
        assert_eq!(result, vec![Token::HexColor]);
    }

    #[test]
    fn test_hash_id_selector() {
        // #my-id: # is not followed by hex digits, so falls through to Hash + Ident
        let result = tokens("#my-id");
        assert_eq!(result, vec![Token::Hash, Token::Ident]);
    }

    // ── Pseudo-classes ───────────────────────────────────────────────

    #[test]
    fn test_pseudo_classes() {
        let result = tokens_with_text(":hover :focus :disabled");
        assert_eq!(result[0], (Token::PseudoClass, ":hover".into()));
        assert_eq!(result[1], (Token::PseudoClass, ":focus".into()));
        assert_eq!(result[2], (Token::PseudoClass, ":disabled".into()));
    }

    #[test]
    fn test_pseudo_class_priority_over_colon() {
        // :hover should be a single PseudoClass, not Colon + Ident
        let result = tokens(":hover");
        assert_eq!(result, vec![Token::PseudoClass]);
    }

    // ── Strings ──────────────────────────────────────────────────────

    #[test]
    fn test_string_literals() {
        let result = tokens_with_text(r#""hello" 'world'"#);
        assert_eq!(result[0], (Token::StringLiteral, "\"hello\"".into()));
        assert_eq!(result[1], (Token::StringLiteralSingle, "'world'".into()));
    }

    // ── Variables ────────────────────────────────────────────────────

    #[test]
    fn test_variables() {
        let result = tokens_with_text("$primary $bg-color $_internal");
        assert_eq!(result[0], (Token::Variable, "$primary".into()));
        assert_eq!(result[1], (Token::Variable, "$bg-color".into()));
        assert_eq!(result[2], (Token::Variable, "$_internal".into()));
    }

    // ── !important ───────────────────────────────────────────────────

    #[test]
    fn test_important() {
        let result = tokens("!important");
        assert_eq!(result, vec![Token::Important]);
    }

    // ── Dimension vs Number priority ─────────────────────────────────

    #[test]
    fn test_dimension_over_number() {
        // 1fr should be a single Dimension, not Number + Ident
        let result = tokens("1fr");
        assert_eq!(result, vec![Token::Dimension]);
    }

    #[test]
    fn test_plain_number_not_dimension() {
        // 42 without unit suffix should be Number
        let result = tokens("42");
        assert_eq!(result, vec![Token::Number]);
    }

    // ── Full CSS rule ────────────────────────────────────────────────

    #[test]
    fn test_full_css_rule() {
        let input = "Button.primary:hover { color: #fff; background: blue; }";
        let result = tokens_with_text(input);

        assert_eq!(result[0], (Token::Ident, "Button".into()));
        assert_eq!(result[1], (Token::Dot, ".".into()));
        assert_eq!(result[2], (Token::Ident, "primary".into()));
        assert_eq!(result[3], (Token::PseudoClass, ":hover".into()));
        assert_eq!(result[4], (Token::BraceOpen, "{".into()));
        assert_eq!(result[5], (Token::Ident, "color".into()));
        assert_eq!(result[6], (Token::Colon, ":".into()));
        assert_eq!(result[7], (Token::HexColor, "#fff".into()));
        assert_eq!(result[8], (Token::Semicolon, ";".into()));
        assert_eq!(result[9], (Token::Ident, "background".into()));
        assert_eq!(result[10], (Token::Colon, ":".into()));
        assert_eq!(result[11], (Token::Ident, "blue".into()));
        assert_eq!(result[12], (Token::Semicolon, ";".into()));
        assert_eq!(result[13], (Token::BraceClose, "}".into()));
    }

    #[test]
    fn test_complex_selector() {
        let input = "Container > Button.primary:hover, #sidebar .item";
        let result = tokens_with_text(input);

        assert_eq!(result[0], (Token::Ident, "Container".into()));
        assert_eq!(result[1], (Token::GreaterThan, ">".into()));
        assert_eq!(result[2], (Token::Ident, "Button".into()));
        assert_eq!(result[3], (Token::Dot, ".".into()));
        assert_eq!(result[4], (Token::Ident, "primary".into()));
        assert_eq!(result[5], (Token::PseudoClass, ":hover".into()));
        assert_eq!(result[6], (Token::Comma, ",".into()));
        assert_eq!(result[7], (Token::Hash, "#".into()));
        assert_eq!(result[8], (Token::Ident, "sidebar".into()));
        assert_eq!(result[9], (Token::Dot, ".".into()));
        assert_eq!(result[10], (Token::Ident, "item".into()));
    }

    #[test]
    fn test_declaration_with_important() {
        let input = "color: red !important;";
        let result = tokens(input);
        assert_eq!(
            result,
            vec![
                Token::Ident,
                Token::Colon,
                Token::Ident,
                Token::Important,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn test_declaration_with_dimensions() {
        let input = "margin: 1 2 50% 1fr;";
        let result = tokens_with_text(input);

        assert_eq!(result[0], (Token::Ident, "margin".into()));
        assert_eq!(result[1], (Token::Colon, ":".into()));
        assert_eq!(result[2], (Token::Number, "1".into()));
        assert_eq!(result[3], (Token::Number, "2".into()));
        assert_eq!(result[4], (Token::Dimension, "50%".into()));
        assert_eq!(result[5], (Token::Dimension, "1fr".into()));
        assert_eq!(result[6], (Token::Semicolon, ";".into()));
    }

    #[test]
    fn test_variable_in_declaration() {
        let input = "color: $primary;";
        let result = tokens_with_text(input);
        assert_eq!(result[0], (Token::Ident, "color".into()));
        assert_eq!(result[1], (Token::Colon, ":".into()));
        assert_eq!(result[2], (Token::Variable, "$primary".into()));
        assert_eq!(result[3], (Token::Semicolon, ";".into()));
    }

    #[test]
    fn test_whitespace_is_skipped() {
        let input = "  color  :  red  ;  ";
        let result = tokens(input);
        assert_eq!(
            result,
            vec![Token::Ident, Token::Colon, Token::Ident, Token::Semicolon]
        );
    }

    #[test]
    fn test_empty_input() {
        let result = tokens("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let result = tokens("   \t\n  ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_universal_selector() {
        let result = tokens("* { color: red; }");
        assert_eq!(
            result,
            vec![
                Token::Star,
                Token::BraceOpen,
                Token::Ident,
                Token::Colon,
                Token::Ident,
                Token::Semicolon,
                Token::BraceClose,
            ]
        );
    }

    #[test]
    fn test_six_digit_hex() {
        let result = tokens_with_text("#abcdef");
        assert_eq!(result[0], (Token::HexColor, "#abcdef".into()));
    }

    #[test]
    fn test_eight_digit_hex_rgba() {
        let result = tokens_with_text("#aabbccdd");
        assert_eq!(result[0], (Token::HexColor, "#aabbccdd".into()));
    }
}
