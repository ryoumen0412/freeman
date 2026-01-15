//! Command handlers - business logic for processing UI events

use std::path::PathBuf;

use crate::app::AppState;
use crate::app::state::{WsDirection, WsLogEntry};
use crate::models::{AuthType, Header, HistoryEntry, HttpMethod, Request};
use crate::messages::ui_events::{AppTab, AuthField, InputMode, Panel};
use crate::messages::{NetworkCommand, NetworkResponse};
use crate::discovery::{self, DiscoveredEndpoint, detector, openapi};
use crate::curl;

impl AppState {
    // ========================
    // Navigation
    // ========================
    
    pub fn next_panel(&mut self) {
        self.active_panel = self.active_panel.next();
    }
    
    pub fn prev_panel(&mut self) {
        self.active_panel = self.active_panel.prev();
    }
    
    pub fn focus_workspace(&mut self) {
        self.active_panel = Panel::Workspace;
    }
    
    // ========================
    // Input editing
    // ========================
    
    pub fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing;
        self.cursor_position = self.current_input().len();
    }
    
    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }
    
    pub fn move_cursor_left(&mut self) {
        let input = self.current_input();
        if self.cursor_position > 0 {
            let new_pos = input[..self.cursor_position]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor_position = new_pos;
        }
    }
    
    pub fn move_cursor_right(&mut self) {
        let input = self.current_input();
        if self.cursor_position < input.len() {
            let new_pos = input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_position + i)
                .unwrap_or(input.len());
            self.cursor_position = new_pos;
        }
    }
    
    pub fn enter_char(&mut self, c: char) {
        let cursor_pos = self.cursor_position;
        let input = self.current_input_mut();
        if cursor_pos <= input.len() {
            input.insert(cursor_pos, c);
            self.cursor_position = cursor_pos + c.len_utf8();
        }
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let cursor_pos = self.cursor_position;
            let input = self.current_input_mut();
            let prev_pos = input[..cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            input.remove(prev_pos);
            self.cursor_position = prev_pos;
        }
    }
    
    // ========================
    // HTTP Method
    // ========================
    
    pub fn cycle_method(&mut self) {
        if !self.is_loading {
            self.request.method = self.request.method.next();
        }
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
            self.selected_header = self.selected_header
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
            None => Some(0),
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
    // ========================
    
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
    
    pub fn close_help(&mut self) {
        self.show_help = false;
    }
    
    // ========================
    // Workspace
    // ========================
    
    pub fn open_workspace_input(&mut self) {
        self.show_workspace_input = true;
    }
    
    pub fn workspace_path_char(&mut self, c: char) {
        self.workspace_path_input.push(c);
    }
    
    pub fn workspace_path_backspace(&mut self) {
        self.workspace_path_input.pop();
    }
    
    pub fn cancel_workspace_input(&mut self) {
        self.show_workspace_input = false;
        self.workspace_path_input.clear();
    }
    
    pub fn workspace_path_autocomplete(&mut self) {
        use std::fs;
        
        // Expand ~ to home directory
        let input = if self.workspace_path_input.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                self.workspace_path_input.replacen("~", &home.to_string_lossy(), 1)
            } else {
                return;
            }
        } else {
            self.workspace_path_input.clone()
        };

        let path = PathBuf::from(&input);
        
        // If it's already a valid directory, try to complete further
        if path.is_dir() && !input.ends_with('/') {
            self.workspace_path_input = format!("{}/", input);
            return;
        }

        // Get parent directory and prefix to match
        let (parent, prefix) = if input.ends_with('/') {
            (PathBuf::from(&input), String::new())
        } else if let Some(parent) = path.parent() {
            let prefix = path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), prefix)
        } else {
            return;
        };

        // Read directory and find matches
        if let Ok(entries) = fs::read_dir(&parent) {
            let mut matches: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name.starts_with(&prefix) && !name.starts_with('.'))
                .collect();
            
            matches.sort();
            
            if matches.len() == 1 {
                // Single match - complete it
                let completed = parent.join(&matches[0]);
                self.workspace_path_input = format!("{}/", completed.to_string_lossy());
            } else if matches.len() > 1 {
                // Multiple matches - complete common prefix
                if let Some(common) = common_prefix(&matches) {
                    if common.len() > prefix.len() {
                        let completed = parent.join(&common);
                        self.workspace_path_input = completed.to_string_lossy().to_string();
                    }
                }
            }
        }
    }
    
    pub fn load_workspace(&mut self) {
        let path = self.workspace_path_input.clone();
        
        // Expand ~ to home directory
        let expanded = if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                path.replacen("~", &home.to_string_lossy(), 1)
            } else {
                path.clone()
            }
        } else {
            path.clone()
        };
        let path_buf = PathBuf::from(&expanded);
        
        // Try to find and parse OpenAPI spec first
        if let Some(spec_path) = detector::find_openapi_spec(&path_buf) {
            match openapi::parse_openapi(&spec_path) {
                Ok(project) => {
                    let count = project.endpoints.len();
                    self.workspace = Some(project);
                    self.selected_endpoint = 0;
                    self.response.body = format!("✓ Loaded {} endpoints from OpenAPI spec", count);
                    self.show_workspace_input = false;
                    self.workspace_path_input.clear();
                    return;
                }
                Err(e) => {
                    self.response.body = format!("Error parsing OpenAPI: {}", e);
                }
            }
        }
        
        // Fallback to source code parsing based on detected framework
        let framework = detector::detect_framework(&path_buf);
        
        let project = match framework {
            discovery::Framework::FastAPI | discovery::Framework::Flask => {
                Some(discovery::load_python_project(&path_buf, framework))
            }
            discovery::Framework::Express => {
                Some(discovery::load_express_project(&path_buf))
            }
            _ => None,
        };
        
        if let Some(proj) = project {
            let count = proj.endpoints.len();
            let fw_name = proj.framework.as_str().to_string();
            self.workspace = Some(proj);
            self.selected_endpoint = 0;
            self.response.body = format!("✓ Loaded {} endpoints from {} source code", count, fw_name);
        } else {
            self.response.body = format!(
                "No supported framework detected in {}\n\nSupported: OpenAPI, FastAPI, Flask, Express.js",
                expanded
            );
        }
        
        self.show_workspace_input = false;
        self.workspace_path_input.clear();
    }
    
    pub fn next_endpoint(&mut self) {
        if let Some(ws) = &self.workspace {
            if !ws.endpoints.is_empty() {
                self.selected_endpoint = (self.selected_endpoint + 1) % ws.endpoints.len();
            }
        }
    }
    
    pub fn prev_endpoint(&mut self) {
        if let Some(ws) = &self.workspace {
            if !ws.endpoints.is_empty() {
                self.selected_endpoint = self.selected_endpoint
                    .checked_sub(1)
                    .unwrap_or(ws.endpoints.len() - 1);
            }
        }
    }
    
    pub fn select_endpoint(&mut self) {
        // Clone endpoint to avoid borrow conflict
        let endpoint_opt = self.workspace.as_ref()
            .and_then(|ws| ws.endpoints.get(self.selected_endpoint).cloned());
        
        if let Some(endpoint) = endpoint_opt {
            self.load_endpoint(&endpoint);
            self.active_panel = Panel::Url;
        }
    }
    
    fn load_endpoint(&mut self, endpoint: &DiscoveredEndpoint) {
        // Set method
        self.request.method = match endpoint.method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        };

        // Set URL (combine base URL with path)
        let base = self.workspace.as_ref()
            .and_then(|w| w.base_url.clone())
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        self.request.url = format!("{}{}", base.trim_end_matches('/'), endpoint.path);
        self.cursor_position = self.request.url.len();

        // Set auth
        self.request.auth = match &endpoint.auth {
            discovery::AuthRequirement::Bearer => AuthType::Bearer(String::new()),
            discovery::AuthRequirement::Basic => AuthType::Basic { 
                username: String::new(), 
                password: String::new() 
            },
            _ => AuthType::None,
        };

        // Set body example if available
        if let Some(body) = &endpoint.body {
            if let Some(example) = &body.example {
                self.request.body = example.clone();
            }
        }

        // Clear previous response
        self.response.body = format!("Loaded: {} {}\n\nAuth: {}", 
            endpoint.method, endpoint.path, endpoint.auth.as_str());
        self.response.status_code = None;
    }
    
    // ========================
    // Request sending
    // ========================
    
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
    
    /// Prepare a streaming request (for large responses with incremental updates)
    pub fn prepare_streaming_request(&mut self) -> Option<NetworkCommand> {
        if self.is_loading {
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
        if let Some(id) = self.pending_request_id {
            Some(NetworkCommand::CancelRequest(id))
        } else {
            None
        }
    }
    
    // ========================
    // Response handling
    // ========================
    
    pub fn handle_response(&mut self, response: NetworkResponse) {
        // Only process if it matches the pending request (for HTTP responses)
        let response_id = response.id();
        let is_for_pending = self.pending_request_id == Some(response_id);
        
        match response {
            NetworkResponse::Success { status, body, time_ms, .. } => {
                if is_for_pending {
                    self.response.status_code = Some(status);
                    self.response.body = body;
                    self.response.time_ms = time_ms;
                    self.finalize_request();
                }
            }
            NetworkResponse::StreamChunk { chunk, bytes_received, .. } => {
                if is_for_pending {
                    // Append chunk to streaming body
                    self.streaming_body.push_str(&chunk);
                    self.bytes_received = bytes_received;
                    // Show streaming progress
                    self.response.body = format!(
                        "Streaming... {} bytes received\n\n{}",
                        bytes_received,
                        &self.streaming_body
                    );
                }
            }
            NetworkResponse::StreamComplete { status, total_bytes, time_ms, .. } => {
                if is_for_pending {
                    // Format final body as JSON if possible
                    let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.streaming_body) {
                        serde_json::to_string_pretty(&json).unwrap_or_else(|_| self.streaming_body.clone())
                    } else {
                        self.streaming_body.clone()
                    };
                    
                    self.response.status_code = Some(status);
                    self.response.body = formatted;
                    self.response.time_ms = time_ms;
                    self.bytes_received = total_bytes;
                    self.finalize_request();
                }
            }
            NetworkResponse::Error { message, time_ms, .. } => {
                if is_for_pending {
                    self.response.status_code = None;
                    self.response.body = message;
                    self.response.time_ms = time_ms;
                    self.finalize_request();
                }
            }
            NetworkResponse::Cancelled { .. } => {
                if is_for_pending {
                    self.response.status_code = None;
                    self.response.body = String::from("Request cancelled");
                    self.response.time_ms = 0;
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
    
    pub fn switch_tab(&mut self, tab: AppTab) {
        self.active_tab = tab;
        self.input_mode = InputMode::Normal;
    }
    
    // ========================
    // WebSocket commands
    // ========================
    
    pub fn ws_connect(&mut self) -> Option<NetworkCommand> {
        if self.ws.connected {
            return None;
        }
        
        let id = self.next_id();
        self.ws.connection_id = Some(id);
        
        // Add system message
        self.ws.messages.push(WsLogEntry {
            direction: WsDirection::System,
            content: format!("Connecting to {}...", self.ws.url),
            timestamp: chrono::Utc::now(),
        });
        
        Some(NetworkCommand::ConnectWebSocket {
            id,
            url: self.ws.url.clone(),
        })
    }
    
    pub fn ws_disconnect(&mut self) -> Option<NetworkCommand> {
        if let Some(id) = self.ws.connection_id {
            self.ws.messages.push(WsLogEntry {
                direction: WsDirection::System,
                content: "Disconnecting...".to_string(),
                timestamp: chrono::Utc::now(),
            });
            Some(NetworkCommand::CloseWebSocket(id))
        } else {
            None
        }
    }
    
    pub fn ws_send(&mut self) -> Option<NetworkCommand> {
        if !self.ws.connected || self.ws.input.is_empty() {
            return None;
        }
        
        if let Some(id) = self.ws.connection_id {
            let message = self.ws.input.clone();
            
            // Add to log
            self.ws.messages.push(WsLogEntry {
                direction: WsDirection::Sent,
                content: message.clone(),
                timestamp: chrono::Utc::now(),
            });
            
            // Clear input
            self.ws.input.clear();
            self.ws.cursor_position = 0;
            
            Some(NetworkCommand::SendWebSocketMessage { id, message })
        } else {
            None
        }
    }
    
    pub fn ws_char(&mut self, c: char) {
        if self.ws.editing_url {
            self.ws.url.insert(self.ws.url_cursor, c);
            self.ws.url_cursor += 1;
        } else {
            self.ws.input.insert(self.ws.cursor_position, c);
            self.ws.cursor_position += 1;
        }
    }
    
    pub fn ws_backspace(&mut self) {
        if self.ws.editing_url {
            if self.ws.url_cursor > 0 {
                self.ws.url_cursor -= 1;
                self.ws.url.remove(self.ws.url_cursor);
            }
        } else {
            if self.ws.cursor_position > 0 {
                self.ws.cursor_position -= 1;
                self.ws.input.remove(self.ws.cursor_position);
            }
        }
    }
    
    pub fn ws_cursor_left(&mut self) {
        if self.ws.editing_url {
            if self.ws.url_cursor > 0 {
                self.ws.url_cursor -= 1;
            }
        } else {
            if self.ws.cursor_position > 0 {
                self.ws.cursor_position -= 1;
            }
        }
    }
    
    pub fn ws_cursor_right(&mut self) {
        if self.ws.editing_url {
            if self.ws.url_cursor < self.ws.url.len() {
                self.ws.url_cursor += 1;
            }
        } else {
            if self.ws.cursor_position < self.ws.input.len() {
                self.ws.cursor_position += 1;
            }
        }
    }
    
    /// Start editing WS URL
    pub fn ws_start_url_edit(&mut self) {
        self.ws.editing_url = true;
        self.ws.url_cursor = self.ws.url.len();
        self.input_mode = InputMode::Editing;
    }
    
    /// Start editing WS message input
    pub fn ws_start_input_edit(&mut self) {
        self.ws.editing_url = false;
        self.input_mode = InputMode::Editing;
    }
    
    /// Finalize a completed request (add to history, reset state)
    fn finalize_request(&mut self) {
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

/// Find common prefix among strings
fn common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }
    let first = &strings[0];
    let mut prefix_len = first.len();
    
    for s in &strings[1..] {
        prefix_len = first.chars()
            .zip(s.chars())
            .take_while(|(a, b)| a == b)
            .count()
            .min(prefix_len);
    }
    
    if prefix_len > 0 {
        Some(first[..prefix_len].to_string())
    } else {
        None
    }
}
