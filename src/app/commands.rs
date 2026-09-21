//! Command handlers - business logic for processing UI events
//!
//! This module contains all the command handlers for [`AppState`].
//! Methods are organized by feature area:
//! - **Navigation**: Panel switching and focus
//! - **Input**: Text editing and cursor movement
//! - **HTTP**: Method cycling, headers, auth
//! - **Request**: Preparation, validation, and response handling
//! - **Workspace**: Project discovery and endpoint loading
//! - **WebSocket**: Connection and message handling

use crate::app::AppState;
use crate::messages::ui_events::{AppTab, InputMode, Panel};

impl AppState {
    // ========================
    // Navigation
    // ========================

    /// Move focus to the next panel in the tab order.
    /// Panels cycle: Url → Body → Headers → Auth → Workspace → Url
    pub fn next_panel(&mut self) {
        self.navigation.active_panel = self.navigation.active_panel.next();
    }

    /// Move focus to the previous panel in the tab order.
    pub fn prev_panel(&mut self) {
        self.navigation.active_panel = self.navigation.active_panel.prev();
    }

    /// Focus the workspace panel directly.
    pub fn focus_workspace(&mut self) {
        self.navigation.active_panel = Panel::Workspace;
    }

    // ========================
    // Input editing
    // ========================

    pub fn start_editing(&mut self) {
        self.ui.input_mode = InputMode::Editing;
        self.ui.cursor_position = self.current_input().len();
    }

    pub fn stop_editing(&mut self) {
        self.ui.input_mode = InputMode::Normal;
    }

    pub fn move_cursor_left(&mut self) {
        let input = self.current_input();
        self.ui.cursor_position =
            crate::app::text_utils::prev_char_boundary(input, self.ui.cursor_position);
    }

    pub fn move_cursor_right(&mut self) {
        let input = self.current_input();
        self.ui.cursor_position =
            crate::app::text_utils::next_char_boundary(input, self.ui.cursor_position);
    }

    pub fn enter_char(&mut self, c: char) {
        let cursor_pos = self.ui.cursor_position;
        let input = self.current_input_mut();
        self.ui.cursor_position = crate::app::text_utils::insert_char(input, cursor_pos, c);
    }

    pub fn delete_char(&mut self) {
        let cursor_pos = self.ui.cursor_position;
        let input = self.current_input_mut();
        self.ui.cursor_position = crate::app::text_utils::delete_char_before(input, cursor_pos);
    }

    // ========================

    pub fn toggle_help(&mut self) {
        self.ui.show_help = !self.ui.show_help;
    }

    pub fn close_help(&mut self) {
        self.ui.show_help = false;
    }

    pub fn switch_tab(&mut self, tab: AppTab) {
        self.navigation.active_tab = tab;
        self.ui.input_mode = InputMode::Normal;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::messages::ui_events::{AppTab, InputMode, Panel};

    fn make_state() -> AppState {
        AppState::new()
    }

    // ── Panel navigation ─────────────────────────────────────────────────────

    #[test]
    fn test_next_panel_cycles_forward() {
        let mut state = make_state();
        assert_eq!(state.navigation.active_panel, Panel::Url);
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Body);
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Headers);
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Auth);
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Response);
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Workspace);
        // Full wrap-around
        state.next_panel();
        assert_eq!(state.navigation.active_panel, Panel::Url);
    }

    #[test]
    fn test_prev_panel_cycles_backward() {
        let mut state = make_state();
        // From Url, prev should wrap to Workspace
        state.prev_panel();
        assert_eq!(state.navigation.active_panel, Panel::Workspace);
        state.prev_panel();
        assert_eq!(state.navigation.active_panel, Panel::Response);
        state.prev_panel();
        assert_eq!(state.navigation.active_panel, Panel::Auth);
    }

    #[test]
    fn test_focus_workspace() {
        let mut state = make_state();
        state.focus_workspace();
        assert_eq!(state.navigation.active_panel, Panel::Workspace);
    }

    // ── Input mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_start_editing_sets_mode_and_cursor() {
        let mut state = make_state();
        let url_len = state.http.request.url.len();
        state.start_editing();
        assert_eq!(state.ui.input_mode, InputMode::Editing);
        // cursor should be at the end of the current input
        assert_eq!(state.ui.cursor_position, url_len);
    }

    #[test]
    fn test_stop_editing_resets_mode() {
        let mut state = make_state();
        state.start_editing();
        state.stop_editing();
        assert_eq!(state.ui.input_mode, InputMode::Normal);
    }

    // ── Cursor movement ──────────────────────────────────────────────────────

    #[test]
    fn test_move_cursor_left_at_start_does_not_underflow() {
        let mut state = make_state();
        state.ui.cursor_position = 0;
        state.move_cursor_left();
        assert_eq!(state.ui.cursor_position, 0);
    }

    #[test]
    fn test_move_cursor_right_at_end_does_not_overflow() {
        let mut state = make_state();
        let end = state.http.request.url.len();
        state.ui.cursor_position = end;
        state.move_cursor_right();
        // Should not advance beyond the string length
        assert_eq!(state.ui.cursor_position, end);
    }

    #[test]
    fn test_enter_char_inserts_and_advances_cursor() {
        let mut state = make_state();
        // Put cursor at end of URL
        state.ui.cursor_position = state.http.request.url.len();
        let original_len = state.http.request.url.len();
        state.enter_char('!');
        // URL grows by 1 ASCII char
        assert_eq!(state.http.request.url.len(), original_len + 1);
        assert_eq!(state.ui.cursor_position, original_len + 1);
        assert!(state.http.request.url.ends_with('!'));
    }

    #[test]
    fn test_delete_char_removes_and_moves_cursor_back() {
        let mut state = make_state();
        state.ui.cursor_position = state.http.request.url.len();
        let original_len = state.http.request.url.len();
        state.delete_char();
        assert_eq!(state.http.request.url.len(), original_len - 1);
        assert_eq!(state.ui.cursor_position, original_len - 1);
    }

    // ── Help popup ───────────────────────────────────────────────────────────

    #[test]
    fn test_toggle_help() {
        let mut state = make_state();
        assert!(!state.ui.show_help);
        state.toggle_help();
        assert!(state.ui.show_help);
        state.toggle_help();
        assert!(!state.ui.show_help);
    }

    #[test]
    fn test_close_help_always_false() {
        let mut state = make_state();
        state.ui.show_help = true;
        state.close_help();
        assert!(!state.ui.show_help);
        // Idempotent
        state.close_help();
        assert!(!state.ui.show_help);
    }

    // ── Tab switching ─────────────────────────────────────────────────────────

    #[test]
    fn test_switch_tab_changes_tab_and_resets_input_mode() {
        let mut state = make_state();
        state.ui.input_mode = InputMode::Editing;
        state.switch_tab(AppTab::WebSocket);
        assert_eq!(state.navigation.active_tab, AppTab::WebSocket);
        assert_eq!(state.ui.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_switch_tab_graphql() {
        let mut state = make_state();
        state.switch_tab(AppTab::GraphQL);
        assert_eq!(state.navigation.active_tab, AppTab::GraphQL);
    }
}
