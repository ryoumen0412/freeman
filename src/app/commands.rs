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
        self.active_panel = self.active_panel.next();
    }

    /// Move focus to the previous panel in the tab order.
    pub fn prev_panel(&mut self) {
        self.active_panel = self.active_panel.prev();
    }

    /// Focus the workspace panel directly.
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
        self.cursor_position = crate::app::text_utils::prev_char_boundary(input, self.cursor_position);
    }

    pub fn move_cursor_right(&mut self) {
        let input = self.current_input();
        self.cursor_position = crate::app::text_utils::next_char_boundary(input, self.cursor_position);
    }

    pub fn enter_char(&mut self, c: char) {
        let cursor_pos = self.cursor_position;
        let input = self.current_input_mut();
        self.cursor_position = crate::app::text_utils::insert_char(input, cursor_pos, c);
    }

    pub fn delete_char(&mut self) {
        let cursor_pos = self.cursor_position;
        let input = self.current_input_mut();
        self.cursor_position = crate::app::text_utils::delete_char_before(input, cursor_pos);
    }

    // ========================

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn close_help(&mut self) {
        self.show_help = false;
    }

    pub fn switch_tab(&mut self, tab: AppTab) {
        self.active_tab = tab;
        self.input_mode = InputMode::Normal;
    }
}
