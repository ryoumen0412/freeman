//! App state - pure data structure with no I/O logic
//!
//! AppState is composed of specialized sub-states, each with a clear responsibility:
//! - `UiState` — interaction mode, cursor, popups
//! - `NavigationState` — where the user is (tab, panel)
//! - `HttpState` — request, response, loading, streaming, headers/auth UI
//! - `WebSocketState` — WS connection, messages, input
//! - `GraphQLState` — GQL endpoint, query, variables, response
//! - `WorkspaceState` — project discovery, endpoint selection
//! - `HistoryState` — history navigation index

use crate::discovery::WorkspaceProject;
use crate::messages::ui_events::{AppTab, AuthField, GqlField, InputMode, Panel};
use crate::messages::RenderState;
use crate::models::{AuthType, Request, Response};
use crate::storage::Storage;
use ratatui::text::Line;

// ============================================================================
// WebSocket types (unchanged)
// ============================================================================

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
    #[allow(dead_code)] // Reserved for future message timestamp display
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WebSocket connection state
#[derive(Clone, Debug)]
pub struct WebSocketState {
    pub url: String,
    pub url_cursor: usize,
    pub editing_url: bool, // true = editing URL, false = editing message input
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

// ============================================================================
// GraphQL state (unchanged)
// ============================================================================

/// GraphQL state
#[derive(Clone, Debug)]
pub struct GraphQLState {
    pub endpoint: String,
    pub endpoint_cursor: usize,
    pub query: String,
    pub query_cursor: usize,
    pub variables: String,
    pub variables_cursor: usize,
    pub active_field: GqlField,
    pub response: String,
    pub response_scroll: u16,
    pub is_loading: bool,
    pub time_ms: u64,
    pub pending_request_id: Option<u64>,
}

impl Default for GraphQLState {
    fn default() -> Self {
        GraphQLState {
            endpoint: String::from("https://api.example.com/graphql"),
            endpoint_cursor: 0,
            query: String::from("query {\n  \n}"),
            query_cursor: 0,
            variables: String::from("{}"),
            variables_cursor: 0,
            active_field: GqlField::Query,
            response: String::new(),
            response_scroll: 0,
            is_loading: false,
            time_ms: 0,
            pending_request_id: None,
        }
    }
}

// ============================================================================
// New specialized sub-states
// ============================================================================

/// UI interaction state — how the interface is presented/interacted
pub struct UiState {
    /// Current input mode (Normal / Editing)
    pub input_mode: InputMode,
    /// Cursor position in the currently active text field
    pub cursor_position: usize,

    // Popups
    pub show_help: bool,
    pub show_curl_import: bool,
    pub curl_import_buffer: String,
    pub show_workspace_input: bool,

    /// Dummy input fallback to avoid mutating URL accidentally
    pub dummy_input: String,
}

/// Navigation state — where the user is in the application
pub struct NavigationState {
    pub active_tab: AppTab,
    pub active_panel: Panel,
}

/// HTTP domain state — everything about the current request/response cycle
pub struct HttpState {
    // Request
    pub request: Request,

    // Response
    pub response: Response,
    pub highlighted_response: Vec<Line<'static>>,
    pub response_scroll: u16,

    // Loading / streaming
    pub is_loading: bool,
    pub pending_request_id: Option<u64>,
    pub streaming_body: String,
    pub bytes_received: usize,

    // Headers panel
    pub selected_header: usize,

    // Auth panel
    pub auth_field: AuthField,
}

/// Workspace domain state — project discovery and endpoint management
pub struct WorkspaceState {
    pub project: Option<WorkspaceProject>,
    pub path_input: String,
    pub selected_endpoint: usize,
}

/// History navigation state
pub struct HistoryState {
    pub index: Option<usize>,
}

// ============================================================================
// AppState — the aggregator
// ============================================================================

/// Main application state - pure data, no I/O
///
/// Composed of specialized sub-states. Each sub-state owns a coherent
/// slice of the application domain. AppState is the aggregator, not
/// the place where all knowledge lives.
pub struct AppState {
    // UI / navigation
    pub ui: UiState,
    pub navigation: NavigationState,

    // Protocol states
    pub http: HttpState,
    pub ws: WebSocketState,
    pub gql: GraphQLState,

    // Domain states
    pub workspace: WorkspaceState,
    pub history: HistoryState,

