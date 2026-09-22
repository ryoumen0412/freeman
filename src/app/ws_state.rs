//! WebSocket domain logic — methods that operate purely on WebSocketState.

use crate::app::state::{WebSocketState, WsDirection, WsLogEntry};
use crate::messages::NetworkCommand;

impl WebSocketState {
    // ========================
    // Connection lifecycle
    // ========================

    /// Prepare a connection command. Caller provides the `id`.
    pub fn connect(&mut self, id: u64) -> Option<NetworkCommand> {
        if self.connected {
            return None;
        }

        self.connection_id = Some(id);
        self.messages.push(WsLogEntry {
            direction: WsDirection::System,
            content: format!("Connecting to {}...", self.url),
            timestamp: chrono::Utc::now(),
        });

        Some(NetworkCommand::ConnectWebSocket {
            id,
            url: self.url.clone(),
        })
    }

    pub fn disconnect(&mut self) -> Option<NetworkCommand> {
        if let Some(id) = self.connection_id {
            self.messages.push(WsLogEntry {
                direction: WsDirection::System,
                content: "Disconnecting...".to_string(),
                timestamp: chrono::Utc::now(),
            });
            Some(NetworkCommand::CloseWebSocket(id))
        } else {
            None
        }
    }

    pub fn send(&mut self) -> Option<NetworkCommand> {
        if !self.connected || self.input.is_empty() {
            return None;
        }

        if let Some(id) = self.connection_id {
            let message = self.input.clone();
            self.messages.push(WsLogEntry {
                direction: WsDirection::Sent,
                content: message.clone(),
                timestamp: chrono::Utc::now(),
            });
            self.input.clear();
            self.cursor_position = 0;
            Some(NetworkCommand::SendWebSocketMessage { id, message })
        } else {
            None
        }
    }

    // ========================
    // Text editing
    // ========================

    pub fn char_input(&mut self, c: char) {
        if self.editing_url {
            let cursor = self.url_cursor;
            self.url_cursor = crate::app::text_utils::insert_char(&mut self.url, cursor, c);
        } else {
            let cursor = self.cursor_position;
            self.cursor_position = crate::app::text_utils::insert_char(&mut self.input, cursor, c);
        }
    }

    pub fn backspace(&mut self) {
        if self.editing_url {
            let cursor = self.url_cursor;
            self.url_cursor = crate::app::text_utils::delete_char_before(&mut self.url, cursor);
        } else {
            let cursor = self.cursor_position;
            self.cursor_position =
                crate::app::text_utils::delete_char_before(&mut self.input, cursor);
        }
    }

    pub fn cursor_left(&mut self) {
        if self.editing_url {
            self.url_cursor =
                crate::app::text_utils::prev_char_boundary(&self.url, self.url_cursor);
        } else {
            self.cursor_position =
                crate::app::text_utils::prev_char_boundary(&self.input, self.cursor_position);
        }
    }

    pub fn cursor_right(&mut self) {
        if self.editing_url {
            self.url_cursor =
                crate::app::text_utils::next_char_boundary(&self.url, self.url_cursor);
        } else {
            self.cursor_position =
                crate::app::text_utils::next_char_boundary(&self.input, self.cursor_position);
        }
    }

    /// Start editing WS URL. Returns true (caller should set input_mode).
    pub fn start_url_edit(&mut self) {
        self.editing_url = true;
        self.url_cursor = self.url.len();
    }

    /// Start editing WS message input. Returns true (caller should set input_mode).
    pub fn start_input_edit(&mut self) {
        self.editing_url = false;
    }

    // ========================
    // Response handling
    // ========================

