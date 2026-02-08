//! Pilot: programmatic interaction with a headless App.
//!
//! The `Pilot` wraps an [`App`](crate::app::App) in headless mode and provides
//! methods to simulate user input (key presses, mouse clicks, resize), process
//! messages, and render widgets to text for snapshot testing.

use crate::app::{App, AppConfig};
use crate::css::styles::Styles;
use crate::event::input::{
    InputEvent, Key, KeyEvent, Modifiers, MouseAction, MouseBtn, MouseEvent,
};
use crate::geometry::Region;
use crate::render::strip::Strip;
use crate::widget::Widget;

// ---------------------------------------------------------------------------
// Pilot
// ---------------------------------------------------------------------------

/// A headless app driver for testing.
///
/// The Pilot creates an [`App`] without a terminal driver, then provides a
/// high-level API for simulating user interaction and inspecting rendered output.
///
/// # Examples
///
/// ```ignore
/// use gilt_tui::testing::Pilot;
/// use gilt_tui::event::Key;
///
/// let mut pilot = Pilot::new(80, 24);
/// pilot.press_key(Key::Char('a'));
/// pilot.process();
/// assert!(pilot.is_running());
/// ```
pub struct Pilot {
    app: App,
}

impl Pilot {
    /// Create a headless app with the given terminal size.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            app: App::new_headless(width, height),
        }
    }

    /// Apply a CSS stylesheet string to the pilot's app config.
    ///
    /// Note: this sets the config's css field. Actual CSS compilation/application
    /// depends on the app's stylesheet pipeline.
    pub fn with_css(mut self, css: &str) -> Self {
        self.app.config.css = Some(css.to_owned());
        self
    }

    /// Create a Pilot from an [`AppConfig`], forcing headless mode.
    ///
    /// The config's title/css/fps are preserved but no terminal driver is created.
    pub fn with_config(config: AppConfig) -> Self {
        let mut app = App::new_headless(80, 24);
        app.config = config;
        Self { app }
    }

    // ── Input simulation ─────────────────────────────────────────────

    /// Simulate a key press with no modifiers.
    pub fn press_key(&mut self, key: Key) {
        let event = InputEvent::Key(KeyEvent::new(key, Modifiers::NONE));
        self.app.handle_input(event);
    }

    /// Simulate a key press with the given modifiers.
    pub fn press_key_with(&mut self, key: Key, modifiers: Modifiers) {
        let event = InputEvent::Key(KeyEvent::new(key, modifiers));
        self.app.handle_input(event);
    }

    /// Simulate typing each character of `text` as individual key presses.
    ///
    /// Each character is sent as a `Key::Char(ch)` with no modifiers.
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.press_key(Key::Char(ch));
        }
    }

    /// Simulate a left-button mouse click at (x, y).
    pub fn click(&mut self, x: u16, y: u16) {
        let event = InputEvent::Mouse(MouseEvent {
            kind: MouseAction::Down(MouseBtn::Left),
            x,
            y,
            modifiers: Modifiers::NONE,
        });
        self.app.handle_input(event);
    }

    /// Simulate a terminal resize to the given dimensions.
    pub fn resize(&mut self, width: u16, height: u16) {
        let event = InputEvent::Resize { width, height };
        self.app.handle_input(event);
    }

    // ── Processing ───────────────────────────────────────────────────

    /// Process all pending messages in the app's dispatcher.
    pub fn process(&mut self) {
        self.app.handle_messages();
    }

    /// Simulate one frame: process all pending messages.
    ///
    /// This is an alias for [`process`](Self::process) — in a real app, a "tick"
    /// would also include rendering, but for headless testing we only process
    /// the message queue.
    pub fn tick(&mut self) {
        self.process();
    }

    // ── Query ────────────────────────────────────────────────────────

    /// Borrow the underlying app immutably.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Borrow the underlying app mutably.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// Whether the app is still running (has not quit).
    pub fn is_running(&self) -> bool {
        !self.app.should_quit()
    }

    // ── Render helpers ───────────────────────────────────────────────

    /// Render a widget into strips within a region of the given dimensions.
    ///
    /// Uses default (empty) styles.
    pub fn render_widget(&self, widget: &dyn Widget, width: i32, height: i32) -> Vec<Strip> {
        let region = Region::new(0, 0, width, height);
        let styles = Styles::new();
        widget.render(region, &styles)
    }

    /// Render a widget to a plain text string.
    ///
    /// Each row of the rendered output becomes one line in the string, with
    /// trailing spaces trimmed. Lines are separated by `'\n'`.
    pub fn render_to_text(&self, widget: &dyn Widget, width: i32, height: i32) -> String {
        let strips = self.render_widget(widget, width, height);
        super::snapshot::strips_to_string(&strips, width, height)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::node::NodeData;
    use crate::widgets::{Button, Footer, Header, Input, Static};

    /// Helper: create a pilot with a DOM so key bindings can produce messages.
    fn pilot_with_dom() -> Pilot {
        let mut pilot = Pilot::new(80, 24);
        let root = pilot
            .app
            .screen
            .dom
            .insert(NodeData::new("Root").focusable(false));
        let _a = pilot
            .app
            .screen
            .dom
            .insert_child(root, NodeData::new("A").focusable(true));
        pilot.app.screen.focus.rebuild(&pilot.app.screen.dom);
        pilot
    }

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn new_creates_headless_app() {
        let pilot = Pilot::new(80, 24);
        assert!(!pilot.app().has_driver());
        assert!(pilot.is_running());
    }

    #[test]
    fn new_sets_screen_dimensions() {
        let pilot = Pilot::new(120, 40);
        assert_eq!(pilot.app().screen.compositor.width, 120);
        assert_eq!(pilot.app().screen.compositor.height, 40);
    }

    #[test]
    fn with_css_sets_config() {
        let pilot = Pilot::new(80, 24).with_css("Button { color: red; }");
        assert_eq!(
            pilot.app().config.css,
            Some("Button { color: red; }".to_owned())
        );
    }

    #[test]
    fn with_config_preserves_settings() {
        let config = AppConfig::new()
            .with_title("Test App")
            .with_css("Container { background: blue; }")
            .with_fps(30);
        let pilot = Pilot::with_config(config);
        assert_eq!(pilot.app().config.title, Some("Test App".to_owned()));
        assert_eq!(
            pilot.app().config.css,
            Some("Container { background: blue; }".to_owned())
        );
        assert_eq!(pilot.app().config.fps, 30);
        assert!(!pilot.app().has_driver());
    }

    // ── Key input ────────────────────────────────────────────────────

    #[test]
    fn press_key_dispatches_event() {
        let mut pilot = pilot_with_dom();
        // 'a' is unbound, so no message is produced
        pilot.press_key(Key::Char('a'));
        assert!(pilot.app().dispatcher.is_empty());
    }

    #[test]
    fn press_key_with_ctrl_c_queues_quit() {
        let mut pilot = pilot_with_dom();
        pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
        assert_eq!(pilot.app().dispatcher.pending_count(), 1);
    }

    #[test]
    fn press_key_tab_queues_focus_next() {
        let mut pilot = pilot_with_dom();
        pilot.press_key(Key::Tab);
        assert_eq!(pilot.app().dispatcher.pending_count(), 1);
    }

    // ── Type text ────────────────────────────────────────────────────

    #[test]
    fn type_text_sends_char_events() {
        let mut pilot = pilot_with_dom();
        // No characters are bound, so the dispatcher stays empty
        pilot.type_text("hello");
        assert!(pilot.app().dispatcher.is_empty());
    }

    #[test]
    fn type_text_empty_string() {
        let mut pilot = Pilot::new(80, 24);
        pilot.type_text("");
        // No events dispatched
        assert!(pilot.app().dispatcher.is_empty());
    }

    // ── Click ────────────────────────────────────────────────────────

    #[test]
    fn click_sends_mouse_event() {
        let mut pilot = Pilot::new(80, 24);
        // Mouse events are currently unhandled at app level, so no panic
        pilot.click(10, 5);
        // The app doesn't queue messages for mouse events currently
        assert!(pilot.app().dispatcher.is_empty());
    }

    #[test]
    fn click_at_origin() {
        let mut pilot = Pilot::new(80, 24);
        pilot.click(0, 0);
        assert!(pilot.is_running());
    }

    // ── Resize ───────────────────────────────────────────────────────

    #[test]
    fn resize_updates_compositor_dimensions() {
        let mut pilot = Pilot::new(80, 24);
        pilot.resize(120, 40);
        assert_eq!(pilot.app().screen.compositor.width, 120);
        assert_eq!(pilot.app().screen.compositor.height, 40);
    }

    #[test]
    fn resize_does_not_quit() {
        let mut pilot = Pilot::new(80, 24);
        pilot.resize(100, 50);
        pilot.process();
        assert!(pilot.is_running());
    }

    // ── Process / Tick ───────────────────────────────────────────────

    #[test]
    fn process_handles_quit() {
        let mut pilot = pilot_with_dom();
        pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
        pilot.process();
        assert!(!pilot.is_running());
    }

    #[test]
    fn tick_is_alias_for_process() {
        let mut pilot = pilot_with_dom();
        pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
        pilot.tick();
        assert!(!pilot.is_running());
    }

    #[test]
    fn process_empty_queue_is_noop() {
        let mut pilot = Pilot::new(80, 24);
        pilot.process();
        assert!(pilot.is_running());
    }

    // ── App access ───────────────────────────────────────────────────

    #[test]
    fn app_mut_allows_mutation() {
        let mut pilot = Pilot::new(80, 24);
        pilot.app_mut().request_quit();
        assert!(!pilot.is_running());
    }

    // ── Render widget ────────────────────────────────────────────────

    #[test]
    fn render_widget_static() {
        let pilot = Pilot::new(80, 24);
        let widget = Static::new("Hello");
        let strips = pilot.render_widget(&widget, 20, 1);
        assert_eq!(strips.len(), 1);
        assert_eq!(strips[0].cells[0].ch, 'H');
    }

    #[test]
    fn render_widget_button() {
        let pilot = Pilot::new(80, 24);
        let widget = Button::new("OK");
        let strips = pilot.render_widget(&widget, 10, 3);
        assert_eq!(strips.len(), 3);
        // Label on row 1, centered
        let row1_text: String = strips[1].cells.iter().map(|c| c.ch).collect();
        assert!(row1_text.contains("OK"));
    }

    #[test]
    fn render_to_text_static() {
        let pilot = Pilot::new(80, 24);
        let widget = Static::new("World");
        let text = pilot.render_to_text(&widget, 10, 1);
        assert!(text.contains("World"));
    }

    #[test]
    fn render_to_text_multiline() {
        let pilot = Pilot::new(80, 24);
        let widget = Static::new("Line1\nLine2");
        let text = pilot.render_to_text(&widget, 10, 2);
        assert!(text.contains("Line1"));
        assert!(text.contains("Line2"));
    }

    #[test]
    fn render_to_text_header() {
        let pilot = Pilot::new(80, 24);
        let widget = Header::new("Title");
        let text = pilot.render_to_text(&widget, 20, 1);
        assert!(text.contains("Title"));
    }

    #[test]
    fn render_to_text_footer() {
        let pilot = Pilot::new(80, 24);
        let widget = Footer::new("Status");
        let text = pilot.render_to_text(&widget, 20, 1);
        assert!(text.contains("Status"));
    }

    #[test]
    fn render_widget_empty_region() {
        let pilot = Pilot::new(80, 24);
        let widget = Static::new("Hello");
        let strips = pilot.render_widget(&widget, 0, 0);
        assert!(strips.is_empty());
    }

    // ── Full flow ────────────────────────────────────────────────────

    #[test]
    fn full_flow_type_resize_quit() {
        let mut pilot = pilot_with_dom();
        pilot.type_text("abc");
        pilot.resize(100, 50);
        pilot.process();
        assert!(pilot.is_running());
        assert_eq!(pilot.app().screen.compositor.width, 100);

        pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
        pilot.process();
        assert!(!pilot.is_running());
    }

    #[test]
    fn input_widget_independent_of_pilot() {
        // Verify Input widget works independently of the pilot
        let mut input = Input::new();
        input.insert_char('H');
        input.insert_char('i');
        assert_eq!(input.value(), "Hi");
    }
}
