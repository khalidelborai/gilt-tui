//! Message trait, envelope, and built-in messages.
//!
//! The [`Message`] trait is object-safe and supports downcasting via `Any`.
//! [`Envelope`] wraps a boxed message with routing metadata (sender, target).
//! Built-in messages: [`Quit`], [`Refresh`], [`FocusNext`], [`FocusPrevious`], [`Custom`].

use std::any::Any;

use crate::dom::node::NodeId;

// ---------------------------------------------------------------------------
// Message trait
// ---------------------------------------------------------------------------

/// Object-safe message trait.
///
/// All messages must implement `as_any` for downcasting and `message_name`
/// for debug/logging purposes.
pub trait Message: Send + 'static {
    /// Upcast to `&dyn Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Human-readable name for this message type.
    fn message_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/// Wraps a boxed message with routing metadata.
pub struct Envelope {
    /// The message payload.
    pub message: Box<dyn Message>,
    /// The node that sent this message.
    pub sender: NodeId,
    /// If `Some`, the message is targeted at a specific node.
    /// If `None`, the message bubbles up from the sender.
    pub target: Option<NodeId>,
    /// Whether this message has been handled (stops propagation).
    pub handled: bool,
}

impl Envelope {
    /// Create a new envelope that will bubble from the sender.
    pub fn new(message: impl Message, sender: NodeId) -> Self {
        Self {
            message: Box::new(message),
            sender,
            target: None,
            handled: false,
        }
    }

    /// Create a new envelope targeted at a specific node.
    pub fn targeted(message: impl Message, sender: NodeId, target: NodeId) -> Self {
        Self {
            message: Box::new(message),
            sender,
            target: Some(target),
            handled: false,
        }
    }

    /// Attempt to downcast the message to a concrete type.
    pub fn downcast_ref<T: Message + 'static>(&self) -> Option<&T> {
        self.message.as_any().downcast_ref::<T>()
    }

    /// Mark this envelope as handled, stopping further propagation.
    pub fn mark_handled(&mut self) {
        self.handled = true;
    }
}

impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Envelope")
            .field("message_name", &self.message.message_name())
            .field("sender", &self.sender)
            .field("target", &self.target)
            .field("handled", &self.handled)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Built-in messages
// ---------------------------------------------------------------------------

/// Request application shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quit;

impl Message for Quit {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn message_name(&self) -> &str {
        "Quit"
    }
}

/// Request a full re-render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refresh;

impl Message for Refresh {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn message_name(&self) -> &str {
        "Refresh"
    }
}

/// Move focus to the next focusable widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusNext;

impl Message for FocusNext {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn message_name(&self) -> &str {
        "FocusNext"
    }
}

/// Move focus to the previous focusable widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusPrevious;

impl Message for FocusPrevious {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn message_name(&self) -> &str {
        "FocusPrevious"
    }
}

/// User-defined string message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Custom(pub String);

impl Custom {
    /// Create a new custom message.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl Message for Custom {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn message_name(&self) -> &str {
        "Custom"
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn make_id(sm: &mut SlotMap<NodeId, ()>) -> NodeId {
        sm.insert(())
    }

    // ── Message trait ────────────────────────────────────────────────

    #[test]
    fn quit_message_name() {
        let q = Quit;
        assert_eq!(q.message_name(), "Quit");
    }

    #[test]
    fn refresh_message_name() {
        let r = Refresh;
        assert_eq!(r.message_name(), "Refresh");
    }

    #[test]
    fn focus_next_message_name() {
        let f = FocusNext;
        assert_eq!(f.message_name(), "FocusNext");
    }

    #[test]
    fn focus_previous_message_name() {
        let f = FocusPrevious;
        assert_eq!(f.message_name(), "FocusPrevious");
    }

    #[test]
    fn custom_message_name() {
        let c = Custom::new("my_event");
        assert_eq!(c.message_name(), "Custom");
        assert_eq!(c.0, "my_event");
    }

    // ── Envelope ─────────────────────────────────────────────────────

    #[test]
    fn envelope_new_bubbling() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Quit, sender);
        assert_eq!(env.sender, sender);
        assert!(env.target.is_none());
        assert!(!env.handled);
    }

    #[test]
    fn envelope_targeted() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let target = make_id(&mut sm);
        let env = Envelope::targeted(Refresh, sender, target);
        assert_eq!(env.sender, sender);
        assert_eq!(env.target, Some(target));
        assert!(!env.handled);
    }

    #[test]
    fn envelope_downcast_ref_success() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Custom::new("test"), sender);
        let custom = env.downcast_ref::<Custom>();
        assert!(custom.is_some());
        assert_eq!(custom.unwrap().0, "test");
    }

    #[test]
    fn envelope_downcast_ref_wrong_type() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Quit, sender);
        let custom = env.downcast_ref::<Custom>();
        assert!(custom.is_none());
    }

    #[test]
    fn envelope_mark_handled() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let mut env = Envelope::new(Quit, sender);
        assert!(!env.handled);
        env.mark_handled();
        assert!(env.handled);
    }

    #[test]
    fn envelope_debug_format() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Quit, sender);
        let dbg = format!("{:?}", env);
        assert!(dbg.contains("Quit"));
        assert!(dbg.contains("Envelope"));
    }

    #[test]
    fn envelope_downcast_quit() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Quit, sender);
        assert!(env.downcast_ref::<Quit>().is_some());
    }

    #[test]
    fn envelope_downcast_focus_next() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(FocusNext, sender);
        assert!(env.downcast_ref::<FocusNext>().is_some());
        assert!(env.downcast_ref::<FocusPrevious>().is_none());
    }

    #[test]
    fn envelope_downcast_focus_previous() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(FocusPrevious, sender);
        assert!(env.downcast_ref::<FocusPrevious>().is_some());
        assert!(env.downcast_ref::<FocusNext>().is_none());
    }

    #[test]
    fn envelope_downcast_refresh() {
        let mut sm = SlotMap::with_key();
        let sender = make_id(&mut sm);
        let env = Envelope::new(Refresh, sender);
        assert!(env.downcast_ref::<Refresh>().is_some());
        assert!(env.downcast_ref::<Quit>().is_none());
    }
}
