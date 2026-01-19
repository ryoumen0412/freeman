//! App actor - message loop processing UI events and network responses

use tokio::sync::mpsc;

use crate::app::state::AppState;
use crate::messages::{NetworkCommand, NetworkResponse, RenderState, UiEvent};

/// App actor that processes UI events and network responses
pub struct AppActor {
    state: AppState,
    network_tx: mpsc::UnboundedSender<NetworkCommand>,
    render_tx: mpsc::UnboundedSender<RenderState>,
}

impl AppActor {
    pub fn new(
        network_tx: mpsc::UnboundedSender<NetworkCommand>,
        render_tx: mpsc::UnboundedSender<RenderState>,
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
        let _ = self.render_tx.send(self.state.to_render_state());

        loop {
            tokio::select! {
                Some(event) = ui_rx.recv() => {
                    if self.handle_ui_event(event) {
                        // Quit signal received
                        let _ = self.network_tx.send(NetworkCommand::Shutdown);
                        break;
                    }
                    let _ = self.render_tx.send(self.state.to_render_state());
                }
                Some(response) = net_rx.recv() => {
                    self.state.handle_response(response);
                    let _ = self.render_tx.send(self.state.to_render_state());
                }
                else => break,
            }
        }
    }

    /// Handle a UI event, returns true if quit was requested
    fn handle_ui_event(&mut self, event: UiEvent) -> bool {
        match event {
            // Tab switching
            UiEvent::SwitchTab(tab) => self.state.switch_tab(tab),

            // Panel navigation
            UiEvent::NextPanel => self.state.next_panel(),
            UiEvent::PrevPanel => self.state.prev_panel(),
            UiEvent::FocusWorkspace => self.state.focus_workspace(),
            UiEvent::ScrollUp => self.state.scroll_up(),
            UiEvent::ScrollDown => self.state.scroll_down(),

            // Input editing
            UiEvent::StartEditing => self.state.start_editing(),
            UiEvent::StopEditing => self.state.stop_editing(),
            UiEvent::CharInput(c) => self.state.enter_char(c),
            UiEvent::Backspace => self.state.delete_char(),
            UiEvent::CursorLeft => self.state.move_cursor_left(),
            UiEvent::CursorRight => self.state.move_cursor_right(),

            // Request actions
            UiEvent::CycleMethod => self.state.cycle_method(),
            UiEvent::ToggleSslErrors => self.state.toggle_ssl_errors(),
            UiEvent::SendRequest => {
                // Stop editing first if in URL panel
                if self.state.input_mode == crate::messages::ui_events::InputMode::Editing {
                    self.state.stop_editing();
                }
                if let Some(cmd) = self.state.prepare_streaming_request() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::CancelRequest => {
                if let Some(cmd) = self.state.cancel_request() {
                    let _ = self.network_tx.send(cmd);
                }
            }

            // Headers
            UiEvent::NextHeader => self.state.next_header(),
            UiEvent::PrevHeader => self.state.prev_header(),
            UiEvent::ToggleHeader => self.state.toggle_header(),
            UiEvent::AddHeader => self.state.add_header(),
            UiEvent::DeleteHeader => self.state.delete_header(),

            // Auth
            UiEvent::CycleAuth => self.state.cycle_auth(),
            UiEvent::NextAuthField => self.state.next_auth_field(),

            // History
            UiEvent::HistoryPrev => self.state.history_prev(),
            UiEvent::HistoryNext => self.state.history_next(),

            // Workspace
            UiEvent::OpenWorkspaceInput => self.state.open_workspace_input(),
            UiEvent::WorkspacePathChar(c) => self.state.workspace_path_char(c),
            UiEvent::WorkspacePathBackspace => self.state.workspace_path_backspace(),
            UiEvent::WorkspacePathAutocomplete => self.state.workspace_path_autocomplete(),
            UiEvent::LoadWorkspace => self.state.load_workspace(),
            UiEvent::CancelWorkspaceInput => self.state.cancel_workspace_input(),
            UiEvent::NextEndpoint => self.state.next_endpoint(),
            UiEvent::PrevEndpoint => self.state.prev_endpoint(),
            UiEvent::SelectEndpoint => self.state.select_endpoint(),

            // cURL
            UiEvent::ShowCurlImport => self.state.show_curl_import(),
            UiEvent::CurlImportChar(c) => self.state.curl_import_char(c),
            UiEvent::CurlImportBackspace => self.state.curl_import_backspace(),
            UiEvent::ImportCurl => self.state.import_curl(),
            UiEvent::CancelCurlImport => self.state.cancel_curl_import(),
            UiEvent::ExportCurl => self.state.export_curl(),

            // WebSocket
            UiEvent::WsConnect => {
                if let Some(cmd) = self.state.ws_connect() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsDisconnect => {
                if let Some(cmd) = self.state.ws_disconnect() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsSend => {
                if let Some(cmd) = self.state.ws_send() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::WsEditUrl => self.state.ws_start_url_edit(),
            UiEvent::WsEditMessage => self.state.ws_start_input_edit(),
            UiEvent::WsCharInput(c) => self.state.ws_char(c),
            UiEvent::WsBackspace => self.state.ws_backspace(),
            UiEvent::WsCursorLeft => self.state.ws_cursor_left(),
            UiEvent::WsCursorRight => self.state.ws_cursor_right(),

            // GraphQL
            UiEvent::GqlExecuteQuery => {
                if self.state.input_mode == crate::messages::ui_events::InputMode::Editing {
                    self.state.stop_editing();
                }
                if let Some(cmd) = self.state.gql_execute_query() {
                    let _ = self.network_tx.send(cmd);
                }
            }
            UiEvent::GqlEditEndpoint => self.state.gql_edit_endpoint(),
            UiEvent::GqlEditQuery => self.state.gql_edit_query(),
            UiEvent::GqlEditVariables => self.state.gql_edit_variables(),
            UiEvent::GqlCharInput(c) => self.state.gql_char(c),
            UiEvent::GqlBackspace => self.state.gql_backspace(),
            UiEvent::GqlCursorLeft => self.state.gql_cursor_left(),
            UiEvent::GqlCursorRight => self.state.gql_cursor_right(),
            UiEvent::GqlNextField => self.state.gql_next_field(),
            UiEvent::GqlScrollUp => self.state.gql_scroll_up(),
            UiEvent::GqlScrollDown => self.state.gql_scroll_down(),

            // Popups
            UiEvent::ToggleHelp => self.state.toggle_help(),
            UiEvent::CloseHelp => self.state.close_help(),

            // System
            UiEvent::Quit => return true,
        }

        false
    }
}
