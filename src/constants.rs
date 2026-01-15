//! Application constants
//!
//! Centralized location for magic strings and configuration defaults.

/// Default URL for new HTTP requests
pub const DEFAULT_HTTP_URL: &str = "https://httpbin.org/get";

/// Default URL for WebSocket connections
pub const DEFAULT_WS_URL: &str = "wss://echo.websocket.org";

/// Application name
#[allow(dead_code)]
pub const APP_NAME: &str = "Freeman TUI";

/// Application version
#[allow(dead_code)]
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
