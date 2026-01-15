//! Network layer - HTTP request execution and WebSocket connections
//!
//! The Network actor receives HTTP/WS commands and sends back responses.

pub mod actor;
pub mod client;
pub mod websocket;

pub use actor::NetworkActor;
