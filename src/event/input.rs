//! Input event types wrapping crossterm for decoupling.
//!
//! Defines [`InputEvent`], [`KeyEvent`], [`MouseEvent`] and supporting types.
//! Crossterm events are converted via `From` impls so the rest of the
//! framework never depends on crossterm directly.

use std::ops::{BitAnd, BitOr};

// ---------------------------------------------------------------------------
// Key
// ---------------------------------------------------------------------------

/// Keyboard key, decoupled from crossterm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Enter,
    Escape,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// Modifier key bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(pub u8);

impl Modifiers {
    pub const NONE: Modifiers = Modifiers(0);
    pub const SHIFT: Modifiers = Modifiers(1);
    pub const CTRL: Modifiers = Modifiers(2);
    pub const ALT: Modifiers = Modifiers(4);

    /// Check whether `self` contains all the bits in `other`.
    pub fn contains(self, other: Modifiers) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check whether no modifier bits are set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Modifiers {
    type Output = Modifiers;
    fn bitor(self, rhs: Self) -> Self::Output {
        Modifiers(self.0 | rhs.0)
    }
}

impl BitAnd for Modifiers {
    type Output = Modifiers;
    fn bitand(self, rhs: Self) -> Self::Output {
        Modifiers(self.0 & rhs.0)
    }
}

// ---------------------------------------------------------------------------
// KeyEvent
// ---------------------------------------------------------------------------

/// A keyboard event with key and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: Key,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Create a new key event.
    pub fn new(code: Key, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

// ---------------------------------------------------------------------------
// MouseBtn / MouseAction / MouseEvent
// ---------------------------------------------------------------------------

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBtn {
    Left,
    Right,
    Middle,
}

/// Mouse action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAction {
    Down(MouseBtn),
    Up(MouseBtn),
    Drag(MouseBtn),
    Moved,
    ScrollUp,
    ScrollDown,
}

/// A mouse event with action, position, and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseAction,
    pub x: u16,
    pub y: u16,
    pub modifiers: Modifiers,
}

// ---------------------------------------------------------------------------
// InputEvent
// ---------------------------------------------------------------------------

/// Top-level input event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    FocusGained,
    FocusLost,
    Paste(String),
}

// ---------------------------------------------------------------------------
// From<crossterm> conversions
// ---------------------------------------------------------------------------

/// Convert crossterm key modifiers to our `Modifiers`.
fn convert_modifiers(m: crossterm::event::KeyModifiers) -> Modifiers {
    let mut out = Modifiers::NONE;
    if m.contains(crossterm::event::KeyModifiers::SHIFT) {
        out = out | Modifiers::SHIFT;
    }
    if m.contains(crossterm::event::KeyModifiers::CONTROL) {
        out = out | Modifiers::CTRL;
    }
    if m.contains(crossterm::event::KeyModifiers::ALT) {
        out = out | Modifiers::ALT;
    }
    out
}

/// Convert a crossterm `KeyEvent` into our `KeyEvent`.
///
/// Returns `None` if the key code is not one we handle.
impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(ct: crossterm::event::KeyEvent) -> Self {
        let code = match ct.code {
            crossterm::event::KeyCode::Char(c) => Key::Char(c),
            crossterm::event::KeyCode::Enter => Key::Enter,
            crossterm::event::KeyCode::Esc => Key::Escape,
            crossterm::event::KeyCode::Tab => Key::Tab,
            crossterm::event::KeyCode::BackTab => Key::BackTab,
            crossterm::event::KeyCode::Backspace => Key::Backspace,
            crossterm::event::KeyCode::Delete => Key::Delete,
            crossterm::event::KeyCode::Left => Key::Left,
            crossterm::event::KeyCode::Right => Key::Right,
            crossterm::event::KeyCode::Up => Key::Up,
            crossterm::event::KeyCode::Down => Key::Down,
            crossterm::event::KeyCode::Home => Key::Home,
            crossterm::event::KeyCode::End => Key::End,
            crossterm::event::KeyCode::PageUp => Key::PageUp,
            crossterm::event::KeyCode::PageDown => Key::PageDown,
            crossterm::event::KeyCode::F(n) => Key::F(n),
            // Map unsupported key codes to Escape as a fallback.
            _ => Key::Escape,
        };
        let modifiers = convert_modifiers(ct.modifiers);
        KeyEvent { code, modifiers }
    }
}

