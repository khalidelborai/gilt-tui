//! App struct: lifecycle, event loop, screen management.
//!
//! [`App`] ties together the screen, event dispatcher, key bindings, and driver.
//! The `new_headless` constructor allows testing without a real terminal.

use std::io;

use crate::event::binding::{BindingAction, KeyBindingRegistry};
use crate::event::handler::EventDispatcher;
use crate::event::input::InputEvent;
use crate::event::message::{self, Envelope};
use crate::render::driver::Driver;
use crate::screen::Screen;

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

/// Configuration for the application.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Optional window/app title.
    pub title: Option<String>,
    /// Optional CSS string to compile and apply.
    pub css: Option<String>,
    /// Target frames per second for the render loop.
    pub fps: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: None,
            css: None,
            fps: 60,
        }
    }
}

impl AppConfig {
    /// Create a new default config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title (builder).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the CSS string (builder).
    pub fn with_css(mut self, css: impl Into<String>) -> Self {
        self.css = Some(css.into());
        self
    }

    /// Set the target FPS (builder).
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// The main application struct.
///
/// Owns the screen, driver, key bindings, event dispatcher, and config.
/// The driver is optional to support headless testing.
pub struct App {
    /// The screen (DOM, styles, layout, compositor, focus).
    pub screen: Screen,
    /// Terminal output driver. `None` in headless mode.
    pub driver: Option<Driver>,
    /// Key binding registry.
    pub bindings: KeyBindingRegistry,
    /// Event dispatcher (message queue).
    pub dispatcher: EventDispatcher,
    /// Application configuration.
    pub config: AppConfig,
    /// Whether the app is still running.
    running: bool,
}

impl App {
    /// Create a new app with a real terminal driver.
    ///
    /// Queries the terminal size to set the initial screen dimensions.
    pub fn new(config: AppConfig) -> io::Result<Self> {
        let (width, height) = Driver::terminal_size()?;
        let driver = Driver::new()?;
        Ok(Self {
            screen: Screen::new(width, height),
            driver: Some(driver),
            bindings: KeyBindingRegistry::with_defaults(),
            dispatcher: EventDispatcher::new(),
            config,
            running: true,
        })
    }

    /// Create a headless app for testing (no terminal driver).
    ///
    /// Uses the given dimensions for the screen size.
    pub fn new_headless(width: u16, height: u16) -> Self {
        Self {
            screen: Screen::new(width, height),
            driver: None,
            bindings: KeyBindingRegistry::with_defaults(),
            dispatcher: EventDispatcher::new(),
            config: AppConfig::default(),
            running: true,
        }
    }

