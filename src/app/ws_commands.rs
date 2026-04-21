use crate::app::AppState;
use crate::app::state::{WsDirection, WsLogEntry};
use crate::messages::ui_events::InputMode;
use crate::messages::NetworkCommand;

impl AppState {
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
        } else if self.ws.cursor_position > 0 {
            self.ws.cursor_position -= 1;
            self.ws.input.remove(self.ws.cursor_position);
        }
    }

    pub fn ws_cursor_left(&mut self) {
        if self.ws.editing_url {
            if self.ws.url_cursor > 0 {
                self.ws.url_cursor -= 1;
            }
        } else if self.ws.cursor_position > 0 {
            self.ws.cursor_position -= 1;
        }
    }

    pub fn ws_cursor_right(&mut self) {
        if self.ws.editing_url {
            if self.ws.url_cursor < self.ws.url.len() {
                self.ws.url_cursor += 1;
            }
        } else if self.ws.cursor_position < self.ws.input.len() {
            self.ws.cursor_position += 1;
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

}