/// Convert a crossterm mouse button to our `MouseBtn`.
fn convert_mouse_button(b: crossterm::event::MouseButton) -> MouseBtn {
    match b {
        crossterm::event::MouseButton::Left => MouseBtn::Left,
        crossterm::event::MouseButton::Right => MouseBtn::Right,
        crossterm::event::MouseButton::Middle => MouseBtn::Middle,
    }
}

/// Convert a crossterm `Event` into our `InputEvent`.
///
/// Returns `None` for events we don't handle.
impl From<crossterm::event::Event> for InputEvent {
    fn from(ct: crossterm::event::Event) -> Self {
        match ct {
            crossterm::event::Event::Key(ke) => InputEvent::Key(KeyEvent::from(ke)),
            crossterm::event::Event::Mouse(me) => {
                let modifiers = convert_modifiers(me.modifiers);
                let kind = match me.kind {
                    crossterm::event::MouseEventKind::Down(b) => {
                        MouseAction::Down(convert_mouse_button(b))
                    }
                    crossterm::event::MouseEventKind::Up(b) => {
                        MouseAction::Up(convert_mouse_button(b))
                    }
                    crossterm::event::MouseEventKind::Drag(b) => {
                        MouseAction::Drag(convert_mouse_button(b))
                    }
                    crossterm::event::MouseEventKind::Moved => MouseAction::Moved,
                    crossterm::event::MouseEventKind::ScrollUp => MouseAction::ScrollUp,
                    crossterm::event::MouseEventKind::ScrollDown => MouseAction::ScrollDown,
                    // Map any other scroll variants to ScrollDown.
                    _ => MouseAction::ScrollDown,
                };
                InputEvent::Mouse(MouseEvent {
                    kind,
                    x: me.column,
                    y: me.row,
                    modifiers,
                })
            }
            crossterm::event::Event::Resize(w, h) => InputEvent::Resize {
                width: w,
                height: h,
            },
            crossterm::event::Event::FocusGained => InputEvent::FocusGained,
            crossterm::event::Event::FocusLost => InputEvent::FocusLost,
            crossterm::event::Event::Paste(s) => InputEvent::Paste(s),
        }
    }
}

