use crate::app::state::{WsDirection, WsLogEntry};
use crate::app::AppState;
use crate::curl;
use crate::messages::ui_events::AuthField;
use crate::messages::{NetworkCommand, NetworkResponse};
use crate::models::{AuthType, Header, HistoryEntry, Request};

impl AppState {
    // ========================
    // HTTP Method
    // ========================

    pub fn cycle_method(&mut self) {
        if !self.http.is_loading {
            self.http.request.method = self.http.request.method.next();
        }
    }

    /// Toggle whether to ignore SSL certificate errors for this request.
    /// Useful for testing environments with self-signed certificates.
    pub fn toggle_ssl_errors(&mut self) {
        self.http.request.ignore_ssl_errors = !self.http.request.ignore_ssl_errors;
    }

    // ========================
    // Response scrolling
    // ========================

    pub fn scroll_up(&mut self) {
        self.http.response_scroll = self.http.response_scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.http.response_scroll = self.http.response_scroll.saturating_add(1);
    }

    // ========================
    // Headers
    // ========================

    pub fn next_header(&mut self) {
        if !self.http.request.headers.is_empty() {
            self.http.selected_header = (self.http.selected_header + 1) % self.http.request.headers.len();
        }
    }

    pub fn prev_header(&mut self) {
        if !self.http.request.headers.is_empty() {
            self.http.selected_header = self
                .http.selected_header
                .checked_sub(1)
                .unwrap_or(self.http.request.headers.len() - 1);
        }
    }

    pub fn toggle_header(&mut self) {
        if let Some(header) = self.http.request.headers.get_mut(self.http.selected_header) {
            header.enabled = !header.enabled;
        }
    }

    pub fn add_header(&mut self) {
        self.http.request.headers.push(Header::new("X-Custom", "value"));
        self.http.selected_header = self.http.request.headers.len() - 1;
    }

    pub fn delete_header(&mut self) {
        if !self.http.request.headers.is_empty() {
            self.http.request.headers.remove(self.http.selected_header);
            if self.http.selected_header > 0 {
                self.http.selected_header -= 1;
            }
        }
    }

    // ========================
    // Auth
    // ========================

    pub fn cycle_auth(&mut self) {
        self.http.request.auth = match &self.http.request.auth {
            AuthType::None => AuthType::Bearer(String::new()),
            AuthType::Bearer(_) => AuthType::Basic {
                username: String::new(),
                password: String::new(),
            },
            AuthType::Basic { .. } => AuthType::None,
        };
        self.http.auth_field = AuthField::Token;
    }

    pub fn next_auth_field(&mut self) {
        if matches!(self.http.request.auth, AuthType::Basic { .. }) {
            self.http.auth_field = match self.http.auth_field {
                AuthField::Username => AuthField::Password,
                AuthField::Password => AuthField::Username,
                _ => AuthField::Username,
            };
            self.ui.cursor_position = self.current_input().len();
        }
    }

    // ========================
    // History
    // ========================

    pub fn history_prev(&mut self) {
        if self.storage.history_len() == 0 {
            return;
        }

        let new_index = match self.history.index {
            Option::None => Some(0),
            Some(i) if i + 1 < self.storage.history_len() => Some(i + 1),
            Some(i) => Some(i),
        };

        if let Some(idx) = new_index {
            if let Some(entry) = self.storage.get_history(idx) {
                self.http.request = entry.request.clone();
                self.history.index = Some(idx);
                self.ui.cursor_position = self.http.request.url.len();
            }
        }
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.history.index {
            if idx > 0 {
                if let Some(entry) = self.storage.get_history(idx - 1) {
                    self.http.request = entry.request.clone();
                    self.history.index = Some(idx - 1);
                    self.ui.cursor_position = self.http.request.url.len();
                }
            } else {
                // Back to newest/empty
                self.http.request = Request::default();
                self.history.index = None;
                self.ui.cursor_position = self.http.request.url.len();
            }
        }
    }

    // ========================
    // cURL import/export
    // ========================

