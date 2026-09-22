//! App actor - message loop processing UI events and network responses

use tokio::sync::mpsc;

use crate::app::state::AppState;
use crate::messages::{NetworkCommand, NetworkResponse, RenderState, UiEvent};

/// App actor that processes UI events and network responses
pub struct AppActor {
    state: AppState,
    network_tx: mpsc::UnboundedSender<NetworkCommand>,
    render_tx: mpsc::UnboundedSender<std::sync::Arc<RenderState>>,
}

impl AppActor {
    pub fn new(
        network_tx: mpsc::UnboundedSender<NetworkCommand>,
        render_tx: mpsc::UnboundedSender<std::sync::Arc<RenderState>>,
    ) -> Self {
        AppActor {
            state: AppState::new(),
            network_tx,
            render_tx,
        }
    }

    /// Run the actor message loop
    pub async fn run(
        mut self,
        mut ui_rx: mpsc::UnboundedReceiver<UiEvent>,
        mut net_rx: mpsc::UnboundedReceiver<NetworkResponse>,
    ) {
        // Send initial render state
        let _ = self
            .render_tx
            .send(std::sync::Arc::new(self.state.to_render_state()));

        loop {
            tokio::select! {
                Some(event) = ui_rx.recv() => {
                    if self.handle_ui_event(event) {
                        // Quit signal received
                        let _ = self.network_tx.send(NetworkCommand::Shutdown);
                        break;
                    }
                    let _ = self.render_tx.send(std::sync::Arc::new(self.state.to_render_state()));
                }
                Some(response) = net_rx.recv() => {
                    self.state.handle_response(response);
                    let _ = self.render_tx.send(std::sync::Arc::new(self.state.to_render_state()));
                }
                else => break,
            }
        }
    }

