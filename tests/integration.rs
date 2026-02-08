//! Integration tests for gilt-tui.
//!
//! These tests exercise the public API from outside the crate, verifying that
//! the testing framework, widgets, and app work together correctly.

use gilt_tui::app::AppConfig;
use gilt_tui::event::input::{Key, Modifiers};
use gilt_tui::testing::pilot::Pilot;
use gilt_tui::testing::render_to_string;
use gilt_tui::testing::snapshot::{compositor_to_string, render_to_styled_string, strips_to_string};
use gilt_tui::widget::Widget;
use gilt_tui::widgets::*;

// ---------------------------------------------------------------------------
// render_to_string with widgets
// ---------------------------------------------------------------------------

#[test]
fn test_static_widget_renders_text() {
    let widget = Static::new("Hello, World!");
    let output = render_to_string(&widget, 20, 1);
    assert!(output.contains("Hello, World!"));
}

#[test]
fn test_button_renders_label() {
    let widget = Button::new("OK");
    let output = render_to_string(&widget, 10, 3);
    assert!(output.contains("OK"));
}

#[test]
fn test_header_renders_title() {
    let header = Header::new("Title");
    let output = render_to_string(&header, 20, 1);
    assert!(output.contains("Title"));
}

#[test]
fn test_footer_renders_content() {
    let footer = Footer::new("Status");
    let output = render_to_string(&footer, 20, 1);
    assert!(output.contains("Status"));
}

#[test]
fn test_header_footer_rendering() {
    let header = Header::new("Title");
    let footer = Footer::new("Status");

    let h_out = render_to_string(&header, 20, 1);
    let f_out = render_to_string(&footer, 20, 1);

    assert!(h_out.contains("Title"));
    assert!(f_out.contains("Status"));
}

// ---------------------------------------------------------------------------
// Input widget
// ---------------------------------------------------------------------------

#[test]
fn test_input_typing() {
    let mut input = Input::new();
    input.insert_char('H');
    input.insert_char('i');
    assert_eq!(input.value(), "Hi");
}

#[test]
fn test_input_delete() {
    let mut input = Input::new();
    input.insert_char('A');
    input.insert_char('B');
    input.insert_char('C');
    input.delete_char(); // remove 'C'
    assert_eq!(input.value(), "AB");
}

#[test]
fn test_input_cursor_movement() {
    let mut input = Input::new().with_value("abcde");
    assert_eq!(input.cursor_position(), 5);
    input.move_cursor_home();
    assert_eq!(input.cursor_position(), 0);
    input.move_cursor_end();
    assert_eq!(input.cursor_position(), 5);
    input.move_cursor_left();
    assert_eq!(input.cursor_position(), 4);
    input.move_cursor_right();
    assert_eq!(input.cursor_position(), 5);
}

#[test]
fn test_input_renders() {
    let input = Input::new().with_value("hello");
    let output = render_to_string(&input, 20, 1);
    assert!(output.contains("hello"));
}

// ---------------------------------------------------------------------------
// Pilot key press
// ---------------------------------------------------------------------------

#[test]
fn test_pilot_key_press() {
    let mut pilot = Pilot::new(80, 24);
    pilot.press_key(Key::Char('a'));
    pilot.process();
    assert!(pilot.is_running());
}

#[test]
fn test_pilot_quit() {
    let mut pilot = pilot_with_dom();
    pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
    pilot.process();
    assert!(!pilot.is_running());
}

#[test]
fn test_pilot_resize() {
    let mut pilot = Pilot::new(80, 24);
    pilot.resize(120, 40);
    pilot.process();
    assert!(pilot.is_running());
    assert_eq!(pilot.app().screen.compositor.width, 120);
    assert_eq!(pilot.app().screen.compositor.height, 40);
}