    pub fn show_curl_import(&mut self) {
        self.ui.show_curl_import = true;
    }

    pub fn curl_import_char(&mut self, c: char) {
        self.ui.curl_import_buffer.push(c);
    }

    pub fn curl_import_backspace(&mut self) {
        self.ui.curl_import_buffer.pop();
    }

    pub fn import_curl(&mut self) {
        if let Ok(request) = curl::parse_curl(&self.ui.curl_import_buffer) {
            self.http.request = request;
            self.ui.cursor_position = self.http.request.url.len();
        }
        self.ui.curl_import_buffer.clear();
        self.ui.show_curl_import = false;
    }

    pub fn cancel_curl_import(&mut self) {
        self.ui.curl_import_buffer.clear();
        self.ui.show_curl_import = false;
    }

    pub fn export_curl(&mut self) {
        self.http.response.body = curl::to_curl(&self.http.request);
        self.http.response.status_code = None;
    }

    // ========================
    // Help popup
    #[allow(dead_code)]
    pub fn prepare_request(&mut self) -> Option<NetworkCommand> {
        if self.http.is_loading {
            return None;
        }

        self.http.is_loading = true;
        self.http.response.body = String::from("Loading...");
        self.http.response.status_code = None;

        let id = self.next_id();
        self.http.pending_request_id = Some(id);

        Some(NetworkCommand::ExecuteRequest {
            id,
            request: self.http.request.clone(),
            environment: self.storage.current_environment().cloned(),
        })
    }

    /// Validate a URL and return error message if invalid
    pub(crate) fn validate_url(&self, url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        // Check for basic URL structure
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        // Check for host
        let without_scheme = url.split("://").nth(1).unwrap_or("");
        if without_scheme.is_empty() || without_scheme.starts_with('/') {
            return Err("URL must contain a host".to_string());
        }

        Ok(())
    }

    /// Prepare a streaming request (for large responses with incremental updates)
    pub fn prepare_streaming_request(&mut self) -> Option<NetworkCommand> {
        if self.http.is_loading {
            return None;
        }

        // Validate URL before sending
        if let Err(error) = self.validate_url(&self.http.request.url) {
            self.http.response.body = format!("Invalid URL: {}", error);
            self.http.response.status_code = None;
            return None;
        }

        self.http.is_loading = true;
        self.http.response.body = String::from("Starting request...");
        self.http.response.status_code = None;
        self.http.streaming_body.clear();
        self.http.bytes_received = 0;

        let id = self.next_id();
        self.http.pending_request_id = Some(id);

        Some(NetworkCommand::ExecuteStreamingRequest {
            id,
            request: self.http.request.clone(),
            environment: self.storage.current_environment().cloned(),
        })
    }

    /// Cancel the current pending request
    pub fn cancel_request(&mut self) -> Option<NetworkCommand> {
        self.http.pending_request_id.map(NetworkCommand::CancelRequest)
    }

    // ========================
    // Response handling
    // ========================