    pub fn on_connected(&mut self, id: u64) {
        if self.connection_id == Some(id) {
            self.connected = true;
            self.messages.push(WsLogEntry {
                direction: WsDirection::System,
                content: "Connected!".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }
    }

    pub fn on_message(&mut self, id: u64, message: String) {
        if self.connection_id == Some(id) {
            self.messages.push(WsLogEntry {
                direction: WsDirection::Received,
                content: message,
                timestamp: chrono::Utc::now(),
            });
        }
    }

    pub fn on_closed(&mut self, id: u64) {
        if self.connection_id == Some(id) {
            self.connected = false;
            self.connection_id = None;
            self.messages.push(WsLogEntry {
                direction: WsDirection::System,
                content: "Connection closed".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }
    }

    pub fn on_error(&mut self, id: u64, error: String) {
        if self.connection_id == Some(id) {
            self.connected = false;
            self.connection_id = None;
            self.messages.push(WsLogEntry {
                direction: WsDirection::System,
                content: format!("Error: {}", error),
                timestamp: chrono::Utc::now(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::{WebSocketState, WsDirection};
    use crate::messages::NetworkCommand;

    fn make_ws() -> WebSocketState {
        WebSocketState::default()
    }

    // ── Connection ───────────────────────────────────────────────────────────

    #[test]
    fn test_connect_when_disconnected_returns_command() {
        let mut ws = make_ws();
        let cmd = ws.connect(1);
        assert!(matches!(cmd, Some(NetworkCommand::ConnectWebSocket { .. })));
        assert_eq!(ws.connection_id, Some(1));
        assert!(!ws.messages.is_empty());
    }

    #[test]
    fn test_connect_blocked_when_already_connected() {
        let mut ws = make_ws();
        ws.connected = true;
        let cmd = ws.connect(1);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_disconnect_with_connection_id_returns_command() {
        let mut ws = make_ws();
        ws.connection_id = Some(42);
        let cmd = ws.disconnect();
        assert!(matches!(cmd, Some(NetworkCommand::CloseWebSocket(42))));
    }

    #[test]
    fn test_disconnect_without_connection_id_returns_none() {
        let mut ws = make_ws();
        let cmd = ws.disconnect();
        assert!(cmd.is_none());
    }

    // ── Send ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_send_connected_with_input_returns_command() {
        let mut ws = make_ws();
        ws.connected = true;
        ws.connection_id = Some(1);
        ws.input = "hello".to_string();
        let cmd = ws.send();
        assert!(matches!(
            cmd,
            Some(NetworkCommand::SendWebSocketMessage { .. })
        ));
        assert!(ws.input.is_empty());
    }

    #[test]
    fn test_send_blocked_when_disconnected() {
        let mut ws = make_ws();
        ws.input = "hello".to_string();
        let cmd = ws.send();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_send_blocked_when_empty_input() {
        let mut ws = make_ws();
        ws.connected = true;
        ws.connection_id = Some(1);
        let cmd = ws.send();
        assert!(cmd.is_none());
    }

    // ── Text editing ─────────────────────────────────────────────────────────

    #[test]
    fn test_char_in_url_editing_mode_modifies_url() {
        let mut ws = make_ws();
        ws.editing_url = true;
        ws.url_cursor = ws.url.len();
        let orig_len = ws.url.len();
        ws.char_input('!');
        assert_eq!(ws.url.len(), orig_len + 1);
        assert!(ws.url.ends_with('!'));
    }

    #[test]
    fn test_char_in_message_mode_modifies_input() {
        let mut ws = make_ws();
        ws.editing_url = false;
        ws.cursor_position = 0;
        ws.char_input('H');
        assert_eq!(ws.input, "H");
    }

    #[test]
    fn test_backspace_in_url_mode_removes_last_char() {
        let mut ws = make_ws();
        ws.editing_url = true;
        ws.url = "ws://localhost:8080".to_string();
        ws.url_cursor = ws.url.len();
        ws.backspace();
        assert_eq!(ws.url, "ws://localhost:808");
    }

    #[test]
    fn test_backspace_in_message_mode_removes_from_input() {
        let mut ws = make_ws();
        ws.editing_url = false;
        ws.input = "hi".to_string();
        ws.cursor_position = 2;
        ws.backspace();
        assert_eq!(ws.input, "h");
    }

    // ── URL/Input edit start ─────────────────────────────────────────────────

    #[test]
    fn test_start_url_edit_sets_editing_url_flag() {
        let mut ws = make_ws();
        ws.start_url_edit();
        assert!(ws.editing_url);
        assert_eq!(ws.url_cursor, ws.url.len());
    }

    #[test]
    fn test_start_input_edit_clears_url_editing_flag() {
        let mut ws = make_ws();
        ws.editing_url = true;
        ws.start_input_edit();
        assert!(!ws.editing_url);
    }

    // ── Response handlers ────────────────────────────────────────────────────

    #[test]
    fn test_on_connected_sets_connected_flag() {
        let mut ws = make_ws();
        ws.connection_id = Some(1);
        ws.on_connected(1);
        assert!(ws.connected);
    }

    #[test]
    fn test_on_connected_wrong_id_ignored() {
        let mut ws = make_ws();
        ws.connection_id = Some(1);
        ws.on_connected(99);
        assert!(!ws.connected);
    }

    #[test]
    fn test_on_message_adds_to_log() {
        let mut ws = make_ws();
        ws.connection_id = Some(1);
        let before = ws.messages.len();
        ws.on_message(1, "hello".to_string());
        assert_eq!(ws.messages.len(), before + 1);
        assert!(matches!(
            ws.messages.last().unwrap().direction,
            WsDirection::Received
        ));
    }

    #[test]
    fn test_on_closed_resets_connection() {
        let mut ws = make_ws();
        ws.connection_id = Some(1);
        ws.connected = true;
        ws.on_closed(1);
        assert!(!ws.connected);
        assert!(ws.connection_id.is_none());
    }

    #[test]
    fn test_on_error_resets_connection() {
        let mut ws = make_ws();
        ws.connection_id = Some(1);
        ws.connected = true;
        ws.on_error(1, "timeout".to_string());
        assert!(!ws.connected);
        assert!(ws.connection_id.is_none());
    }
}