    /// Handle an input event by resolving key bindings and pushing messages.
    ///
    /// For key events, looks up the binding and converts it to a message.
    /// For resize events, updates the screen dimensions.
    /// Other events are currently ignored.
    pub fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key(ke) => {
                if let Some(action) = self.bindings.resolve(&ke) {
                    // We need to create a sender NodeId. Use root if available,
                    // or skip if the DOM is empty.
                    let sender = match self.screen.dom.root() {
                        Some(root) => root,
                        None => return,
                    };

                    match action {
                        BindingAction::Quit => {
                            self.dispatcher
                                .push(Envelope::new(message::Quit, sender));
                        }
                        BindingAction::FocusNext => {
                            self.dispatcher
                                .push(Envelope::new(message::FocusNext, sender));
                        }
                        BindingAction::FocusPrevious => {
                            self.dispatcher
                                .push(Envelope::new(message::FocusPrevious, sender));
                        }
                        BindingAction::Custom(name) => {
                            self.dispatcher
                                .push(Envelope::new(message::Custom::new(name.clone()), sender));
                        }
                        BindingAction::Message(factory) => {
                            self.dispatcher.push(Envelope {
                                message: factory(),
                                sender,
                                target: None,
                                handled: false,
                            });
                        }
                    }
                }
            }
            InputEvent::Resize { width, height } => {
                self.screen.resize(width, height);
            }
            // Mouse, focus, paste events are currently unhandled at the app level.
            _ => {}
        }
    }

    /// Process all pending messages in the dispatcher.
    ///
    /// Built-in messages (Quit, FocusNext, FocusPrevious) are handled directly.
    /// Other messages are currently ignored (widgets will handle them in future phases).
    pub fn handle_messages(&mut self) {
        let messages = self.dispatcher.drain();
        for envelope in messages {
            if envelope.downcast_ref::<message::Quit>().is_some() {
                self.running = false;
            } else if envelope.downcast_ref::<message::FocusNext>().is_some() {
                self.screen.focus.focus_next();
            } else if envelope.downcast_ref::<message::FocusPrevious>().is_some() {
                self.screen.focus.focus_previous();
            }
            // Refresh and Custom messages are noted but not yet actionable
            // at this phase. They will be handled when widgets can process them.
        }
    }

    /// Whether the app should quit.
    pub fn should_quit(&self) -> bool {
        !self.running
    }

    /// Request the app to quit.
    pub fn request_quit(&mut self) {
        self.running = false;
    }

    /// Whether the app has a terminal driver (not headless).
    pub fn has_driver(&self) -> bool {
        self.driver.is_some()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::node::NodeData;
    use crate::event::input::{Key, KeyEvent, Modifiers};
    use crate::event::message::{FocusNext, FocusPrevious, Quit, Refresh};

    fn headless_app() -> App {
        App::new_headless(80, 24)
    }

    fn headless_app_with_dom() -> App {
        let mut app = App::new_headless(80, 24);
        let root = app
            .screen
            .dom
            .insert(NodeData::new("Root").focusable(false));
        let _a = app
            .screen
            .dom
            .insert_child(root, NodeData::new("A").focusable(true));
        let _b = app
            .screen
            .dom
            .insert_child(root, NodeData::new("B").focusable(true));
        let _c = app
            .screen
            .dom
            .insert_child(root, NodeData::new("C").focusable(true));
        app.screen.focus.rebuild(&app.screen.dom);
        app
    }

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn headless_app_no_driver() {
        let app = headless_app();
        assert!(!app.has_driver());
        assert!(!app.should_quit());
    }

    #[test]
    fn headless_app_screen_size() {
        let app = App::new_headless(120, 40);
        assert_eq!(app.screen.compositor.width, 120);
        assert_eq!(app.screen.compositor.height, 40);
    }

    #[test]
    fn headless_app_has_default_bindings() {
        let app = headless_app();
        assert_eq!(app.bindings.len(), 3);
    }

    // ── request_quit / should_quit ───────────────────────────────────

    #[test]
    fn request_quit() {
        let mut app = headless_app();
        assert!(!app.should_quit());
        app.request_quit();
        assert!(app.should_quit());
    }

    // ── handle_input: key events ─────────────────────────────────────

    #[test]
    fn handle_input_ctrl_c_produces_quit_message() {
        let mut app = headless_app_with_dom();
        let event = InputEvent::Key(KeyEvent::new(Key::Char('c'), Modifiers::CTRL));
        app.handle_input(event);

        assert_eq!(app.dispatcher.pending_count(), 1);
        let messages = app.dispatcher.drain();
        assert!(messages[0].downcast_ref::<Quit>().is_some());
    }

    #[test]
    fn handle_input_tab_produces_focus_next_message() {
        let mut app = headless_app_with_dom();
        let event = InputEvent::Key(KeyEvent::new(Key::Tab, Modifiers::NONE));
        app.handle_input(event);

        let messages = app.dispatcher.drain();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].downcast_ref::<FocusNext>().is_some());
    }

    #[test]
    fn handle_input_backtab_produces_focus_previous_message() {
        let mut app = headless_app_with_dom();
        let event = InputEvent::Key(KeyEvent::new(Key::BackTab, Modifiers::NONE));
        app.handle_input(event);

        let messages = app.dispatcher.drain();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].downcast_ref::<FocusPrevious>().is_some());
    }

    #[test]
    fn handle_input_unbound_key_no_message() {
        let mut app = headless_app_with_dom();
        let event = InputEvent::Key(KeyEvent::new(Key::Char('z'), Modifiers::NONE));
        app.handle_input(event);

        assert!(app.dispatcher.is_empty());
    }

    #[test]
    fn handle_input_resize_updates_screen() {
        let mut app = headless_app();
        let event = InputEvent::Resize {
            width: 120,
            height: 40,
        };
        app.handle_input(event);

        assert_eq!(app.screen.compositor.width, 120);
        assert_eq!(app.screen.compositor.height, 40);
    }

    // ── handle_messages ──────────────────────────────────────────────

    #[test]
    fn handle_messages_quit() {
        let mut app = headless_app_with_dom();
        let root = app.screen.dom.root().unwrap();
        app.dispatcher.push(Envelope::new(Quit, root));
        app.handle_messages();
        assert!(app.should_quit());
    }

    #[test]
    fn handle_messages_focus_next() {
        let mut app = headless_app_with_dom();
        let root = app.screen.dom.root().unwrap();
        app.dispatcher.push(Envelope::new(FocusNext, root));
        app.handle_messages();

        // Focus should have moved to the first focusable node.
        assert!(app.screen.focused_node().is_some());
    }

    #[test]
    fn handle_messages_focus_previous() {
        let mut app = headless_app_with_dom();
        let root = app.screen.dom.root().unwrap();
        app.dispatcher.push(Envelope::new(FocusPrevious, root));
        app.handle_messages();

        // Focus should have moved to the last focusable node.
        assert!(app.screen.focused_node().is_some());
    }

    #[test]
    fn handle_messages_multiple() {
        let mut app = headless_app_with_dom();
        let root = app.screen.dom.root().unwrap();
        app.dispatcher.push(Envelope::new(FocusNext, root));
        app.dispatcher.push(Envelope::new(FocusNext, root));
        app.dispatcher.push(Envelope::new(Quit, root));
        app.handle_messages();

        // Focus moved twice, then quit.
        assert!(app.should_quit());
    }

    #[test]
    fn handle_messages_drains_queue() {
        let mut app = headless_app_with_dom();
        let root = app.screen.dom.root().unwrap();
        app.dispatcher.push(Envelope::new(Refresh, root));
        app.handle_messages();
        assert!(app.dispatcher.is_empty());
    }

    // ── handle_input without DOM root ────────────────────────────────

    #[test]
    fn handle_input_no_dom_root_no_panic() {
        let mut app = headless_app();
        // No DOM root — should not panic.
        let event = InputEvent::Key(KeyEvent::new(Key::Char('c'), Modifiers::CTRL));
        app.handle_input(event);
        assert!(app.dispatcher.is_empty());
    }

    // ── AppConfig builder ────────────────────────────────────────────

    #[test]
    fn app_config_defaults() {
        let config = AppConfig::new();
        assert!(config.title.is_none());
        assert!(config.css.is_none());
        assert_eq!(config.fps, 60);
    }

    #[test]
    fn app_config_builder() {
        let config = AppConfig::new()
            .with_title("My App")
            .with_css("Button { color: red; }")
            .with_fps(30);
        assert_eq!(config.title, Some("My App".into()));
        assert_eq!(config.css, Some("Button { color: red; }".into()));
        assert_eq!(config.fps, 30);
    }
}
