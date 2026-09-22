//! HTTP domain logic — methods that operate purely on HttpState.

use crate::app::state::HttpState;
use crate::messages::ui_events::AuthField;
use crate::messages::NetworkCommand;
use crate::models::{AuthType, Header};

impl HttpState {
    // ========================
    // HTTP Method
    // ========================

    pub fn cycle_method(&mut self) {
        if !self.is_loading {
            self.request.method = self.request.method.next();
        }
    }

    pub fn toggle_ssl_errors(&mut self) {
        self.request.ignore_ssl_errors = !self.request.ignore_ssl_errors;
    }

    // ========================
    // Response scrolling
    // ========================

    pub fn scroll_up(&mut self) {
        self.response_scroll = self.response_scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.response_scroll = self.response_scroll.saturating_add(1);
    }

    // ========================
    // Headers
    // ========================

    pub fn next_header(&mut self) {
        if !self.request.headers.is_empty() {
            self.selected_header = (self.selected_header + 1) % self.request.headers.len();
        }
    }

    pub fn prev_header(&mut self) {
        if !self.request.headers.is_empty() {
            self.selected_header = self
                .selected_header
                .checked_sub(1)
                .unwrap_or(self.request.headers.len() - 1);
        }
    }

    pub fn toggle_header(&mut self) {
        if let Some(header) = self.request.headers.get_mut(self.selected_header) {
            header.enabled = !header.enabled;
        }
    }

    pub fn add_header(&mut self) {
        self.request.headers.push(Header::new("X-Custom", "value"));
        self.selected_header = self.request.headers.len() - 1;
    }

    pub fn delete_header(&mut self) {
        if !self.request.headers.is_empty() {
            self.request.headers.remove(self.selected_header);
            if self.selected_header > 0 {
                self.selected_header -= 1;
            }
        }
    }

    // ========================
    // Auth
    // ========================

    pub fn cycle_auth(&mut self) {
        self.request.auth = match &self.request.auth {
            AuthType::None => AuthType::Bearer(String::new()),
            AuthType::Bearer(_) => AuthType::Basic {
                username: String::new(),
                password: String::new(),
            },
            AuthType::Basic { .. } => AuthType::None,
        };
        self.auth_field = AuthField::Token;
    }

    /// Cycle auth field within Basic auth. Returns true if cursor needs updating.
    pub fn next_auth_field(&mut self) -> bool {
        if matches!(self.request.auth, AuthType::Basic { .. }) {
            self.auth_field = match self.auth_field {
                AuthField::Username => AuthField::Password,
                AuthField::Password => AuthField::Username,
                _ => AuthField::Username,
            };
            true
        } else {
            false
        }
    }

    // ========================
    // cURL export
    // ========================

    pub fn export_curl(&mut self) {
        self.response.body = crate::curl::to_curl(&self.request);
        self.response.status_code = None;
    }

    // ========================
    // URL validation
    // ========================

    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        let without_scheme = url.split("://").nth(1).unwrap_or("");
        if without_scheme.is_empty() || without_scheme.starts_with('/') {
            return Err("URL must contain a host".to_string());
        }

