//! Message types for inter-layer communication in the actor-based architecture.
//!
//! This module defines all messages that flow between the UI, App, and Network layers.

pub mod network;
pub mod render;
pub mod ui_events;

pub use network::{NetworkCommand, NetworkResponse};
pub use render::RenderState;
pub use ui_events::UiEvent;