#[test]
fn test_pilot_click() {
    let mut pilot = Pilot::new(80, 24);
    pilot.click(10, 5);
    pilot.process();
    assert!(pilot.is_running());
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

#[test]
fn test_container_default_css() {
    let container = Container::new();
    assert!(!container.default_css().is_empty());
}

#[test]
fn test_container_with_children() {
    let container = Container::new()
        .with_child(Static::new("child1"))
        .with_child(Button::new("child2"));
    assert_eq!(container.child_count(), 2);
}

// ---------------------------------------------------------------------------
// Pilot with config
// ---------------------------------------------------------------------------

#[test]
fn test_pilot_with_config() {
    let config = AppConfig::new()
        .with_title("Test")
        .with_css("Button { color: blue; }")
        .with_fps(30);
    let pilot = Pilot::with_config(config);
    assert_eq!(pilot.app().config.title, Some("Test".to_owned()));
    assert!(!pilot.app().has_driver());
}

#[test]
fn test_pilot_with_css() {
    let pilot = Pilot::new(80, 24).with_css("Header { background: red; }");
    assert_eq!(
        pilot.app().config.css,
        Some("Header { background: red; }".to_owned())
    );
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

#[test]
fn test_strips_to_string() {
    use gilt_tui::render::strip::{CellStyle, Strip};

    let mut strip = Strip::new(0, 0);
    strip.push_str("Test", CellStyle::default());
    let output = strips_to_string(&[strip], 10, 1);
    assert!(output.starts_with("Test"));
}

#[test]
fn test_compositor_to_string() {
    use gilt_tui::geometry::Region;
    use gilt_tui::render::compositor::Compositor;
    use gilt_tui::render::strip::{CellStyle, Strip};

    let mut compositor = Compositor::new(10, 2);
    let mut strip = Strip::new(0, 0);
    strip.push_str("OK", CellStyle::default());
    compositor.place_strips(&[strip], &Region::new(0, 0, 10, 2));
    let output = compositor_to_string(&compositor);
    assert!(output.starts_with("OK"));
}

#[test]
fn test_render_to_styled_string() {
    use gilt_tui::css::styles::Styles;

    let widget = Static::new("Styled");
    let styles = Styles::new();
    let output = render_to_styled_string(&widget, 20, 1, &styles);
    assert!(output.contains("Styled"));
}

// ---------------------------------------------------------------------------
// Pilot render helpers
// ---------------------------------------------------------------------------

#[test]
fn test_pilot_render_widget() {
    let pilot = Pilot::new(80, 24);
    let widget = Static::new("pilot-render");
    let strips = pilot.render_widget(&widget, 20, 1);
    assert_eq!(strips.len(), 1);
    let text: String = strips[0].cells.iter().map(|c| c.ch).collect();
    assert!(text.contains("pilot-render"));
}

#[test]
fn test_pilot_render_to_text() {
    let pilot = Pilot::new(80, 24);
    let widget = Footer::new("bottom text");
    let text = pilot.render_to_text(&widget, 30, 1);
    assert!(text.contains("bottom text"));
}

// ---------------------------------------------------------------------------
// Full flow
// ---------------------------------------------------------------------------

#[test]
fn test_full_lifecycle() {
    let mut pilot = pilot_with_dom();

    // App starts running
    assert!(pilot.is_running());

    // Type some text, resize, process
    pilot.type_text("hello");
    pilot.resize(100, 50);
    pilot.process();
    assert!(pilot.is_running());

    // Verify resize took effect
    assert_eq!(pilot.app().screen.compositor.width, 100);
    assert_eq!(pilot.app().screen.compositor.height, 50);

    // Quit
    pilot.press_key_with(Key::Char('c'), Modifiers::CTRL);
    pilot.process();
    assert!(!pilot.is_running());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a pilot with a DOM root so key bindings can produce messages.
fn pilot_with_dom() -> Pilot {
    use gilt_tui::dom::node::NodeData;

    let mut pilot = Pilot::new(80, 24);
    let app = pilot.app_mut();
    let root = app
        .screen
        .dom
        .insert(NodeData::new("Root").focusable(false));
    let _a = app
        .screen
        .dom
        .insert_child(root, NodeData::new("A").focusable(true));
    app.screen.focus.rebuild(&app.screen.dom);
    pilot
}
