//! HTTP orchestration — methods that coordinate across multiple sub-states.
//!
//! Pure HTTP domain logic lives in `http_state.rs` (impl HttpState).
//! This file contains only methods that need cross-state coordination
//! (e.g., http + storage, http + ui, http + history).

use crate::app::AppState;
use crate::messages::ui_events::InputMode;
use crate::messages::{NetworkCommand, NetworkResponse};
use crate::models::{HistoryEntry, Request};

impl AppState {
    // ========================
    // Auth (cross-state: http + ui)
    // ========================

    /// Cycle auth field and update UI cursor position.
    pub fn next_auth_field(&mut self) {
        if self.http.next_auth_field() {
            self.ui.cursor_position = self.current_input().len();
        }
    }

    // ========================
    // History (cross-state: storage + http + history + ui)
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
                self.http.request = Request::default();
                self.history.index = None;
                self.ui.cursor_position = self.http.request.url.len();
            }
        }
    }

    // ========================
    // cURL import (cross-state: ui + http)
    // ========================

    pub fn import_curl(&mut self) {
        if let Ok(request) = crate::curl::parse_curl(&self.ui.curl_import_buffer) {
            self.http.request = request;
            self.ui.cursor_position = self.http.request.url.len();
        }
        self.ui.curl_import_buffer.clear();
        self.ui.show_curl_import = false;
    }

    // ========================
    // Request lifecycle (cross-state: http + storage + next_id)
    // ========================

    pub fn prepare_streaming_request(&mut self) -> Option<NetworkCommand> {
        let id = self.next_id();
        let env = self.storage.current_environment().cloned();
        self.http.start_streaming(id, env)
    }

    /// Finalize a completed request (cross-state: http + storage + history)
    pub(crate) fn finalize_request(&mut self) {
        self.http.reset_after_complete();

        let entry = HistoryEntry {
            request: self.http.request.clone(),
            response: self.http.response.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.storage.add_to_history(entry);
        self.history.index = None;
    }

    // ========================
    // Response dispatch (central routing: http + gql + ws)
    // ========================

    pub fn handle_response(&mut self, response: NetworkResponse) {
        let response_id = response.id();
        let is_for_http = self.http.pending_request_id == Some(response_id);
        let is_for_gql = self.gql.pending_request_id == Some(response_id);

        match response {
            NetworkResponse::Success {
                status,
                body,
                time_ms,
                ..
            } => {
                if is_for_http {
                    self.http.apply_success(status, body, time_ms);
                    self.finalize_request();
                } else if is_for_gql {
                    self.gql.apply_response(response_id, body, time_ms);
                }
            }
            NetworkResponse::StreamChunk {
                chunk,
                bytes_received,
                ..
            } => {
                if is_for_http {
                    self.http.apply_stream_chunk(&chunk, bytes_received);
                }
            }
            NetworkResponse::StreamComplete {
                status,
                total_bytes,
                time_ms,
                ..
            } => {
                if is_for_http {
                    self.http.complete_stream(status, total_bytes, time_ms);
                    self.finalize_request();
                }
            }
            NetworkResponse::Error {
                message, time_ms, ..
            } => {
                if is_for_http {
                    self.http.apply_error(message, time_ms);
                    self.finalize_request();
                } else if is_for_gql {
                    self.gql.apply_error(response_id, message, time_ms);
                }
            }
            NetworkResponse::Cancelled { .. } => {
                if is_for_http {
                    self.http.apply_cancelled();
                }
            }
            // WebSocket responses — delegate to WebSocketState
            NetworkResponse::WebSocketConnected { id } => self.ws.on_connected(id),
            NetworkResponse::WebSocketMessage { id, message } => self.ws.on_message(id, message),
            NetworkResponse::WebSocketClosed { id } => self.ws.on_closed(id),
            NetworkResponse::WebSocketError { id, error } => self.ws.on_error(id, error),
        }
    }

    // ========================
    // WebSocket orchestration (cross-state: ws + next_id + ui)
    // ========================

    pub fn ws_connect(&mut self) -> Option<NetworkCommand> {
        let id = self.next_id();
        self.ws.connect(id)
    }

    pub fn ws_start_url_edit(&mut self) {
        self.ws.start_url_edit();
        self.ui.input_mode = InputMode::Editing;
    }

    pub fn ws_start_input_edit(&mut self) {
        self.ws.start_input_edit();
        self.ui.input_mode = InputMode::Editing;
    }

    // ========================
    // GraphQL orchestration (cross-state: gql + http.headers/auth + next_id + ui)
    // ========================

    pub fn gql_execute_query(&mut self) -> Option<NetworkCommand> {
        use crate::app::state::HttpState;

        if self.gql.is_loading {
            return None;
        }

        if let Err(error) = HttpState::validate_url(&self.gql.endpoint) {
            self.gql.response = format!("Invalid endpoint: {}", error);
            return None;
        }

        self.gql.is_loading = true;
        self.gql.response = String::from("Executing query...");

        let id = self.next_id();
        self.gql.pending_request_id = Some(id);

        let variables = if self.gql.variables.trim().is_empty() || self.gql.variables.trim() == "{}"
        {
            None
        } else {
            Some(self.gql.variables.clone())
        };

        Some(NetworkCommand::ExecuteGraphQL {
            id,
            endpoint: self.gql.endpoint.clone(),
            query: self.gql.query.clone(),
            variables,
            headers: self.http.request.headers.clone(),
            auth: self.http.request.auth.clone(),
        })
    }

    pub fn gql_edit_endpoint(&mut self) {
        self.gql.edit_endpoint();
        self.ui.input_mode = InputMode::Editing;
    }

    pub fn gql_edit_query(&mut self) {
        self.gql.edit_query();
        self.ui.input_mode = InputMode::Editing;
    }

    pub fn gql_edit_variables(&mut self) {
        self.gql.edit_variables();
        self.ui.input_mode = InputMode::Editing;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::messages::ui_events::AuthField;
    use crate::messages::NetworkResponse;
    use crate::models::{AuthType, HistoryEntry, HttpMethod, Request, Response};

    fn make_state() -> AppState {
        AppState::new()
    }

    fn make_history_entry(url: &str) -> HistoryEntry {
        let req = Request {
            url: url.to_string(),
            ..Request::default()
        };
        HistoryEntry {
            request: req,
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    // ── Auth orchestration ────────────────────────────────────────────────────

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
    }

    #[test]
    fn test_next_auth_field_no_op_for_bearer() {
        let mut state = make_state();
        state.http.request.auth = AuthType::Bearer("tok".to_string());
        state.http.auth_field = AuthField::Token;
        state.next_auth_field();
        assert_eq!(state.http.auth_field, AuthField::Token);
    }

    // ── Request lifecycle ─────────────────────────────────────────────────────

    #[test]
    fn test_prepare_streaming_request_with_invalid_url() {
        let mut state = make_state();
        state.http.request.url = "not-a-url".to_string();
        let cmd = state.prepare_streaming_request();
        assert!(cmd.is_none());
        assert!(state.http.response.body.contains("Invalid URL"));
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
        assert_eq!(state.storage.history_len(), 1);
        assert!(state.history.index.is_none());
    }

    // ── Response handling ─────────────────────────────────────────────────────

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
        assert!(!state.http.is_loading);
    }

    #[test]
    fn test_handle_response_cancelled_resets_state() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);
        state.http.is_loading = true;

        state.handle_response(NetworkResponse::Cancelled { id });

        assert!(!state.http.is_loading);
        assert_eq!(state.http.response.body, "Request cancelled");
    }

    #[test]
    fn test_handle_response_wrong_id_does_not_update_state() {
        let mut state = make_state();
        let own_id = state.next_id();
        state.http.pending_request_id = Some(own_id);
        state.http.is_loading = true;

        state.handle_response(NetworkResponse::Success {
            id: own_id + 99,
            status: 200,
            body: "should be ignored".to_string(),
            time_ms: 1,
        });

        assert!(state.http.is_loading);
        assert!(state.http.response.status_code.is_none());
    }

    #[test]
    fn test_handle_response_stream_chunk_accumulates() {
        let mut state = make_state();
        let id = state.next_id();
        state.http.pending_request_id = Some(id);

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
    }

    // ── cURL import ──────────────────────────────────────────────────────────

    #[test]
    fn test_import_curl_parses_and_loads_request() {
        let mut state = make_state();
        state.ui.curl_import_buffer = r#"curl -X POST https://api.example.com/items"#.to_string();
        state.import_curl();
        assert_eq!(state.http.request.method, HttpMethod::POST);
        assert_eq!(state.http.request.url, "https://api.example.com/items");
        assert!(!state.ui.show_curl_import);
    }

    // ── History ───────────────────────────────────────────────────────────────

    #[test]
    fn test_history_prev_loads_previous_request() {
        let mut state = make_state();
        state
            .storage
            .add_to_history(make_history_entry("https://a.example.com"));
        state.history_prev();
        assert_eq!(state.http.request.url, "https://a.example.com");
        assert_eq!(state.history.index, Some(0));
    }

    #[test]
    fn test_history_next_after_prev_restores_default() {
        let mut state = make_state();
        state
            .storage
            .add_to_history(make_history_entry("https://a.example.com"));
        state.history_prev();
        state.history_next();
        assert!(state.history.index.is_none());
    }

    // ── WS orchestration ─────────────────────────────────────────────────────

    #[test]
    fn test_ws_start_url_edit_sets_editing_and_input_mode() {
        let mut state = make_state();
        state.ws_start_url_edit();
        assert!(state.ws.editing_url);
        assert_eq!(
            state.ui.input_mode,
            crate::messages::ui_events::InputMode::Editing
        );
    }

    // ── GQL orchestration ────────────────────────────────────────────────────

    #[test]
    fn test_gql_edit_endpoint_sets_field_and_input_mode() {
        let mut state = make_state();
        state.gql_edit_endpoint();
        assert_eq!(
            state.gql.active_field,
            crate::messages::ui_events::GqlField::Endpoint
        );
        assert_eq!(
            state.ui.input_mode,
            crate::messages::ui_events::InputMode::Editing
        );
    }
}
