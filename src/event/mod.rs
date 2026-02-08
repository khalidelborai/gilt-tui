//! Event system: messages, input, key bindings, dispatch.

pub mod binding;
pub mod handler;
pub mod input;
pub mod message;

pub use binding::{BindingAction, KeyBindingRegistry};
pub use handler::EventDispatcher;
pub use input::{InputEvent, Key, KeyEvent, Modifiers, MouseAction, MouseBtn, MouseEvent};
pub use message::{Custom, Envelope, FocusNext, FocusPrevious, Message, Quit, Refresh};
