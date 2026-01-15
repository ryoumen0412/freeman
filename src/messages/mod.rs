//! Message types for inter-layer communication in the actor-based architecture.
//!
//! This module defines all messages that flow between the UI, App, and Network layers.

pub mod ui_events;
pub mod network;
pub mod render;

pub use ui_events::UiEvent;
pub use network::{NetworkCommand, NetworkResponse};
pub use render::RenderState;
