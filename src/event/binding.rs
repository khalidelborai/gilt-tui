//! Key binding registry and resolution.
//!
//! [`KeyBindingRegistry`] maps key+modifier combinations to [`BindingAction`]s.
//! The `with_defaults()` constructor installs standard bindings (Ctrl+C -> Quit, etc.).

use std::collections::HashMap;

use super::input::{Key, KeyEvent, Modifiers};
use super::message::Message;

// ---------------------------------------------------------------------------
// BindingAction
// ---------------------------------------------------------------------------

/// Action to take when a key binding is matched.
pub enum BindingAction {
    /// Quit the application.
    Quit,
    /// Move focus to the next focusable widget.
    FocusNext,
    /// Move focus to the previous focusable widget.
    FocusPrevious,
    /// A named custom action.
    Custom(String),
    /// Produce a message via a factory function.
    Message(fn() -> Box<dyn Message>),
}

impl std::fmt::Debug for BindingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quit => write!(f, "Quit"),
            Self::FocusNext => write!(f, "FocusNext"),
            Self::FocusPrevious => write!(f, "FocusPrevious"),
            Self::Custom(name) => write!(f, "Custom({name:?})"),
            Self::Message(_) => write!(f, "Message(<fn>)"),
        }
    }
}

// ---------------------------------------------------------------------------
// KeyBinding
// ---------------------------------------------------------------------------

/// A single key binding: key + modifiers -> action.
#[derive(Debug)]
pub struct KeyBinding {
    pub key: Key,
    pub modifiers: Modifiers,
    pub action: BindingAction,
}

// ---------------------------------------------------------------------------
// KeyBindingRegistry
// ---------------------------------------------------------------------------

/// Registry of key bindings, mapping (Key, Modifiers) -> BindingAction.
#[derive(Debug)]
pub struct KeyBindingRegistry {
    bindings: HashMap<(Key, Modifiers), BindingAction>,
}

