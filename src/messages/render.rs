//! Render state - data structure sent from App layer to UI for rendering.
//!
//! Composed of domain-specific sub-structs for clean separation.

use crate::app::state::WsLogEntry;
use crate::discovery::WorkspaceProject;
use crate::messages::ui_events::{AppTab, AuthField, GqlField, InputMode, Panel};
use crate::models::{AuthType, Header, HttpMethod, Response};
use ratatui::text::Line;

/// Complete state needed by the UI to render.
///
/// Composed of domain sub-structs to avoid a flat 40+ field struct.
#[derive(Debug, Clone)]
pub struct RenderState {
    /// Active tab
    pub active_tab: AppTab,
    /// Current input mode (Normal / Editing)
    pub input_mode: InputMode,
    /// Whether help popup is shown
    pub show_help: bool,

    /// HTTP tab state
    pub http: HttpRenderState,
    /// WebSocket tab state
    pub ws: WsRenderState,
    /// GraphQL tab state
    pub gql: GqlRenderState,
}

/// HTTP tab render state.
#[derive(Debug, Clone)]
pub struct HttpRenderState {
    // Request
    pub method: HttpMethod,
    pub url: String,
    pub body: String,
    pub headers: Vec<Header>,
    pub auth: AuthType,
    pub ignore_ssl_errors: bool,

    // UI
    pub active_panel: Panel,
    pub cursor_position: usize,

    // Response
    pub response: Response,
    pub highlighted_response: Vec<Line<'static>>,
    pub response_scroll: u16,
    pub is_loading: bool,

    // Headers panel
    pub selected_header: usize,

    // Auth panel
    #[allow(dead_code)] // Reserved for cursor display in auth editing
    pub auth_field: AuthField,

    // History
    pub history_index: Option<usize>,

    // Workspace
    pub workspace: Option<WorkspaceProject>,
    pub workspace_path_input: String,
    pub selected_endpoint: usize,

    // Popups
    pub show_curl_import: bool,
    pub curl_import_buffer: String,
    pub show_workspace_input: bool,
}

/// WebSocket tab render state.
#[derive(Debug, Clone)]
pub struct WsRenderState {
    pub url: String,
    pub connected: bool,
    pub messages: Vec<WsLogEntry>,
    pub input: String,
    pub scroll: u16,
}

/// GraphQL tab render state.
#[derive(Debug, Clone)]
pub struct GqlRenderState {
    pub endpoint: String,
    pub query: String,
    pub variables: String,
    pub active_field: GqlField,
    pub response: String,
    pub response_scroll: u16,
    pub is_loading: bool,
    pub time_ms: u64,
}

// ============================================================================
// Defaults
// ============================================================================

impl Default for RenderState {
    fn default() -> Self {
        RenderState {
            active_tab: AppTab::Http,
            input_mode: InputMode::Normal,
            show_help: false,
            http: HttpRenderState::default(),
            ws: WsRenderState::default(),
            gql: GqlRenderState::default(),
        }
    }
}

impl Default for HttpRenderState {
    fn default() -> Self {
        use crate::constants::DEFAULT_HTTP_URL;
        HttpRenderState {
            method: HttpMethod::GET,
            url: String::from(DEFAULT_HTTP_URL),
            body: String::new(),
            headers: vec![
                Header::new("Content-Type", "application/json"),
                Header::new("Accept", "application/json"),
            ],
            auth: AuthType::None,
            ignore_ssl_errors: false,
            active_panel: Panel::Url,
            cursor_position: 24,
            response: Response::default(),
            highlighted_response: crate::tui::widgets::highlight_json(&Response::default().body),
            response_scroll: 0,
            is_loading: false,
            selected_header: 0,
            auth_field: AuthField::Token,
            history_index: None,
            workspace: None,
            workspace_path_input: String::new(),
            selected_endpoint: 0,
            show_curl_import: false,
            curl_import_buffer: String::new(),
            show_workspace_input: false,
        }
    }
}

impl Default for WsRenderState {
    fn default() -> Self {
        use crate::constants::DEFAULT_WS_URL;
        WsRenderState {
            url: String::from(DEFAULT_WS_URL),
            connected: false,
            messages: Vec::new(),
            input: String::new(),
            scroll: 0,
        }
    }
}

impl Default for GqlRenderState {
    fn default() -> Self {
        GqlRenderState {
            endpoint: String::from("https://api.example.com/graphql"),
            query: String::from("query {\n  \n}"),
            variables: String::from("{}"),
            active_field: GqlField::Query,
            response: String::new(),
            response_scroll: 0,
            is_loading: false,
            time_ms: 0,
        }
    }
}
