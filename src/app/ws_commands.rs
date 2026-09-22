//! WebSocket orchestration tests.
//!
//! Pure WS domain logic lives in `ws_state.rs` (impl WebSocketState).
//! WS orchestration methods that touch multiple sub-states are in
//! `http_commands.rs` (ws_connect, ws_start_url_edit, ws_start_input_edit).
//!
//! This file retains integration tests that verify the full AppState flow.

#[cfg(test)]
mod tests {
    use crate::app::state::WsDirection;
    use crate::app::AppState;
    use crate::messages::ui_events::InputMode;
    use crate::messages::NetworkCommand;

    fn make_state() -> AppState {
        AppState::new()
    }

    // ── Connect (via AppState orchestration) ──────────────────────────────────

    #[test]
    fn test_ws_connect_when_disconnected_returns_command() {
        let mut state = make_state();
        state.ws.connected = false;
        state.ws.url = "ws://localhost:8080/ws".to_string();

        let cmd = state.ws_connect();
        assert!(cmd.is_some());
        assert!(state.ws.connection_id.is_some());
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

    // ── Disconnect (direct on sub-state) ─────────────────────────────────────

    #[test]
    fn test_ws_disconnect_with_connection_id_returns_command() {
        let mut state = make_state();
        state.ws.connection_id = Some(42);

        let cmd = state.ws.disconnect();
        assert!(matches!(cmd, Some(NetworkCommand::CloseWebSocket(42))));
        assert!(!state.ws.messages.is_empty());
    }

    #[test]
    fn test_ws_disconnect_without_connection_id_returns_none() {
        let mut state = make_state();
        state.ws.connection_id = None;
        let cmd = state.ws.disconnect();
        assert!(cmd.is_none());
    }

    // ── Send (direct on sub-state) ───────────────────────────────────────────

    #[test]
    fn test_ws_send_connected_with_input_returns_command() {
        let mut state = make_state();
        state.ws.connected = true;
        state.ws.connection_id = Some(7);
        state.ws.input = "ping".to_string();
        state.ws.cursor_position = 4;

        let cmd = state.ws.send();
        assert!(matches!(
            cmd,
            Some(NetworkCommand::SendWebSocketMessage { id: 7, .. })
        ));
        assert!(state.ws.input.is_empty());
        assert_eq!(state.ws.cursor_position, 0);
        assert!(!state.ws.messages.is_empty());
        assert!(matches!(state.ws.messages[0].direction, WsDirection::Sent));
    }

    #[test]
    fn test_ws_send_blocked_when_disconnected() {
        let mut state = make_state();
        state.ws.connected = false;
        state.ws.input = "hello".to_string();
        let cmd = state.ws.send();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_ws_send_blocked_when_empty_input() {
        let mut state = make_state();
        state.ws.connected = true;
        state.ws.connection_id = Some(1);
        state.ws.input = String::new();
        let cmd = state.ws.send();
        assert!(cmd.is_none());
    }

    // ── Character routing (direct on sub-state) ──────────────────────────────

    #[test]
    fn test_ws_char_in_url_editing_mode_modifies_url() {
        let mut state = make_state();
        state.ws.editing_url = true;
        state.ws.url = "ws://".to_string();
        state.ws.url_cursor = state.ws.url.len();
        let original_len = state.ws.url.len();
        state.ws.char_input('x');
        assert_eq!(state.ws.url.len(), original_len + 1);
        assert!(state.ws.url.ends_with('x'));
        assert!(state.ws.input.is_empty());
    }

    #[test]
    fn test_ws_char_in_message_mode_modifies_input() {
        let mut state = make_state();
        state.ws.editing_url = false;
        state.ws.input = String::new();
        state.ws.cursor_position = 0;
        let original_url = state.ws.url.clone();
        state.ws.char_input('h');
        assert_eq!(state.ws.input, "h");
        assert_eq!(state.ws.url, original_url);
    }

    #[test]
    fn test_ws_backspace_in_url_mode_removes_last_char() {
        let mut state = make_state();
        state.ws.editing_url = true;
        state.ws.url = "ws://x".to_string();
        state.ws.url_cursor = state.ws.url.len();
        state.ws.backspace();
        assert_eq!(state.ws.url, "ws://");
    }

    #[test]
    fn test_ws_backspace_in_message_mode_removes_from_input() {
        let mut state = make_state();
        state.ws.editing_url = false;
        state.ws.input = "ab".to_string();
        state.ws.cursor_position = 2;
        state.ws.backspace();
        assert_eq!(state.ws.input, "a");
    }

    // ── Edit mode entry (via AppState orchestration) ─────────────────────────

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