    pub fn handle_response(&mut self, response: NetworkResponse) {
        // Only process if it matches the pending request (for HTTP responses)
        let response_id = response.id();
        let is_for_pending = self.http.pending_request_id == Some(response_id);
        let is_for_gql = self.gql.pending_request_id == Some(response_id);

        match response {
            NetworkResponse::Success {
                status,
                body,
                time_ms,
                ..
            } => {
                if is_for_pending {
                    self.http.response.status_code = Some(status);
                    self.http.response.body = body;
                    self.http.response.time_ms = time_ms;
                    self.http.highlighted_response =
                        crate::tui::widgets::highlight_json(&self.http.response.body);
                    self.finalize_request();
                } else if is_for_gql {
                    self.gql.response = body;
                    self.gql.time_ms = time_ms;
                    self.gql.is_loading = false;
                    self.gql.pending_request_id = None;
                }
            }
            NetworkResponse::StreamChunk {
                chunk,
                bytes_received,
                ..
            } => {
                if is_for_pending {
                    // Append chunk to streaming body
                    self.http.streaming_body.push_str(&chunk);
                    self.http.bytes_received = bytes_received;
                    // Show streaming progress
                    self.http.response.body = format!(
                        "Streaming... {} bytes received\n\n{}",
                        bytes_received, self.http.streaming_body
                    );
                }
            }
            NetworkResponse::StreamComplete {
                status,
                total_bytes,
                time_ms,
                ..
            } => {
                if is_for_pending {
                    // Format final body as JSON if possible
                    let formatted = if let Ok(json) =
                        serde_json::from_str::<serde_json::Value>(&self.http.streaming_body)
                    {
                        serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| self.http.streaming_body.clone())
                    } else {
                        self.http.streaming_body.clone()
                    };

                    self.http.response.status_code = Some(status);
                    self.http.response.body = formatted;
                    self.http.response.time_ms = time_ms;
                    self.http.bytes_received = total_bytes;
                    self.http.highlighted_response =
                        crate::tui::widgets::highlight_json(&self.http.response.body);
                    self.finalize_request();
                }
            }
            NetworkResponse::Error {
                message, time_ms, ..
            } => {
                if is_for_pending {
                    self.http.response.status_code = None;
                    self.http.response.body = message;
                    self.http.response.time_ms = time_ms;
                    self.http.highlighted_response =
                        crate::tui::widgets::highlight_json(&self.http.response.body);
                    self.finalize_request();
                } else if is_for_gql {
                    self.gql.response = message;
                    self.gql.time_ms = time_ms;
                    self.gql.is_loading = false;
                    self.gql.pending_request_id = None;
                }
            }
            NetworkResponse::Cancelled { .. } => {
                if is_for_pending {
                    self.http.response.status_code = None;
                    self.http.response.body = String::from("Request cancelled");
                    self.http.response.time_ms = 0;
                    self.http.highlighted_response =
                        crate::tui::widgets::highlight_json(&self.http.response.body);
                    self.http.is_loading = false;
                    self.http.pending_request_id = None;
                    self.http.streaming_body.clear();
                    self.http.bytes_received = 0;
                }
            }
            // WebSocket responses
            NetworkResponse::WebSocketConnected { id } => {
                if self.ws.connection_id == Some(id) {
                    self.ws.connected = true;
                    self.ws.messages.push(WsLogEntry {
                        direction: WsDirection::System,
                        content: "Connected!".to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            NetworkResponse::WebSocketMessage { id, message } => {
                if self.ws.connection_id == Some(id) {
                    self.ws.messages.push(WsLogEntry {
                        direction: WsDirection::Received,
                        content: message,
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            NetworkResponse::WebSocketClosed { id } => {
                if self.ws.connection_id == Some(id) {
                    self.ws.connected = false;
                    self.ws.connection_id = None;
                    self.ws.messages.push(WsLogEntry {
                        direction: WsDirection::System,
                        content: "Connection closed".to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            NetworkResponse::WebSocketError { id, error } => {
                if self.ws.connection_id == Some(id) {
                    self.ws.connected = false;
                    self.ws.connection_id = None;
                    self.ws.messages.push(WsLogEntry {
                        direction: WsDirection::System,
                        content: format!("Error: {}", error),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }
    }

    // ========================
    // Tab navigation
    // ========================

    /// Finalize a completed request (add to history, reset state)
    pub(crate) fn finalize_request(&mut self) {
        self.http.is_loading = false;
        self.http.pending_request_id = None;
        self.http.response_scroll = 0;
        self.http.streaming_body.clear();
        self.http.bytes_received = 0;

        // Add to history
        let entry = HistoryEntry {
            request: self.http.request.clone(),
            response: self.http.response.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.storage.add_to_history(entry);
        self.history.index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::messages::ui_events::AuthField;
    use crate::models::{AuthType, Header, HistoryEntry, HttpMethod, Request, Response};

    fn make_state() -> AppState {
        AppState::new()
    }

    fn make_history_entry(url: &str) -> HistoryEntry {
        let mut req = Request::default();
        req.url = url.to_string();
        HistoryEntry {
            request: req,
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    // ── HTTP Method ──────────────────────────────────────────────────────────

    #[test]
    fn test_cycle_method_advances() {
        let mut state = make_state();
        assert_eq!(state.http.request.method, HttpMethod::GET);
        state.cycle_method();
        assert_eq!(state.http.request.method, HttpMethod::POST);
        state.cycle_method();
        assert_eq!(state.http.request.method, HttpMethod::PUT);
    }

    #[test]
    fn test_cycle_method_blocked_when_loading() {
        let mut state = make_state();
        state.http.is_loading = true;
        state.cycle_method();
        // Method must NOT change while a request is in flight
        assert_eq!(state.http.request.method, HttpMethod::GET);
    }

    #[test]
    fn test_toggle_ssl_errors() {
        let mut state = make_state();
        assert!(!state.http.request.ignore_ssl_errors);
        state.toggle_ssl_errors();
        assert!(state.http.request.ignore_ssl_errors);
        state.toggle_ssl_errors();
        assert!(!state.http.request.ignore_ssl_errors);
    }

    // ── Response scrolling ───────────────────────────────────────────────────

    #[test]
    fn test_scroll_up_does_not_underflow() {
        let mut state = make_state();
        state.http.response_scroll = 0;
        state.scroll_up();
        assert_eq!(state.http.response_scroll, 0);
    }

    #[test]
    fn test_scroll_down_increments() {
        let mut state = make_state();
        state.scroll_down();
        assert_eq!(state.http.response_scroll, 1);
        state.scroll_down();
        assert_eq!(state.http.response_scroll, 2);
    }

    // ── Headers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_add_header_appends_and_selects() {
        let mut state = make_state();
        let initial_len = state.http.request.headers.len();
        state.add_header();
        assert_eq!(state.http.request.headers.len(), initial_len + 1);
        assert_eq!(state.http.selected_header, initial_len);
    }

    #[test]
    fn test_delete_header_removes_selected() {
        let mut state = make_state();
        // Ensure at least 2 headers exist
        state.http.request.headers = vec![
            Header::new("A", "1"),
            Header::new("B", "2"),
            Header::new("C", "3"),
        ];
        state.http.selected_header = 1;
        state.delete_header();
        assert_eq!(state.http.request.headers.len(), 2);
        // Remaining headers should be A and C
        assert_eq!(state.http.request.headers[0].key, "A");
        assert_eq!(state.http.request.headers[1].key, "C");
    }

    #[test]
    fn test_toggle_header_flips_enabled() {
        let mut state = make_state();
        state.http.request.headers = vec![Header::new("X-Test", "value")];
        state.http.selected_header = 0;
        assert!(state.http.request.headers[0].enabled);
        state.toggle_header();
        assert!(!state.http.request.headers[0].enabled);
        state.toggle_header();
        assert!(state.http.request.headers[0].enabled);
    }

    #[test]
    fn test_next_header_wraps_around() {
        let mut state = make_state();
        state.http.request.headers = vec![
            Header::new("A", "1"),
            Header::new("B", "2"),
        ];
        state.http.selected_header = 1; // last
        state.next_header();
        assert_eq!(state.http.selected_header, 0); // wraps to first
    }

    #[test]
    fn test_prev_header_wraps_around() {
        let mut state = make_state();
        state.http.request.headers = vec![
            Header::new("A", "1"),
            Header::new("B", "2"),
        ];
        state.http.selected_header = 0; // first
        state.prev_header();
        assert_eq!(state.http.selected_header, 1); // wraps to last
    }

    // ── Auth ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_cycle_auth_none_to_bearer_to_basic_to_none() {
        let mut state = make_state();
        assert_eq!(state.http.request.auth, AuthType::None);

        state.cycle_auth();
        assert!(matches!(state.http.request.auth, AuthType::Bearer(_)));

        state.cycle_auth();
        assert!(matches!(state.http.request.auth, AuthType::Basic { .. }));

        state.cycle_auth();
        assert_eq!(state.http.request.auth, AuthType::None);
    }

    #[test]
    fn test_next_auth_field_toggles_in_basic() {
        let mut state = make_state();
        state.http.request.auth = AuthType::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        state.http.auth_field = AuthField::Username;
        state.next_auth_field();
        assert_eq!(state.http.auth_field, AuthField::Password);
        state.next_auth_field();
        assert_eq!(state.http.auth_field, AuthField::Username);
    }

    #[test]
    fn test_next_auth_field_no_op_for_bearer() {
        let mut state = make_state();
        state.http.request.auth = AuthType::Bearer("tok".to_string());
        state.http.auth_field = AuthField::Token;
        state.next_auth_field();
        // Should not change auth_field for Bearer
        assert_eq!(state.http.auth_field, AuthField::Token);
    }

    // ── URL validation ────────────────────────────────────────────────────────

    #[test]
    fn test_validate_url_empty_returns_error() {
        let state = make_state();
        assert!(state.validate_url("").is_err());
    }

    #[test]
    fn test_validate_url_no_scheme_returns_error() {
        let state = make_state();
        assert!(state.validate_url("api.example.com/users").is_err());
    }

    #[test]
    fn test_validate_url_http_scheme_ok() {
        let state = make_state();
        assert!(state.validate_url("http://api.example.com/users").is_ok());
    }

    #[test]
    fn test_validate_url_https_scheme_ok() {
        let state = make_state();
        assert!(state.validate_url("https://api.example.com/users").is_ok());
    }

    #[test]
    fn test_validate_url_no_host_returns_error() {
        let state = make_state();
        // scheme present but no host
        assert!(state.validate_url("https://").is_err());
    }

    // ── Request lifecycle ─────────────────────────────────────────────────────

    #[test]
    fn test_prepare_streaming_request_with_invalid_url() {
        let mut state = make_state();
        state.http.request.url = "not-a-url".to_string();
        let cmd = state.prepare_streaming_request();
        assert!(cmd.is_none(), "invalid URL should produce no command");
        assert!(state.http.response.body.contains("Invalid URL"));
        assert!(!state.http.is_loading);
    }

    #[test]
    fn test_prepare_streaming_request_blocked_when_loading() {
        let mut state = make_state();
        state.http.is_loading = true;
        state.http.request.url = "https://api.example.com".to_string();
        let cmd = state.prepare_streaming_request();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cancel_request_with_pending_id() {
        let mut state = make_state();
        state.http.pending_request_id = Some(42);
        let cmd = state.cancel_request();
        assert!(matches!(cmd, Some(NetworkCommand::CancelRequest(42))));
    }

    #[test]
    fn test_cancel_request_without_pending_id() {
        let mut state = make_state();
        state.http.pending_request_id = None;
        let cmd = state.cancel_request();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_finalize_request_resets_state_and_adds_to_history() {
        let mut state = make_state();
        state.http.is_loading = true;
        state.http.pending_request_id = Some(1);
        state.http.streaming_body = "partial...".to_string();
        state.http.bytes_received = 512;
        state.http.response_scroll = 5;

        state.finalize_request();

        assert!(!state.http.is_loading);
        assert!(state.http.pending_request_id.is_none());
        assert!(state.http.streaming_body.is_empty());
        assert_eq!(state.http.bytes_received, 0);
        assert_eq!(state.http.response_scroll, 0);
        // Entry was added to history
        assert_eq!(state.storage.history_len(), 1);
        // history_index reset
        assert!(state.history.index.is_none());
    }

    // ── NetworkResponse handling ──────────────────────────────────────────────

    #[test]
    fn test_handle_response_success_updates_state() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);
        state.http.is_loading = true;

        state.handle_response(NetworkResponse::Success {
            id,
            status: 200,
            body: r#"{"ok":true}"#.to_string(),
            time_ms: 42,
        });

        assert_eq!(state.http.response.status_code, Some(200));
        assert_eq!(state.http.response.body, r#"{"ok":true}"#);
        assert_eq!(state.http.response.time_ms, 42);
        assert!(!state.http.is_loading);
    }

    #[test]
    fn test_handle_response_error_updates_body() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);
        state.http.is_loading = true;

        state.handle_response(NetworkResponse::Error {
            id,
            message: "Connection refused".to_string(),
            time_ms: 10,
        });

        assert_eq!(state.http.response.body, "Connection refused");
        assert!(state.http.response.status_code.is_none());
        assert!(!state.http.is_loading);
    }

    #[test]
    fn test_handle_response_cancelled_resets_state() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);
        state.http.is_loading = true;
        state.http.streaming_body = "partial".to_string();

        state.handle_response(NetworkResponse::Cancelled { id });

        assert!(!state.http.is_loading);
        assert!(state.http.pending_request_id.is_none());
        assert!(state.http.streaming_body.is_empty());
        assert_eq!(state.http.response.body, "Request cancelled");
    }

    #[test]
    fn test_handle_response_wrong_id_does_not_update_state() {
        let mut state = make_state();
        let own_id = state.next_id();
        state.http.pending_request_id = Some(own_id);
        state.http.is_loading = true;

        let other_id = own_id + 99;
        state.handle_response(NetworkResponse::Success {
            id: other_id,
            status: 200,
            body: "should be ignored".to_string(),
            time_ms: 1,
        });

        // State should be unchanged — still loading, no status set
        assert!(state.http.is_loading);
        assert!(state.http.response.status_code.is_none());
    }

    #[test]
    fn test_handle_response_stream_chunk_accumulates() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);
        state.http.is_loading = true;

        state.handle_response(NetworkResponse::StreamChunk {
            id,
            chunk: "hello ".to_string(),
            bytes_received: 6,
        });
        state.handle_response(NetworkResponse::StreamChunk {
            id,
            chunk: "world".to_string(),
            bytes_received: 11,
        });

        assert_eq!(state.http.streaming_body, "hello world");
        assert_eq!(state.http.bytes_received, 11);
    }

    // ── cURL import/export ────────────────────────────────────────────────────

    #[test]
    fn test_import_curl_parses_and_loads_request() {
        let mut state = make_state();
        state.ui.curl_import_buffer =
            r#"curl -X POST https://api.example.com/items"#.to_string();
        state.import_curl();
        assert_eq!(state.http.request.method, HttpMethod::POST);
        assert_eq!(state.http.request.url, "https://api.example.com/items");
        assert!(!state.ui.show_curl_import);
        assert!(state.ui.curl_import_buffer.is_empty());
    }

    #[test]
    fn test_cancel_curl_import_clears_and_hides() {
        let mut state = make_state();
        state.ui.show_curl_import = true;
        state.ui.curl_import_buffer = "curl some junk".to_string();
        state.cancel_curl_import();
        assert!(!state.ui.show_curl_import);
        assert!(state.ui.curl_import_buffer.is_empty());
    }

    #[test]
    fn test_export_curl_writes_to_response_body() {
        let mut state = make_state();
        state.http.request.url = "https://api.example.com/users".to_string();
        state.export_curl();
        assert!(state.http.response.body.contains("curl"));
        assert!(state.http.response.body.contains("api.example.com"));
        assert!(state.http.response.status_code.is_none());
    }

    // ── History ───────────────────────────────────────────────────────────────

    #[test]
    fn test_history_prev_loads_previous_request() {
        let mut state = make_state();
        state.storage.add_to_history(make_history_entry("https://a.example.com"));
        state.history_prev();
        assert_eq!(state.http.request.url, "https://a.example.com");
        assert_eq!(state.history.index, Some(0));
    }

    #[test]
    fn test_history_next_after_prev_restores_default() {
        let mut state = make_state();
        state.storage.add_to_history(make_history_entry("https://a.example.com"));
        state.history_prev(); // go to index 0
        state.history_next(); // go back to "current" (no history)
        assert!(state.history.index.is_none());
    }
}