    /// Handle a UI event, returns true if quit was requested
    fn handle_ui_event(&mut self, event: UiEvent) -> bool {
        match event {
            // Tab switching (cross-state: navigation + ui)
            UiEvent::SwitchTab(tab) => self.state.switch_tab(tab),

            // Panel navigation
            UiEvent::NextPanel => self.state.next_panel(),
            UiEvent::PrevPanel => self.state.prev_panel(),
            UiEvent::FocusWorkspace => self.state.focus_workspace(),

            // HTTP scrolling (direct on sub-state)
            UiEvent::ScrollUp => self.state.http.scroll_up(),
            UiEvent::ScrollDown => self.state.http.scroll_down(),

            // Input editing (cross-state: ui + current_input)
            UiEvent::StartEditing => self.state.start_editing(),
            UiEvent::StopEditing => self.state.stop_editing(),
            UiEvent::CharInput(c) => self.state.enter_char(c),
            UiEvent::Backspace => self.state.delete_char(),
            UiEvent::CursorLeft => self.state.move_cursor_left(),
            UiEvent::CursorRight => self.state.move_cursor_right(),

            // Request actions
            UiEvent::CycleMethod => self.state.http.cycle_method(),
            UiEvent::ToggleSslErrors => self.state.http.toggle_ssl_errors(),
            UiEvent::SendRequest => {
                if self.state.ui.input_mode == crate::messages::ui_events::InputMode::Editing {
                    self.state.stop_editing();
                }
                if let Some(cmd) = self.state.prepare_streaming_request() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::CancelRequest => {
                if let Some(cmd) = self.state.http.cancel_request() {
                    let _ = self.network_tx.send(cmd);
                }
            }

            // Headers (direct on sub-state)
            UiEvent::NextHeader => self.state.http.next_header(),
            UiEvent::PrevHeader => self.state.http.prev_header(),
            UiEvent::ToggleHeader => self.state.http.toggle_header(),
            UiEvent::AddHeader => self.state.http.add_header(),
            UiEvent::DeleteHeader => self.state.http.delete_header(),

            // Auth
            UiEvent::CycleAuth => self.state.http.cycle_auth(),
            UiEvent::NextAuthField => self.state.next_auth_field(), // cross-state: http + ui

            // History (cross-state: storage + http + history + ui)
            UiEvent::HistoryPrev => self.state.history_prev(),
            UiEvent::HistoryNext => self.state.history_next(),

            // Workspace
            UiEvent::OpenWorkspaceInput => self.state.open_workspace_input(),
            UiEvent::WorkspacePathChar(c) => self.state.workspace.path_char(c),
            UiEvent::WorkspacePathBackspace => self.state.workspace.path_backspace(),
            UiEvent::WorkspacePathAutocomplete => self.state.workspace.path_autocomplete(),
            UiEvent::LoadWorkspace => {
                let path = self.state.workspace.path_input.clone();
                match crate::discovery::load_workspace(&path) {
                    Ok(result) => self.state.apply_workspace_result(result),
                    Err(err) => self.state.apply_workspace_error(&err),
                }
            }
            UiEvent::CancelWorkspaceInput => self.state.cancel_workspace_input(), // cross-state
            UiEvent::NextEndpoint => self.state.workspace.next_endpoint(),
            UiEvent::PrevEndpoint => self.state.workspace.prev_endpoint(),
            UiEvent::SelectEndpoint => self.state.select_endpoint(), // cross-state

            // cURL
            UiEvent::ShowCurlImport => self.state.ui.open_curl_import(),
            UiEvent::CurlImportChar(c) => self.state.ui.curl_import_char(c),
            UiEvent::CurlImportBackspace => self.state.ui.curl_import_backspace(),
            UiEvent::ImportCurl => self.state.import_curl(), // cross-state: ui + http
            UiEvent::CancelCurlImport => self.state.ui.cancel_curl_import(),
            UiEvent::ExportCurl => self.state.http.export_curl(),

            // WebSocket
            UiEvent::WsConnect => {
                if let Some(cmd) = self.state.ws_connect() {
                    // cross-state: ws + next_id
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsDisconnect => {
                if let Some(cmd) = self.state.ws.disconnect() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsSend => {
                if let Some(cmd) = self.state.ws.send() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsEditUrl => self.state.ws_start_url_edit(), // cross-state: ws + ui
            UiEvent::WsEditMessage => self.state.ws_start_input_edit(), // cross-state: ws + ui
            UiEvent::WsCharInput(c) => self.state.ws.char_input(c),
            UiEvent::WsBackspace => self.state.ws.backspace(),
            UiEvent::WsCursorLeft => self.state.ws.cursor_left(),
            UiEvent::WsCursorRight => self.state.ws.cursor_right(),

            // GraphQL
            UiEvent::GqlExecuteQuery => {
                if self.state.ui.input_mode == crate::messages::ui_events::InputMode::Editing {
                    self.state.stop_editing();
                }
                if let Some(cmd) = self.state.gql_execute_query() {
                    // cross-state
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::GqlEditEndpoint => self.state.gql_edit_endpoint(), // cross-state: gql + ui
            UiEvent::GqlEditQuery => self.state.gql_edit_query(),       // cross-state: gql + ui
            UiEvent::GqlEditVariables => self.state.gql_edit_variables(), // cross-state: gql + ui
            UiEvent::GqlCharInput(c) => self.state.gql.char_input(c),
            UiEvent::GqlBackspace => self.state.gql.backspace(),
            UiEvent::GqlCursorLeft => self.state.gql.cursor_left(),
            UiEvent::GqlCursorRight => self.state.gql.cursor_right(),
            UiEvent::GqlNextField => self.state.gql.next_field(),
            UiEvent::GqlScrollUp => self.state.gql.scroll_up(),
            UiEvent::GqlScrollDown => self.state.gql.scroll_down(),

            // Popups
            UiEvent::ToggleHelp => self.state.toggle_help(),
            UiEvent::CloseHelp => self.state.close_help(),

            // System
            UiEvent::Quit => return true,
        }

        false
    }
}
