//! Render state - data structure sent from App layer to UI for rendering

use crate::app::state::WsLogEntry;
use crate::discovery::WorkspaceProject;
use crate::messages::ui_events::{AppTab, AuthField, GqlField, InputMode, Panel};
use crate::models::{AuthType, Header, HttpMethod, Response};

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
    /// Whether SSL certificate errors should be ignored (for testing environments)
    pub ignore_ssl_errors: bool,

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
    #[allow(dead_code)] // Reserved for cursor display
    pub ws_url_cursor: usize,
    #[allow(dead_code)] // Reserved for cursor display
    pub ws_editing_url: bool,
    pub ws_connected: bool,
    pub ws_messages: Vec<WsLogEntry>,
    pub ws_input: String,
    #[allow(dead_code)] // Reserved for cursor display
    pub ws_input_cursor: usize,
    pub ws_scroll: u16,

    // GraphQL
    pub gql_endpoint: String,
    #[allow(dead_code)] // Reserved for cursor display
    pub gql_endpoint_cursor: usize,
    pub gql_query: String,
    #[allow(dead_code)] // Reserved for cursor display
    pub gql_query_cursor: usize,
    pub gql_variables: String,
    #[allow(dead_code)] // Reserved for cursor display
    pub gql_variables_cursor: usize,
    pub gql_active_field: GqlField,
    pub gql_response: String,
    pub gql_response_scroll: u16,
    pub gql_is_loading: bool,
    pub gql_time_ms: u64,
}

impl Default for RenderState {
    fn default() -> Self {
        use crate::constants::{DEFAULT_HTTP_URL, DEFAULT_WS_URL};
        RenderState {
            active_tab: AppTab::Http,
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
            ws_url: String::from(DEFAULT_WS_URL),
            ws_url_cursor: 0,
            ws_editing_url: false,
            ws_connected: false,
            ws_messages: Vec::new(),
            ws_input: String::new(),
            ws_input_cursor: 0,
            ws_scroll: 0,
            gql_endpoint: String::from("https://api.example.com/graphql"),
            gql_endpoint_cursor: 0,
            gql_query: String::from("query {\n  \n}"),
            gql_query_cursor: 0,
            gql_variables: String::from("{}"),
            gql_variables_cursor: 0,
            gql_active_field: GqlField::Query,
            gql_response: String::new(),
            gql_response_scroll: 0,
            gql_is_loading: false,
            gql_time_ms: 0,
        }
    }
}
