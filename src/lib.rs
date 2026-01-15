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

pub mod models;
pub mod storage;
pub mod ui;
pub mod curl;
pub mod discovery;
pub mod messages;
pub mod app;
pub mod network;

// Re-export commonly used types
pub use models::{Request, HttpMethod, Header, AuthType, Collection, Environment};
pub use curl::{parse_curl, to_curl};
pub use discovery::{DiscoveredEndpoint, WorkspaceProject, Framework};
pub use messages::{UiEvent, NetworkCommand, NetworkResponse, RenderState};
pub use app::{AppState, AppActor};
pub use network::NetworkActor;
