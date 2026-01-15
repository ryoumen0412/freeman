//! Render state - data structure sent from App layer to UI for rendering

use crate::models::{AuthType, Header, HttpMethod, Response};
use crate::discovery::WorkspaceProject;
use crate::messages::ui_events::{AppTab, AuthField, InputMode, Panel};
use crate::app::state::WsLogEntry;

/// Complete state needed by the UI to render
#[derive(Debug, Clone)]
pub struct RenderState {
    // Tab
    pub active_tab: AppTab,
    
    // HTTP Request data
    pub method: HttpMethod,
    pub url: String,
    pub body: String,
    pub headers: Vec<Header>,
    pub auth: AuthType,
    
    // UI state
    pub active_panel: Panel,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    
    // HTTP Response
    pub response: Response,
    pub response_scroll: u16,
    pub is_loading: bool,
    
    // Headers panel
    pub selected_header: usize,
    
    // Auth panel
    #[allow(dead_code)]
    pub auth_field: AuthField,
    
    // History
    pub history_index: Option<usize>,
    
    // Workspace
    pub workspace: Option<WorkspaceProject>,
    pub workspace_path_input: String,
    pub selected_endpoint: usize,
    
    // Popups
    pub show_help: bool,
    pub show_curl_import: bool,
    pub curl_import_buffer: String,
    pub show_workspace_input: bool,
    
    // WebSocket
    pub ws_url: String,
    pub ws_url_cursor: usize,
    pub ws_editing_url: bool,
    pub ws_connected: bool,
    pub ws_messages: Vec<WsLogEntry>,
    pub ws_input: String,
    pub ws_input_cursor: usize,
    pub ws_scroll: u16,
}

impl Default for RenderState {
    fn default() -> Self {
        RenderState {
            active_tab: AppTab::Http,
            method: HttpMethod::GET,
            url: String::from("https://httpbin.org/get"),
            body: String::new(),
            headers: vec![
                Header::new("Content-Type", "application/json"),
                Header::new("Accept", "application/json"),
            ],
            auth: AuthType::None,
            active_panel: Panel::Url,
            input_mode: InputMode::Normal,
            cursor_position: 24,
            response: Response::default(),
            response_scroll: 0,
            is_loading: false,
            selected_header: 0,
            auth_field: AuthField::Token,
            history_index: None,
            workspace: None,
            workspace_path_input: String::new(),
            selected_endpoint: 0,
            show_help: false,
            show_curl_import: false,
            curl_import_buffer: String::new(),
            show_workspace_input: false,
            ws_url: String::from("wss://echo.websocket.org"),
            ws_url_cursor: 0,
            ws_editing_url: false,
            ws_connected: false,
            ws_messages: Vec::new(),
            ws_input: String::new(),
            ws_input_cursor: 0,
            ws_scroll: 0,
        }
    }
}
