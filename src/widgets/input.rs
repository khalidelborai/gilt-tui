//! Input widget: a focusable text input field.
//!
//! Supports cursor movement, character insertion/deletion, placeholder text,
//! and password masking mode.

use std::any::Any;

use crate::css::styles::Styles;
use crate::geometry::Region;
use crate::render::strip::{CellStyle, Strip};
use crate::widget::traits::Widget;

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// A text input widget with cursor, placeholder, and password support.
///
/// The cursor position is tracked as a byte offset into the value string.
/// All cursor operations are char-boundary safe.
///
/// # Examples
///
/// ```ignore
/// let input = Input::new()
///     .with_placeholder("Enter your name...")
///     .with_value("Alice");
/// ```
pub struct Input {
    value: String,
    placeholder: String,
    cursor_position: usize,
    password: bool,
}

impl Input {
    /// Create a new empty input.
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            cursor_position: 0,
            password: false,
        }
    }

    /// Set the placeholder text (builder pattern).
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the initial value (builder pattern).
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor_position = self.value.len();
        self
    }

    /// Enable or disable password masking (builder pattern).
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Return the current value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value, moving the cursor to the end.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor_position = self.value.len();
    }

    /// Clear the input value and reset the cursor.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    /// Insert a character at the current cursor position.
    pub fn insert_char(&mut self, ch: char) {
        self.value.insert(self.cursor_position, ch);
        self.cursor_position += ch.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_char(&mut self) {
        if self.cursor_position == 0 {
            return;
        }
        // Find the previous char boundary.
        let prev = self.prev_char_boundary();
        self.value.drain(prev..self.cursor_position);
        self.cursor_position = prev;
    }

    /// Delete the character after the cursor (delete forward).
    pub fn delete_forward(&mut self) {
        if self.cursor_position >= self.value.len() {
            return;
        }
        let next = self.next_char_boundary();
        self.value.drain(self.cursor_position..next);
    }

    /// Move the cursor left by one character.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position = self.prev_char_boundary();
        }
    }

    /// Move the cursor right by one character.
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            self.cursor_position = self.next_char_boundary();
        }
    }

    /// Move the cursor to the start of the input.
    pub fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move the cursor to the end of the input.
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    /// Return the cursor position (byte offset).
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Find the byte offset of the previous character boundary.
    fn prev_char_boundary(&self) -> usize {
        let mut pos = self.cursor_position.saturating_sub(1);
        while pos > 0 && !self.value.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    /// Find the byte offset of the next character boundary.
    fn next_char_boundary(&self) -> usize {
        let mut pos = self.cursor_position + 1;
        while pos < self.value.len() && !self.value.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }

    /// Display string: either the value (possibly masked) or the placeholder.
    fn display_text(&self) -> String {
        if self.value.is_empty() {
            self.placeholder.clone()
        } else if self.password {
            // One dot per character
            "\u{2022}".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Input {
    fn widget_type(&self) -> &str {
        "Input"
    }

    fn default_css(&self) -> &str {
        "Input { height: 1; width: 1fr; }"
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn render(&self, region: Region, styles: &Styles) -> Vec<Strip> {
        if region.width <= 0 || region.height <= 0 {
            return Vec::new();
        }

        let width = region.width as usize;
        let mut style = CellStyle::from_styles(styles);
        let display = self.display_text();
        let is_placeholder = self.value.is_empty() && !self.placeholder.is_empty();

        // Placeholder text is rendered dim.
        if is_placeholder {
            style.dim = true;
        }

        let mut strip = Strip::new(region.y, region.x);
        let truncated: String = display.chars().take(width).collect();
        strip.push_str(&truncated, style.clone());

        // Reset dim for fill padding if we used it for placeholder.
        if is_placeholder {
            style.dim = false;
        }
        strip.fill(region.width, style);

        vec![strip]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn region(w: i32, h: i32) -> Region {
        Region::new(0, 0, w, h)
    }

    fn styles() -> Styles {
        Styles::new()
    }

    // -----------------------------------------------------------------------
    // Widget trait
    // -----------------------------------------------------------------------

    #[test]
    fn widget_type_is_input() {
        let i = Input::new();
        assert_eq!(i.widget_type(), "Input");
    }

    #[test]
    fn default_css_has_expected_props() {
        let i = Input::new();
        assert!(i.default_css().contains("height: 1"));
        assert!(i.default_css().contains("width: 1fr"));
    }

    #[test]
    fn can_focus_is_true() {
        let i = Input::new();
        assert!(i.can_focus());
    }

    // -----------------------------------------------------------------------
    // Builder
    // -----------------------------------------------------------------------

    #[test]
    fn with_value_sets_cursor_to_end() {
        let i = Input::new().with_value("hello");
        assert_eq!(i.value(), "hello");
        assert_eq!(i.cursor_position(), 5);
    }

    #[test]
    fn with_placeholder() {
        let i = Input::new().with_placeholder("Type here...");
        let strips = i.render(region(20, 1), &styles());
        let text: String = strips[0].cells.iter().take(12).map(|c| c.ch).collect();
        assert_eq!(text, "Type here...");
        // Placeholder is rendered dim
        assert!(strips[0].cells[0].style.dim);
    }

    #[test]
    fn password_mode() {
        let i = Input::new().with_value("secret").password(true);
        let strips = i.render(region(20, 1), &styles());
        // Should render 6 dots (bullet chars)
        assert_eq!(strips[0].cells[0].ch, '\u{2022}');
        assert_eq!(strips[0].cells[5].ch, '\u{2022}');
    }

    // -----------------------------------------------------------------------
    // Cursor movement
    // -----------------------------------------------------------------------

    #[test]
    fn move_cursor_left() {
        let mut i = Input::new().with_value("abc");
        assert_eq!(i.cursor_position(), 3);
        i.move_cursor_left();
        assert_eq!(i.cursor_position(), 2);
        i.move_cursor_left();
        assert_eq!(i.cursor_position(), 1);
    }

    #[test]
    fn move_cursor_left_at_start() {
        let mut i = Input::new().with_value("abc");
        i.move_cursor_home();
        i.move_cursor_left(); // should not underflow
        assert_eq!(i.cursor_position(), 0);
    }

    #[test]
    fn move_cursor_right() {
        let mut i = Input::new().with_value("abc");
        i.move_cursor_home();
        i.move_cursor_right();
        assert_eq!(i.cursor_position(), 1);
    }

    #[test]
    fn move_cursor_right_at_end() {
        let mut i = Input::new().with_value("abc");
        i.move_cursor_right(); // already at end
        assert_eq!(i.cursor_position(), 3);
    }

    #[test]
    fn move_cursor_home_and_end() {
        let mut i = Input::new().with_value("hello");
        i.move_cursor_home();
        assert_eq!(i.cursor_position(), 0);
        i.move_cursor_end();
        assert_eq!(i.cursor_position(), 5);
    }

    // -----------------------------------------------------------------------
    // Insertion
    // -----------------------------------------------------------------------

    #[test]
    fn insert_char_at_end() {
        let mut i = Input::new().with_value("ab");
        i.insert_char('c');
        assert_eq!(i.value(), "abc");
        assert_eq!(i.cursor_position(), 3);
    }

    #[test]
    fn insert_char_at_start() {
        let mut i = Input::new().with_value("bc");
        i.move_cursor_home();
        i.insert_char('a');
        assert_eq!(i.value(), "abc");
        assert_eq!(i.cursor_position(), 1);
    }

    #[test]
    fn insert_char_in_middle() {
        let mut i = Input::new().with_value("ac");
        i.move_cursor_home();
        i.move_cursor_right();
        i.insert_char('b');
        assert_eq!(i.value(), "abc");
        assert_eq!(i.cursor_position(), 2);
    }

    // -----------------------------------------------------------------------
    // Deletion
    // -----------------------------------------------------------------------

    #[test]
    fn delete_char_backspace() {
        let mut i = Input::new().with_value("abc");
        i.delete_char();
        assert_eq!(i.value(), "ab");
        assert_eq!(i.cursor_position(), 2);
    }

    #[test]
    fn delete_char_at_start_does_nothing() {
        let mut i = Input::new().with_value("abc");
        i.move_cursor_home();
        i.delete_char();
        assert_eq!(i.value(), "abc");
        assert_eq!(i.cursor_position(), 0);
    }

    #[test]
    fn delete_forward() {
        let mut i = Input::new().with_value("abc");
        i.move_cursor_home();
        i.delete_forward();
        assert_eq!(i.value(), "bc");
        assert_eq!(i.cursor_position(), 0);
    }

    #[test]
    fn delete_forward_at_end_does_nothing() {
        let mut i = Input::new().with_value("abc");
        i.delete_forward();
        assert_eq!(i.value(), "abc");
    }

    // -----------------------------------------------------------------------
    // Set / Clear
    // -----------------------------------------------------------------------

    #[test]
    fn set_value() {
        let mut i = Input::new().with_value("old");
        i.set_value("new");
        assert_eq!(i.value(), "new");
        assert_eq!(i.cursor_position(), 3);
    }

    #[test]
    fn clear() {
        let mut i = Input::new().with_value("abc");
        i.clear();
        assert_eq!(i.value(), "");
        assert_eq!(i.cursor_position(), 0);
    }

    // -----------------------------------------------------------------------
    // Unicode
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_insert_and_delete() {
        let mut i = Input::new();
        i.insert_char('a');
        i.insert_char('\u{00e9}'); // e-acute, 2 bytes
        i.insert_char('b');
        assert_eq!(i.value(), "a\u{00e9}b");
        i.delete_char(); // remove 'b'
        assert_eq!(i.value(), "a\u{00e9}");
        i.delete_char(); // remove e-acute
        assert_eq!(i.value(), "a");
    }

    #[test]
    fn unicode_cursor_movement() {
        let mut i = Input::new().with_value("a\u{00e9}b"); // 4 bytes: a(1) + e-acute(2) + b(1)
        assert_eq!(i.cursor_position(), 4);
        i.move_cursor_left(); // before 'b' -> byte 3
        assert_eq!(i.cursor_position(), 3);
        i.move_cursor_left(); // before e-acute -> byte 1
        assert_eq!(i.cursor_position(), 1);
        i.move_cursor_left(); // before 'a' -> byte 0
        assert_eq!(i.cursor_position(), 0);
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    #[test]
    fn render_value() {
        let i = Input::new().with_value("abc");
        let strips = i.render(region(10, 1), &styles());
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].cells[0].ch, 'a');
        assert_eq!(strips[0].cells[1].ch, 'b');
        assert_eq!(strips[0].cells[2].ch, 'c');
        assert_eq!(strips[0].width(), 10);
    }

    #[test]
    fn render_truncates_to_width() {
        let i = Input::new().with_value("Hello World!");
        let strips = i.render(region(5, 1), &styles());
        assert_eq!(strips[0].width(), 5);
        assert_eq!(strips[0].cells[4].ch, 'o');
    }

    #[test]
    fn render_zero_region() {
        let i = Input::new().with_value("abc");
        assert!(i.render(region(0, 1), &styles()).is_empty());
        assert!(i.render(region(10, 0), &styles()).is_empty());
    }

    #[test]
    fn render_empty_no_placeholder() {
        let i = Input::new();
        let strips = i.render(region(5, 1), &styles());
        assert_eq!(strips.len(), 1);
        // All spaces
        for cell in &strips[0].cells {
            assert_eq!(cell.ch, ' ');
        }
    }

    #[test]
    fn as_any_downcast() {
        let i = Input::new().with_value("test");
        let any_ref = i.as_any();
        let downcasted = any_ref.downcast_ref::<Input>().unwrap();
        assert_eq!(downcasted.value(), "test");
    }

    #[test]
    fn default_creates_empty() {
        let i = Input::default();
        assert_eq!(i.value(), "");
        assert_eq!(i.cursor_position(), 0);
    }
}
