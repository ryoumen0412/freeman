//! App state - pure data structure with no I/O logic

use crate::models::{AuthType, Request, Response};
use crate::storage::Storage;
use crate::discovery::WorkspaceProject;
use crate::messages::ui_events::{AppTab, AuthField, InputMode, Panel};
use crate::messages::RenderState;

/// Direction of WebSocket message
#[derive(Clone, Debug)]
pub enum WsDirection {
    Sent,
    Received,
    System,
}

/// A WebSocket log entry
#[derive(Clone, Debug)]
pub struct WsLogEntry {
    pub direction: WsDirection,
    pub content: String,
    #[allow(dead_code)]  // Reserved for future message timestamp display
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WebSocket connection state
#[derive(Clone, Debug)]
pub struct WebSocketState {
    pub url: String,
    pub url_cursor: usize,
    pub editing_url: bool,  // true = editing URL, false = editing message input
    pub connected: bool,
    pub connection_id: Option<u64>,
    pub messages: Vec<WsLogEntry>,
    pub input: String,
    pub cursor_position: usize,
    pub scroll: u16,
}

impl Default for WebSocketState {
    fn default() -> Self {
        use crate::constants::DEFAULT_WS_URL;
        WebSocketState {
            url: String::from(DEFAULT_WS_URL),
            url_cursor: 0,
            editing_url: false,
            connected: false,
            connection_id: None,
            messages: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            scroll: 0,
        }
    }
}

/// Main application state - pure data, no I/O
pub struct AppState {
    // Tab navigation
    pub active_tab: AppTab,
    
    // HTTP Request data
    pub request: Request,
    pub cursor_position: usize,
    
    // UI state
    pub active_panel: Panel,
    pub input_mode: InputMode,
    pub response_scroll: u16,
    
    // HTTP Response
    pub response: Response,
    pub is_loading: bool,
    pub next_request_id: u64,
    pub pending_request_id: Option<u64>,
    
    // Streaming state
    pub streaming_body: String,
    pub bytes_received: usize,
    
    // Headers panel
    pub selected_header: usize,
    
    // Auth panel
    pub auth_field: AuthField,
    
    // History
    pub history_index: Option<usize>,
    
    // Storage (persisted data)
    pub storage: Storage,
    
    // Workspace discovery
    pub workspace: Option<WorkspaceProject>,
    pub workspace_path_input: String,
    pub selected_endpoint: usize,
    
    // Popups
    pub show_help: bool,
    pub show_curl_import: bool,
    pub curl_import_buffer: String,
    pub show_workspace_input: bool,
    
    // WebSocket state (persists across tab switches)
    pub ws: WebSocketState,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            active_tab: AppTab::Http,
            request: Request::default(),
            cursor_position: 24, // Length of default URL
            active_panel: Panel::Url,
            input_mode: InputMode::Normal,
            response_scroll: 0,
            response: Response::default(),
            is_loading: false,
            next_request_id: 1,
            pending_request_id: None,
            streaming_body: String::new(),
            bytes_received: 0,
            selected_header: 0,
            auth_field: AuthField::Token,
            history_index: None,
            storage: Storage::new(),
            workspace: None,
            workspace_path_input: String::new(),
            selected_endpoint: 0,
            show_help: false,
            show_curl_import: false,
            curl_import_buffer: String::new(),
            show_workspace_input: false,
            ws: WebSocketState::default(),
        }
    }
    
    /// Generate a unique request ID
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }
    
    /// Get the current input field content
    pub fn current_input(&self) -> &str {
        match self.active_panel {
            Panel::Url => &self.request.url,
            Panel::Body => &self.request.body,
            Panel::Auth => match &self.request.auth {
                AuthType::Bearer(token) => token,
                AuthType::Basic { username, password } => {
                    match self.auth_field {
                        AuthField::Token => "",
                        AuthField::Username => username,
                        AuthField::Password => password,
                    }
                }
                AuthType::None => "",
            },
            _ => "",
        }
    }

    /// Get mutable reference to current input field
    pub fn current_input_mut(&mut self) -> &mut String {
        match self.active_panel {
            Panel::Url => &mut self.request.url,
            Panel::Body => &mut self.request.body,
            Panel::Auth => match &mut self.request.auth {
                AuthType::Bearer(token) => token,
                AuthType::Basic { username, password } => {
                    match self.auth_field {
                        AuthField::Token => &mut self.request.url, // fallback
                        AuthField::Username => username,
                        AuthField::Password => password,
                    }
                }
                AuthType::None => &mut self.request.url, // fallback
            },
            _ => &mut self.request.url, // fallback
        }
    }
    
    /// Convert state to RenderState for UI
    pub fn to_render_state(&self) -> RenderState {
        RenderState {
            active_tab: self.active_tab,
            method: self.request.method.clone(),
            url: self.request.url.clone(),
            body: self.request.body.clone(),
            headers: self.request.headers.clone(),
            auth: self.request.auth.clone(),
            active_panel: self.active_panel,
            input_mode: self.input_mode,
            cursor_position: self.cursor_position,
            response: self.response.clone(),
            response_scroll: self.response_scroll,
            is_loading: self.is_loading,
            selected_header: self.selected_header,
            auth_field: self.auth_field,
            history_index: self.history_index,
            workspace: self.workspace.clone(),
            workspace_path_input: self.workspace_path_input.clone(),
            selected_endpoint: self.selected_endpoint,
            show_help: self.show_help,
            show_curl_import: self.show_curl_import,
            curl_import_buffer: self.curl_import_buffer.clone(),
            show_workspace_input: self.show_workspace_input,
            ws_url: self.ws.url.clone(),
            ws_url_cursor: self.ws.url_cursor,
            ws_editing_url: self.ws.editing_url,
            ws_connected: self.ws.connected,
            ws_messages: self.ws.messages.clone(),
            ws_input: self.ws.input.clone(),
            ws_input_cursor: self.ws.cursor_position,
            ws_scroll: self.ws.scroll,
        }
    }
}
