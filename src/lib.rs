//! # Freeman TUI
//!
//! A minimal terminal-based API testing tool, similar to Postman/Insomnia.
//!
//! ## Features
//! - HTTP methods: GET, POST, PUT, PATCH, DELETE
//! - Request body editor
//! - Custom headers
//! - Auth support (Bearer, Basic)
//! - Request history
//! - cURL import/export
//! - JSON syntax highlighting
//! - Workspace discovery (OpenAPI, FastAPI, Express)
//!
//! ## Architecture
//! Actor-based with channels:
//! - UI Layer (Ratatui) - synchronous
//! - App Layer (State machine)
//! - Network Layer (Tokio runtime)

pub mod app;
pub mod constants;
pub mod curl;
pub mod discovery;
pub mod messages;
pub mod models;
pub mod network;
pub mod storage;
pub mod tui;
pub mod ui;

// Re-export commonly used types
pub use app::{AppActor, AppState};
pub use curl::{parse_curl, to_curl};
pub use discovery::{DiscoveredEndpoint, Framework, WorkspaceProject};
pub use messages::{NetworkCommand, NetworkResponse, RenderState, UiEvent};
pub use models::{AuthType, Collection, Environment, Header, HttpMethod, Request};
pub use network::NetworkActor;