/// Try to convert a crossterm `Event` into our `InputEvent`.
///
/// This always succeeds since we handle all crossterm event variants.
pub fn try_from_crossterm(event: crossterm::event::Event) -> Option<InputEvent> {
    Some(InputEvent::from(event))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Modifiers ────────────────────────────────────────────────────

    #[test]
    fn modifiers_none_is_empty() {
        assert!(Modifiers::NONE.is_empty());
    }

    #[test]
    fn modifiers_single_flag() {
        assert!(Modifiers::CTRL.contains(Modifiers::CTRL));
        assert!(!Modifiers::CTRL.contains(Modifiers::SHIFT));
        assert!(!Modifiers::CTRL.is_empty());
    }

    #[test]
    fn modifiers_combined() {
        let mods = Modifiers::CTRL | Modifiers::ALT;
        assert!(mods.contains(Modifiers::CTRL));
        assert!(mods.contains(Modifiers::ALT));
        assert!(!mods.contains(Modifiers::SHIFT));
    }

    #[test]
    fn modifiers_bitand() {
        let mods = Modifiers::CTRL | Modifiers::SHIFT;
        let result = mods & Modifiers::CTRL;
        assert_eq!(result, Modifiers::CTRL);
    }

    #[test]
    fn modifiers_contains_none() {
        // Every modifier set contains NONE.
        assert!(Modifiers::CTRL.contains(Modifiers::NONE));
        assert!(Modifiers::NONE.contains(Modifiers::NONE));
    }

    // ── KeyEvent ─────────────────────────────────────────────────────

    #[test]
    fn key_event_new() {
        let ke = KeyEvent::new(Key::Char('a'), Modifiers::NONE);
        assert_eq!(ke.code, Key::Char('a'));
        assert!(ke.modifiers.is_empty());
    }

    #[test]
    fn key_event_with_modifiers() {
        let ke = KeyEvent::new(Key::Char('c'), Modifiers::CTRL);
        assert_eq!(ke.code, Key::Char('c'));
        assert!(ke.modifiers.contains(Modifiers::CTRL));
    }

    // ── From<crossterm::event::KeyEvent> ─────────────────────────────

    #[test]
    fn from_crossterm_key_char() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::Char('x'));
        assert!(ke.modifiers.is_empty());
    }

    #[test]
    fn from_crossterm_key_enter() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::Enter);
    }

    #[test]
    fn from_crossterm_key_f5() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::F(5),
            crossterm::event::KeyModifiers::NONE,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::F(5));
    }

    #[test]
    fn from_crossterm_key_with_ctrl() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::CONTROL,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::Char('c'));
        assert!(ke.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn from_crossterm_key_with_shift_alt() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('A'),
            crossterm::event::KeyModifiers::SHIFT | crossterm::event::KeyModifiers::ALT,
        );
        let ke = KeyEvent::from(ct);
        assert!(ke.modifiers.contains(Modifiers::SHIFT));
        assert!(ke.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn from_crossterm_key_tab() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Tab,
            crossterm::event::KeyModifiers::NONE,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::Tab);
    }

    #[test]
    fn from_crossterm_key_backtab() {
        let ct = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::BackTab,
            crossterm::event::KeyModifiers::SHIFT,
        );
        let ke = KeyEvent::from(ct);
        assert_eq!(ke.code, Key::BackTab);
        assert!(ke.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn from_crossterm_key_arrows() {
        for (ct_code, expected) in [
            (crossterm::event::KeyCode::Left, Key::Left),
            (crossterm::event::KeyCode::Right, Key::Right),
            (crossterm::event::KeyCode::Up, Key::Up),
            (crossterm::event::KeyCode::Down, Key::Down),
        ] {
            let ct = crossterm::event::KeyEvent::new(
                ct_code,
                crossterm::event::KeyModifiers::NONE,
            );
            let ke = KeyEvent::from(ct);
            assert_eq!(ke.code, expected);
        }
    }

    #[test]
    fn from_crossterm_key_navigation() {
        for (ct_code, expected) in [
            (crossterm::event::KeyCode::Home, Key::Home),
            (crossterm::event::KeyCode::End, Key::End),
            (crossterm::event::KeyCode::PageUp, Key::PageUp),
            (crossterm::event::KeyCode::PageDown, Key::PageDown),
            (crossterm::event::KeyCode::Delete, Key::Delete),
            (crossterm::event::KeyCode::Backspace, Key::Backspace),
            (crossterm::event::KeyCode::Esc, Key::Escape),
        ] {
            let ct = crossterm::event::KeyEvent::new(
                ct_code,
                crossterm::event::KeyModifiers::NONE,
            );
            let ke = KeyEvent::from(ct);
            assert_eq!(ke.code, expected);
        }
    }

    // ── From<crossterm::event::Event> ────────────────────────────────

    #[test]
    fn from_crossterm_event_key() {
        let ct = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Key(ke) => assert_eq!(ke.code, Key::Char('q')),
            _ => panic!("expected Key event"),
        }
    }

    #[test]
    fn from_crossterm_event_resize() {
        let ct = crossterm::event::Event::Resize(120, 40);
        let input = InputEvent::from(ct);
        assert_eq!(
            input,
            InputEvent::Resize {
                width: 120,
                height: 40
            }
        );
    }

    #[test]
    fn from_crossterm_event_focus() {
        assert_eq!(
            InputEvent::from(crossterm::event::Event::FocusGained),
            InputEvent::FocusGained
        );
        assert_eq!(
            InputEvent::from(crossterm::event::Event::FocusLost),
            InputEvent::FocusLost
        );
    }

    #[test]
    fn from_crossterm_event_paste() {
        let ct = crossterm::event::Event::Paste("hello".to_string());
        let input = InputEvent::from(ct);
        assert_eq!(input, InputEvent::Paste("hello".to_string()));
    }

    #[test]
    fn try_from_crossterm_returns_some() {
        let ct = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(try_from_crossterm(ct).is_some());
    }

    // ── MouseEvent ───────────────────────────────────────────────────

    #[test]
    fn mouse_event_from_crossterm() {
        let ct = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseAction::Down(MouseBtn::Left));
                assert_eq!(me.x, 10);
                assert_eq!(me.y, 5);
                assert!(me.modifiers.is_empty());
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn mouse_scroll_from_crossterm() {
        let ct = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseAction::ScrollUp);
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn mouse_drag_from_crossterm() {
        let ct = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Right),
            column: 3,
            row: 7,
            modifiers: crossterm::event::KeyModifiers::CONTROL,
        });
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseAction::Drag(MouseBtn::Right));
                assert_eq!(me.x, 3);
                assert_eq!(me.y, 7);
                assert!(me.modifiers.contains(Modifiers::CTRL));
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn mouse_moved_from_crossterm() {
        let ct = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Moved,
            column: 1,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseAction::Moved);
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn mouse_up_middle_from_crossterm() {
        let ct = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Middle),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let input = InputEvent::from(ct);
        match input {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseAction::Up(MouseBtn::Middle));
            }
            _ => panic!("expected Mouse event"),
        }
    }
}
