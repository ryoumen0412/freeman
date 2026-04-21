//! Main draw dispatcher - routes rendering to the active tab.

use ratatui::prelude::*;

use crate::messages::ui_events::AppTab;
use crate::messages::RenderState;
use crate::tui::{chrome, gql_tab, http_tab, popups, ws_tab};

/// Root drawing function. Reads [`RenderState`] and renders all UI elements.
///
/// This is a pure rendering function — it does not mutate any state.
pub fn draw_ui(f: &mut Frame, state: &RenderState) {
    let area = f.area();

    // Main layout with tab bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    // Draw tab bar
    chrome::draw_tab_bar(f, state, main_chunks[0]);

    // Draw content based on active tab
    match state.active_tab {
        AppTab::Http => http_tab::draw_http_tab(f, state, main_chunks[1]),
        AppTab::WebSocket => ws_tab::draw_ws_tab(f, state, main_chunks[1]),
        AppTab::GraphQL => gql_tab::draw_gql_tab(f, state, main_chunks[1]),
    }

    // Status bar
    chrome::draw_status_bar(f, state, main_chunks[2]);

    // Popups (rendered on top)
    if state.show_help {
        popups::draw_help_popup(f, area);
    }

    if state.http.show_curl_import {
        popups::draw_curl_import_popup(f, state, area);
    }

    if state.http.show_workspace_input {
        popups::draw_workspace_input_popup(f, state, area);
    }
}