    // Storage (persisted data — repository, not domain state)
    pub storage: Storage,

    // Shared ID generator
    pub next_request_id: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            ui: UiState {
                input_mode: InputMode::Normal,
                cursor_position: 24, // Length of default URL
                show_help: false,
                show_curl_import: false,
                curl_import_buffer: String::new(),
                show_workspace_input: false,
                dummy_input: String::new(),
            },
            navigation: NavigationState {
                active_tab: AppTab::Http,
                active_panel: Panel::Url,
            },
            http: HttpState {
                request: Request::default(),
                response: Response::default(),
                highlighted_response: crate::tui::widgets::highlight_json(
                    &Response::default().body,
                ),
                response_scroll: 0,
                is_loading: false,
                pending_request_id: None,
                streaming_body: String::new(),
                bytes_received: 0,
                selected_header: 0,
                auth_field: AuthField::Token,
            },
            ws: WebSocketState::default(),
            gql: GraphQLState::default(),
            workspace: WorkspaceState {
                project: None,
                path_input: String::new(),
                selected_endpoint: 0,
            },
            history: HistoryState { index: None },
            storage: Storage::new(),
            next_request_id: 1,
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
        match self.navigation.active_panel {
            Panel::Url => &self.http.request.url,
            Panel::Body => &self.http.request.body,
            Panel::Auth => match &self.http.request.auth {
                AuthType::Bearer(token) => token,
                AuthType::Basic { username, password } => match self.http.auth_field {
                    AuthField::Token => "",
                    AuthField::Username => username,
                    AuthField::Password => password,
                },
                AuthType::None => "",
            },
            _ => "",
        }
    }

    /// Get mutable reference to current input field
    pub fn current_input_mut(&mut self) -> &mut String {
        match self.navigation.active_panel {
            Panel::Url => &mut self.http.request.url,
            Panel::Body => &mut self.http.request.body,
            Panel::Auth => match &mut self.http.request.auth {
                AuthType::Bearer(token) => token,
                AuthType::Basic { username, password } => match self.http.auth_field {
                    AuthField::Token => &mut self.ui.dummy_input,
                    AuthField::Username => username,
                    AuthField::Password => password,
                },
                AuthType::None => &mut self.ui.dummy_input,
            },
            _ => &mut self.ui.dummy_input,
        }
    }

    /// Convert state to RenderState for UI
    pub fn to_render_state(&self) -> RenderState {
        use crate::messages::render::{GqlRenderState, HttpRenderState, WsRenderState};

        RenderState {
            active_tab: self.navigation.active_tab,
            input_mode: self.ui.input_mode,
            show_help: self.ui.show_help,
            http: HttpRenderState {
                method: self.http.request.method.clone(),
                url: self.http.request.url.clone(),
                body: self.http.request.body.clone(),
                headers: self.http.request.headers.clone(),
                auth: self.http.request.auth.clone(),
                ignore_ssl_errors: self.http.request.ignore_ssl_errors,
                active_panel: self.navigation.active_panel,
                cursor_position: self.ui.cursor_position,
                response: self.http.response.clone(),
                highlighted_response: self.http.highlighted_response.clone(),
                response_scroll: self.http.response_scroll,
                is_loading: self.http.is_loading,
                selected_header: self.http.selected_header,
                auth_field: self.http.auth_field,
                history_index: self.history.index,
                workspace: self.workspace.project.clone(),
                workspace_path_input: self.workspace.path_input.clone(),
                selected_endpoint: self.workspace.selected_endpoint,
                show_curl_import: self.ui.show_curl_import,
                curl_import_buffer: self.ui.curl_import_buffer.clone(),
                show_workspace_input: self.ui.show_workspace_input,
            },
            ws: WsRenderState {
                url: self.ws.url.clone(),
                connected: self.ws.connected,
                messages: self.ws.messages.clone(),
                input: self.ws.input.clone(),
                scroll: self.ws.scroll,
            },
            gql: GqlRenderState {
                endpoint: self.gql.endpoint.clone(),
                query: self.gql.query.clone(),
                variables: self.gql.variables.clone(),
                active_field: self.gql.active_field,
                response: self.gql.response.clone(),
                response_scroll: self.gql.response_scroll,
                is_loading: self.gql.is_loading,
                time_ms: self.gql.time_ms,
            },
        }
    }
}