        Ok(())
    }

    // ========================
    // Request lifecycle (self-contained parts)
    // ========================

    pub fn cancel_request(&self) -> Option<NetworkCommand> {
        self.pending_request_id.map(NetworkCommand::CancelRequest)
    }

    /// Start a streaming request. Returns the NetworkCommand if ready.
    /// Caller provides `id` and `environment` (from outside HttpState).
    pub fn start_streaming(
        &mut self,
        id: u64,
        environment: Option<crate::models::Environment>,
    ) -> Option<NetworkCommand> {
        if self.is_loading {
            return None;
        }

        if let Err(error) = Self::validate_url(&self.request.url) {
            self.response.body = format!("Invalid URL: {}", error);
            self.response.status_code = None;
            return None;
        }

        self.is_loading = true;
        self.response.body = String::from("Starting request...");
        self.response.status_code = None;
        self.streaming_body.clear();
        self.bytes_received = 0;
        self.pending_request_id = Some(id);

        Some(NetworkCommand::ExecuteStreamingRequest {
            id,
            request: self.request.clone(),
            environment,
        })
    }

    // ========================
    // Response handling (self-contained parts)
    // ========================

    pub fn apply_success(&mut self, status: u16, body: String, time_ms: u64) {
        self.response.status_code = Some(status);
        self.response.body = body;
        self.response.time_ms = time_ms;
        self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
    }

    pub fn apply_stream_chunk(&mut self, chunk: &str, bytes_received: usize) {
        self.streaming_body.push_str(chunk);
        self.bytes_received = bytes_received;
        self.response.body = format!(
            "Streaming... {} bytes received\n\n{}",
            bytes_received, self.streaming_body
        );
    }

    pub fn complete_stream(&mut self, status: u16, total_bytes: usize, time_ms: u64) {
        let formatted =
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.streaming_body) {
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| self.streaming_body.clone())
            } else {
                self.streaming_body.clone()
            };

        self.response.status_code = Some(status);
        self.response.body = formatted;
        self.response.time_ms = time_ms;
        self.bytes_received = total_bytes;
        self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
    }

    pub fn apply_error(&mut self, message: String, time_ms: u64) {
        self.response.status_code = None;
        self.response.body = message;
        self.response.time_ms = time_ms;
        self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
    }

    pub fn apply_cancelled(&mut self) {
        self.response.status_code = None;
        self.response.body = String::from("Request cancelled");
        self.response.time_ms = 0;
        self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
        self.is_loading = false;
        self.pending_request_id = None;
        self.streaming_body.clear();
        self.bytes_received = 0;
    }

    /// Reset loading/streaming state after a completed request.
    /// Does NOT handle history — that's the orchestrator's job.
    pub fn reset_after_complete(&mut self) {
        self.is_loading = false;
        self.pending_request_id = None;
        self.response_scroll = 0;
        self.streaming_body.clear();
        self.bytes_received = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::HttpState;
    use crate::messages::ui_events::AuthField;
    use crate::models::{AuthType, Header, HttpMethod};

    fn make_http() -> HttpState {
        // Build a default HttpState matching AppState::new()
        HttpState {
            request: crate::models::Request::default(),
            response: crate::models::Response::default(),
            highlighted_response: crate::tui::widgets::highlight_json(
                &crate::models::Response::default().body,
            ),
            response_scroll: 0,
            is_loading: false,
            pending_request_id: None,
            streaming_body: String::new(),
            bytes_received: 0,
            selected_header: 0,
            auth_field: AuthField::Token,
        }
    }

    // ── HTTP Method ──────────────────────────────────────────────────────────

    #[test]
    fn test_cycle_method_advances() {
        let mut http = make_http();
        assert_eq!(http.request.method, HttpMethod::GET);
        http.cycle_method();
        assert_eq!(http.request.method, HttpMethod::POST);
        http.cycle_method();
        assert_eq!(http.request.method, HttpMethod::PUT);
    }

    #[test]
    fn test_cycle_method_blocked_when_loading() {
        let mut http = make_http();
        http.is_loading = true;
        http.cycle_method();
        assert_eq!(http.request.method, HttpMethod::GET);
    }

    #[test]
    fn test_toggle_ssl_errors() {
        let mut http = make_http();
        assert!(!http.request.ignore_ssl_errors);
        http.toggle_ssl_errors();
        assert!(http.request.ignore_ssl_errors);
        http.toggle_ssl_errors();
        assert!(!http.request.ignore_ssl_errors);
    }

    // ── Response scrolling ───────────────────────────────────────────────────

    #[test]
    fn test_scroll_up_does_not_underflow() {
        let mut http = make_http();
        http.response_scroll = 0;
        http.scroll_up();
        assert_eq!(http.response_scroll, 0);
    }

    #[test]
    fn test_scroll_down_increments() {
        let mut http = make_http();
        http.scroll_down();
        assert_eq!(http.response_scroll, 1);
        http.scroll_down();
        assert_eq!(http.response_scroll, 2);
    }

    // ── Headers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_add_header_appends_and_selects() {
        let mut http = make_http();
        let initial_len = http.request.headers.len();
        http.add_header();
        assert_eq!(http.request.headers.len(), initial_len + 1);
        assert_eq!(http.selected_header, initial_len);
    }

    #[test]
    fn test_delete_header_removes_selected() {
        let mut http = make_http();
        http.request.headers = vec![
            Header::new("A", "1"),
            Header::new("B", "2"),
            Header::new("C", "3"),
        ];
        http.selected_header = 1;
        http.delete_header();
        assert_eq!(http.request.headers.len(), 2);
        assert_eq!(http.request.headers[0].key, "A");
        assert_eq!(http.request.headers[1].key, "C");
    }

    #[test]
    fn test_toggle_header_flips_enabled() {
        let mut http = make_http();
        http.request.headers = vec![Header::new("X-Test", "value")];
        http.selected_header = 0;
        assert!(http.request.headers[0].enabled);
        http.toggle_header();
        assert!(!http.request.headers[0].enabled);
        http.toggle_header();
        assert!(http.request.headers[0].enabled);
    }

    #[test]
    fn test_next_header_wraps_around() {
        let mut http = make_http();
        http.request.headers = vec![Header::new("A", "1"), Header::new("B", "2")];
        http.selected_header = 1;
        http.next_header();
        assert_eq!(http.selected_header, 0);
    }

    #[test]
    fn test_prev_header_wraps_around() {
        let mut http = make_http();
        http.request.headers = vec![Header::new("A", "1"), Header::new("B", "2")];
        http.selected_header = 0;
        http.prev_header();
        assert_eq!(http.selected_header, 1);
    }

    // ── Auth ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_cycle_auth_none_to_bearer_to_basic_to_none() {
        let mut http = make_http();
        assert_eq!(http.request.auth, AuthType::None);
        http.cycle_auth();
        assert!(matches!(http.request.auth, AuthType::Bearer(_)));
        http.cycle_auth();
        assert!(matches!(http.request.auth, AuthType::Basic { .. }));
        http.cycle_auth();
        assert_eq!(http.request.auth, AuthType::None);
    }

    #[test]
    fn test_next_auth_field_toggles_in_basic() {
        let mut http = make_http();
        http.request.auth = AuthType::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        http.auth_field = AuthField::Username;
        assert!(http.next_auth_field());
        assert_eq!(http.auth_field, AuthField::Password);
        assert!(http.next_auth_field());
        assert_eq!(http.auth_field, AuthField::Username);
    }

    #[test]
    fn test_next_auth_field_no_op_for_bearer() {
        let mut http = make_http();
        http.request.auth = AuthType::Bearer("tok".to_string());
        http.auth_field = AuthField::Token;
        assert!(!http.next_auth_field());
        assert_eq!(http.auth_field, AuthField::Token);
    }

    // ── URL validation ────────────────────────────────────────────────────────

    #[test]
    fn test_validate_url_empty_returns_error() {
        assert!(HttpState::validate_url("").is_err());
    }

    #[test]
    fn test_validate_url_no_scheme_returns_error() {
        assert!(HttpState::validate_url("api.example.com/users").is_err());
    }

    #[test]
    fn test_validate_url_http_scheme_ok() {
        assert!(HttpState::validate_url("http://api.example.com/users").is_ok());
    }

    #[test]
    fn test_validate_url_https_scheme_ok() {
        assert!(HttpState::validate_url("https://api.example.com/users").is_ok());
    }

    #[test]
    fn test_validate_url_no_host_returns_error() {
        assert!(HttpState::validate_url("https://").is_err());
    }

    // ── Request lifecycle ─────────────────────────────────────────────────────

    #[test]
    fn test_start_streaming_with_invalid_url() {
        let mut http = make_http();
        http.request.url = "not-a-url".to_string();
        let cmd = http.start_streaming(1, None);
        assert!(cmd.is_none());
        assert!(http.response.body.contains("Invalid URL"));
        assert!(!http.is_loading);
    }

    #[test]
    fn test_start_streaming_blocked_when_loading() {
        let mut http = make_http();
        http.is_loading = true;
        http.request.url = "https://api.example.com".to_string();
        let cmd = http.start_streaming(1, None);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cancel_request_with_pending_id() {
        let mut http = make_http();
        http.pending_request_id = Some(42);
        let cmd = http.cancel_request();
        assert!(matches!(
            cmd,
            Some(crate::messages::NetworkCommand::CancelRequest(42))
        ));
    }

    #[test]
    fn test_cancel_request_without_pending_id() {
        let http = make_http();
        let cmd = http.cancel_request();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_reset_after_complete() {
        let mut http = make_http();
        http.is_loading = true;
        http.pending_request_id = Some(1);
        http.streaming_body = "partial...".to_string();
        http.bytes_received = 512;
        http.response_scroll = 5;

        http.reset_after_complete();

        assert!(!http.is_loading);
        assert!(http.pending_request_id.is_none());
        assert!(http.streaming_body.is_empty());
        assert_eq!(http.bytes_received, 0);
        assert_eq!(http.response_scroll, 0);
    }

    // ── Response application ──────────────────────────────────────────────────

    #[test]
    fn test_apply_success_updates_state() {
        let mut http = make_http();
        http.apply_success(200, r#"{"ok":true}"#.to_string(), 42);
        assert_eq!(http.response.status_code, Some(200));
        assert_eq!(http.response.body, r#"{"ok":true}"#);
        assert_eq!(http.response.time_ms, 42);
    }

    #[test]
    fn test_apply_error_updates_body() {
        let mut http = make_http();
        http.apply_error("Connection refused".to_string(), 10);
        assert_eq!(http.response.body, "Connection refused");
        assert!(http.response.status_code.is_none());
    }

    #[test]
    fn test_apply_cancelled_resets_state() {
        let mut http = make_http();
        http.is_loading = true;
        http.pending_request_id = Some(1);
        http.streaming_body = "partial".to_string();
        http.apply_cancelled();
        assert!(!http.is_loading);
        assert!(http.pending_request_id.is_none());
        assert!(http.streaming_body.is_empty());
        assert_eq!(http.response.body, "Request cancelled");
    }

    #[test]
    fn test_apply_stream_chunk_accumulates() {
        let mut http = make_http();
        http.apply_stream_chunk("hello ", 6);
        http.apply_stream_chunk("world", 11);
        assert_eq!(http.streaming_body, "hello world");
        assert_eq!(http.bytes_received, 11);
    }

    // ── cURL export ──────────────────────────────────────────────────────────

    #[test]
    fn test_export_curl_writes_to_response_body() {
        let mut http = make_http();
        http.request.url = "https://api.example.com/users".to_string();
        http.export_curl();
        assert!(http.response.body.contains("curl"));
        assert!(http.response.body.contains("api.example.com"));
        assert!(http.response.status_code.is_none());
    }
}
