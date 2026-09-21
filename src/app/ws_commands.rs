use crate::app::state::{WsDirection, WsLogEntry};
use crate::app::AppState;
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
            let cursor = self.ws.url_cursor;
            self.ws.url_cursor = crate::app::text_utils::insert_char(&mut self.ws.url, cursor, c);
        } else {
            let cursor = self.ws.cursor_position;
            self.ws.cursor_position =
                crate::app::text_utils::insert_char(&mut self.ws.input, cursor, c);
        }
    }

    pub fn ws_backspace(&mut self) {
        if self.ws.editing_url {
            let cursor = self.ws.url_cursor;
            self.ws.url_cursor =
                crate::app::text_utils::delete_char_before(&mut self.ws.url, cursor);
        } else {
            let cursor = self.ws.cursor_position;
            self.ws.cursor_position =
                crate::app::text_utils::delete_char_before(&mut self.ws.input, cursor);
        }
    }

    pub fn ws_cursor_left(&mut self) {
        if self.ws.editing_url {
            self.ws.url_cursor =
                crate::app::text_utils::prev_char_boundary(&self.ws.url, self.ws.url_cursor);
        } else {
            self.ws.cursor_position =
                crate::app::text_utils::prev_char_boundary(&self.ws.input, self.ws.cursor_position);
        }
    }

    pub fn ws_cursor_right(&mut self) {
        if self.ws.editing_url {
            self.ws.url_cursor =
                crate::app::text_utils::next_char_boundary(&self.ws.url, self.ws.url_cursor);
        } else {
            self.ws.cursor_position =
                crate::app::text_utils::next_char_boundary(&self.ws.input, self.ws.cursor_position);
        }
    }

    /// Start editing WS URL
    pub fn ws_start_url_edit(&mut self) {
        self.ws.editing_url = true;
        self.ws.url_cursor = self.ws.url.len();
        self.ui.input_mode = InputMode::Editing;
    }

    /// Start editing WS message input
    pub fn ws_start_input_edit(&mut self) {
        self.ws.editing_url = false;
        self.ui.input_mode = InputMode::Editing;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::WsDirection;
    use crate::app::AppState;
    use crate::messages::ui_events::InputMode;
    use crate::messages::NetworkCommand;

    fn make_state() -> AppState {
        AppState::new()
    }

    // ── Connect ───────────────────────────────────────────────────────────────

    #[test]
    fn test_ws_connect_when_disconnected_returns_command() {
        let mut state = make_state();
        state.ws.connected = false;
        state.ws.url = "ws://localhost:8080/ws".to_string();

        let cmd = state.ws_connect();
        assert!(cmd.is_some());
        assert!(state.ws.connection_id.is_some());
        // A "Connecting..." system log entry should be added
        assert!(!state.ws.messages.is_empty());
        assert!(matches!(
            state.ws.messages[0].direction,
            WsDirection::System
        ));
    }

    #[test]
    fn test_ws_connect_blocked_when_already_connected() {
        let mut state = make_state();
        state.ws.connected = true;
        let cmd = state.ws_connect();
        assert!(cmd.is_none());
    }

    // ── Disconnect ────────────────────────────────────────────────────────────

    #[test]
    fn test_ws_disconnect_with_connection_id_returns_command() {
        let mut state = make_state();
        state.ws.connection_id = Some(42);

        let cmd = state.ws_disconnect();
        assert!(matches!(cmd, Some(NetworkCommand::CloseWebSocket(42))));
        // System log entry for disconnect
        assert!(!state.ws.messages.is_empty());
    }

    #[test]
    fn test_ws_disconnect_without_connection_id_returns_none() {
        let mut state = make_state();
        state.ws.connection_id = None;
        let cmd = state.ws_disconnect();
        assert!(cmd.is_none());
    }

    // ── Send ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_ws_send_connected_with_input_returns_command() {
        let mut state = make_state();
        state.ws.connected = true;
        state.ws.connection_id = Some(7);
        state.ws.input = "ping".to_string();
        state.ws.cursor_position = 4;

        let cmd = state.ws_send();
        assert!(matches!(
            cmd,
            Some(NetworkCommand::SendWebSocketMessage { id: 7, .. })
        ));
        // Input cleared after send
        assert!(state.ws.input.is_empty());
        assert_eq!(state.ws.cursor_position, 0);
        // Sent message logged
        assert!(!state.ws.messages.is_empty());
        assert!(matches!(state.ws.messages[0].direction, WsDirection::Sent));
    }

    #[test]
    fn test_ws_send_blocked_when_disconnected() {
        let mut state = make_state();
        state.ws.connected = false;
        state.ws.input = "hello".to_string();
        let cmd = state.ws_send();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_ws_send_blocked_when_empty_input() {
        let mut state = make_state();
        state.ws.connected = true;
        state.ws.connection_id = Some(1);
        state.ws.input = String::new();
        let cmd = state.ws_send();
        assert!(cmd.is_none());
    }

    // ── Character routing ─────────────────────────────────────────────────────

    #[test]
    fn test_ws_char_in_url_editing_mode_modifies_url() {
        let mut state = make_state();
        state.ws.editing_url = true;
        state.ws.url = "ws://".to_string();
        state.ws.url_cursor = state.ws.url.len();
        let original_len = state.ws.url.len();
        state.ws_char('x');
        assert_eq!(state.ws.url.len(), original_len + 1);
        assert!(state.ws.url.ends_with('x'));
        // input field untouched
        assert!(state.ws.input.is_empty());
    }

    #[test]
    fn test_ws_char_in_message_mode_modifies_input() {
        let mut state = make_state();
        state.ws.editing_url = false;
        state.ws.input = String::new();
        state.ws.cursor_position = 0;
        let original_url = state.ws.url.clone();
        state.ws_char('h');
        assert_eq!(state.ws.input, "h");
        // URL untouched
        assert_eq!(state.ws.url, original_url);
    }

    #[test]
    fn test_ws_backspace_in_url_mode_removes_last_char() {
        let mut state = make_state();
        state.ws.editing_url = true;
        state.ws.url = "ws://x".to_string();
        state.ws.url_cursor = state.ws.url.len();
        state.ws_backspace();
        assert_eq!(state.ws.url, "ws://");
    }

    #[test]
    fn test_ws_backspace_in_message_mode_removes_from_input() {
        let mut state = make_state();
        state.ws.editing_url = false;
        state.ws.input = "ab".to_string();
        state.ws.cursor_position = 2;
        state.ws_backspace();
        assert_eq!(state.ws.input, "a");
    }

    // ── Edit mode entry ───────────────────────────────────────────────────────

    #[test]
    fn test_ws_start_url_edit_sets_editing_url_and_mode() {
        let mut state = make_state();
        state.ws.url = "ws://localhost:8080".to_string();
        state.ws_start_url_edit();
        assert!(state.ws.editing_url);
        assert_eq!(state.ws.url_cursor, state.ws.url.len());
        assert_eq!(state.ui.input_mode, InputMode::Editing);
    }

    #[test]
    fn test_ws_start_input_edit_clears_url_editing_flag() {
        let mut state = make_state();
        state.ws.editing_url = true;
        state.ws_start_input_edit();
        assert!(!state.ws.editing_url);
        assert_eq!(state.ui.input_mode, InputMode::Editing);
    }
}
