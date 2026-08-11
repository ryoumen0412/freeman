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
        if !self.is_loading {
            self.request.method = self.request.method.next();
        }
    }

    /// Toggle whether to ignore SSL certificate errors for this request.
    /// Useful for testing environments with self-signed certificates.
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

    pub fn next_auth_field(&mut self) {
        if matches!(self.request.auth, AuthType::Basic { .. }) {
            self.auth_field = match self.auth_field {
                AuthField::Username => AuthField::Password,
                AuthField::Password => AuthField::Username,
                _ => AuthField::Username,
            };
            self.cursor_position = self.current_input().len();
        }
    }

    // ========================
    // History
    // ========================

    pub fn history_prev(&mut self) {
        if self.storage.history_len() == 0 {
            return;
        }

        let new_index = match self.history_index {
            Option::None => Some(0),
            Some(i) if i + 1 < self.storage.history_len() => Some(i + 1),
            Some(i) => Some(i),
        };

        if let Some(idx) = new_index {
            if let Some(entry) = self.storage.get_history(idx) {
                self.request = entry.request.clone();
                self.history_index = Some(idx);
                self.cursor_position = self.request.url.len();
            }
        }
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.history_index {
            if idx > 0 {
                if let Some(entry) = self.storage.get_history(idx - 1) {
                    self.request = entry.request.clone();
                    self.history_index = Some(idx - 1);
                    self.cursor_position = self.request.url.len();
                }
            } else {
                // Back to newest/empty
                self.request = Request::default();
                self.history_index = None;
                self.cursor_position = self.request.url.len();
            }
        }
    }

    // ========================
    // cURL import/export
    // ========================

    pub fn show_curl_import(&mut self) {
        self.show_curl_import = true;
    }

    pub fn curl_import_char(&mut self, c: char) {
        self.curl_import_buffer.push(c);
    }

    pub fn curl_import_backspace(&mut self) {
        self.curl_import_buffer.pop();
    }

    pub fn import_curl(&mut self) {
        if let Ok(request) = curl::parse_curl(&self.curl_import_buffer) {
            self.request = request;
            self.cursor_position = self.request.url.len();
        }
        self.curl_import_buffer.clear();
        self.show_curl_import = false;
    }

    pub fn cancel_curl_import(&mut self) {
        self.curl_import_buffer.clear();
        self.show_curl_import = false;
    }

    pub fn export_curl(&mut self) {
        self.response.body = curl::to_curl(&self.request);
        self.response.status_code = None;
    }

    // ========================
    // Help popup
    #[allow(dead_code)]
    pub fn prepare_request(&mut self) -> Option<NetworkCommand> {
        if self.is_loading {
            return None;
        }

        self.is_loading = true;
        self.response.body = String::from("Loading...");
        self.response.status_code = None;

        let id = self.next_id();
        self.pending_request_id = Some(id);

        Some(NetworkCommand::ExecuteRequest {
            id,
            request: self.request.clone(),
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
        if self.is_loading {
            return None;
        }

        // Validate URL before sending
        if let Err(error) = self.validate_url(&self.request.url) {
            self.response.body = format!("Invalid URL: {}", error);
            self.response.status_code = None;
            return None;
        }

        self.is_loading = true;
        self.response.body = String::from("Starting request...");
        self.response.status_code = None;
        self.streaming_body.clear();
        self.bytes_received = 0;

        let id = self.next_id();
        self.pending_request_id = Some(id);

        Some(NetworkCommand::ExecuteStreamingRequest {
            id,
            request: self.request.clone(),
            environment: self.storage.current_environment().cloned(),
        })
    }

    /// Cancel the current pending request
    pub fn cancel_request(&mut self) -> Option<NetworkCommand> {
        self.pending_request_id.map(NetworkCommand::CancelRequest)
    }

    // ========================
    // Response handling
    // ========================

    pub fn handle_response(&mut self, response: NetworkResponse) {
        // Only process if it matches the pending request (for HTTP responses)
        let response_id = response.id();
        let is_for_pending = self.pending_request_id == Some(response_id);
        let is_for_gql = self.gql.pending_request_id == Some(response_id);

        match response {
            NetworkResponse::Success {
                status,
                body,
                time_ms,
                ..
            } => {
                if is_for_pending {
                    self.response.status_code = Some(status);
                    self.response.body = body;
                    self.response.time_ms = time_ms;
                    self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
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
                    self.streaming_body.push_str(&chunk);
                    self.bytes_received = bytes_received;
                    // Show streaming progress
                    self.response.body = format!(
                        "Streaming... {} bytes received\n\n{}",
                        bytes_received, &self.streaming_body
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
                        serde_json::from_str::<serde_json::Value>(&self.streaming_body)
                    {
                        serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| self.streaming_body.clone())
                    } else {
                        self.streaming_body.clone()
                    };

                    self.response.status_code = Some(status);
                    self.response.body = formatted;
                    self.response.time_ms = time_ms;
                    self.bytes_received = total_bytes;
                    self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
                    self.finalize_request();
                }
            }
            NetworkResponse::Error {
                message, time_ms, ..
            } => {
                if is_for_pending {
                    self.response.status_code = None;
                    self.response.body = message;
                    self.response.time_ms = time_ms;
                    self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
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
                    self.response.status_code = None;
                    self.response.body = String::from("Request cancelled");
                    self.response.time_ms = 0;
                    self.highlighted_response = crate::tui::widgets::highlight_json(&self.response.body);
                    self.is_loading = false;
                    self.pending_request_id = None;
                    self.streaming_body.clear();
                    self.bytes_received = 0;
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
        self.is_loading = false;
        self.pending_request_id = None;
        self.response_scroll = 0;
        self.streaming_body.clear();
        self.bytes_received = 0;

        // Add to history
        let entry = HistoryEntry {
            request: self.request.clone(),
            response: self.response.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.storage.add_to_history(entry);
        self.history_index = None;
    }
}