impl KeyBindingRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Create a registry with standard default bindings.
    ///
    /// Defaults:
    /// - `Ctrl+C` -> Quit
    /// - `Tab` -> FocusNext
    /// - `BackTab` (Shift+Tab) -> FocusPrevious
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.bind(Key::Char('c'), Modifiers::CTRL, BindingAction::Quit);
        registry.bind(Key::Tab, Modifiers::NONE, BindingAction::FocusNext);
        registry.bind(
            Key::BackTab,
            Modifiers::NONE,
            BindingAction::FocusPrevious,
        );
        registry
    }

    /// Register a key binding.
    ///
    /// If a binding already exists for this key+modifier combination, it is replaced.
    pub fn bind(&mut self, key: Key, modifiers: Modifiers, action: BindingAction) {
        self.bindings.insert((key, modifiers), action);
    }

    /// Remove a key binding.
    ///
    /// Returns the removed action, if any.
    pub fn unbind(&mut self, key: Key, modifiers: Modifiers) -> Option<BindingAction> {
        self.bindings.remove(&(key, modifiers))
    }

    /// Look up the action for a given key event.
    ///
    /// First tries exact match of key + modifiers. Returns `None` if no
    /// matching binding is found.
    pub fn resolve(&self, event: &KeyEvent) -> Option<&BindingAction> {
        self.bindings.get(&(event.code, event.modifiers))
    }

    /// Number of registered bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether the registry has no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl Default for KeyBindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn new_registry_is_empty() {
        let reg = KeyBindingRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn default_registry_is_empty() {
        let reg = KeyBindingRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn with_defaults_has_three_bindings() {
        let reg = KeyBindingRegistry::with_defaults();
        assert_eq!(reg.len(), 3);
    }

    // ── Bind / Unbind ────────────────────────────────────────────────

    #[test]
    fn bind_and_resolve() {
        let mut reg = KeyBindingRegistry::new();
        reg.bind(Key::Char('q'), Modifiers::NONE, BindingAction::Quit);

        let event = KeyEvent::new(Key::Char('q'), Modifiers::NONE);
        let action = reg.resolve(&event);
        assert!(action.is_some());
        assert!(matches!(action.unwrap(), BindingAction::Quit));
    }

    #[test]
    fn resolve_no_match() {
        let reg = KeyBindingRegistry::new();
        let event = KeyEvent::new(Key::Char('q'), Modifiers::NONE);
        assert!(reg.resolve(&event).is_none());
    }

    #[test]
    fn resolve_wrong_modifiers() {
        let mut reg = KeyBindingRegistry::new();
        reg.bind(Key::Char('q'), Modifiers::CTRL, BindingAction::Quit);

        // Without Ctrl — no match.
        let event = KeyEvent::new(Key::Char('q'), Modifiers::NONE);
        assert!(reg.resolve(&event).is_none());

        // With Ctrl — match.
        let event2 = KeyEvent::new(Key::Char('q'), Modifiers::CTRL);
        assert!(reg.resolve(&event2).is_some());
    }

    #[test]
    fn unbind_removes_binding() {
        let mut reg = KeyBindingRegistry::new();
        reg.bind(Key::Char('q'), Modifiers::NONE, BindingAction::Quit);
        assert_eq!(reg.len(), 1);

        let removed = reg.unbind(Key::Char('q'), Modifiers::NONE);
        assert!(removed.is_some());
        assert!(reg.is_empty());

        let event = KeyEvent::new(Key::Char('q'), Modifiers::NONE);
        assert!(reg.resolve(&event).is_none());
    }

    #[test]
    fn unbind_nonexistent_returns_none() {
        let mut reg = KeyBindingRegistry::new();
        let removed = reg.unbind(Key::Char('z'), Modifiers::NONE);
        assert!(removed.is_none());
    }

    #[test]
    fn bind_overwrites_existing() {
        let mut reg = KeyBindingRegistry::new();
        reg.bind(
            Key::Char('q'),
            Modifiers::NONE,
            BindingAction::Custom("first".into()),
        );
        reg.bind(
            Key::Char('q'),
            Modifiers::NONE,
            BindingAction::Custom("second".into()),
        );
        assert_eq!(reg.len(), 1);

        let event = KeyEvent::new(Key::Char('q'), Modifiers::NONE);
        let action = reg.resolve(&event).unwrap();
        match action {
            BindingAction::Custom(name) => assert_eq!(name, "second"),
            _ => panic!("expected Custom action"),
        }
    }

    // ── Default bindings ─────────────────────────────────────────────

    #[test]
    fn defaults_ctrl_c_quit() {
        let reg = KeyBindingRegistry::with_defaults();
        let event = KeyEvent::new(Key::Char('c'), Modifiers::CTRL);
        let action = reg.resolve(&event);
        assert!(matches!(action, Some(BindingAction::Quit)));
    }

    #[test]
    fn defaults_tab_focus_next() {
        let reg = KeyBindingRegistry::with_defaults();
        let event = KeyEvent::new(Key::Tab, Modifiers::NONE);
        let action = reg.resolve(&event);
        assert!(matches!(action, Some(BindingAction::FocusNext)));
    }

    #[test]
    fn defaults_backtab_focus_previous() {
        let reg = KeyBindingRegistry::with_defaults();
        let event = KeyEvent::new(Key::BackTab, Modifiers::NONE);
        let action = reg.resolve(&event);
        assert!(matches!(action, Some(BindingAction::FocusPrevious)));
    }

    // ── Message factory action ───────────────────────────────────────

    #[test]
    fn message_factory_action() {
        let mut reg = KeyBindingRegistry::new();
        reg.bind(
            Key::F(1),
            Modifiers::NONE,
            BindingAction::Message(|| Box::new(crate::event::message::Custom::new("help"))),
        );

        let event = KeyEvent::new(Key::F(1), Modifiers::NONE);
        let action = reg.resolve(&event);
        assert!(matches!(action, Some(BindingAction::Message(_))));

        // Invoke the factory.
        if let Some(BindingAction::Message(factory)) = action {
            let msg = factory();
            assert_eq!(msg.message_name(), "Custom");
        }
    }

    // ── Debug ────────────────────────────────────────────────────────

    #[test]
    fn binding_action_debug() {
        assert_eq!(format!("{:?}", BindingAction::Quit), "Quit");
        assert_eq!(format!("{:?}", BindingAction::FocusNext), "FocusNext");
        assert_eq!(
            format!("{:?}", BindingAction::Custom("test".into())),
            "Custom(\"test\")"
        );
    }
}
