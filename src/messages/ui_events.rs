//! UI events - messages from UI layer to App layer

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Application tabs
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum AppTab {
    #[default]
    Http,
    WebSocket,
}

/// Events generated from user input in the UI layer
#[derive(Debug, Clone)]
pub enum UiEvent {
    // Tab navigation
    SwitchTab(AppTab),
    
    // Panel navigation
    NextPanel,
    PrevPanel,
    ScrollUp,
    ScrollDown,
    
    // Input editing
    StartEditing,
    StopEditing,
    CharInput(char),
    Backspace,
    CursorLeft,
    CursorRight,
    
    // HTTP Request actions
    SendRequest,
    CancelRequest,
    CycleMethod,
    
    // Headers
    NextHeader,
    PrevHeader,
    ToggleHeader,
    AddHeader,
    DeleteHeader,
    
    // Auth
    CycleAuth,
    NextAuthField,
    
    // History (reserved for future key bindings)
    #[allow(dead_code)]
    HistoryPrev,
    #[allow(dead_code)]
    HistoryNext,
    
    // Workspace
    FocusWorkspace,
    OpenWorkspaceInput,
    WorkspacePathChar(char),
    WorkspacePathBackspace,
    WorkspacePathAutocomplete,
    LoadWorkspace,
    CancelWorkspaceInput,
    NextEndpoint,
    PrevEndpoint,
    SelectEndpoint,
    
    // cURL
    ShowCurlImport,
    CurlImportChar(char),
    CurlImportBackspace,
    ImportCurl,
    CancelCurlImport,
    ExportCurl,
    
    // WebSocket actions
    WsConnect,
    WsDisconnect,
    WsSend,
    WsEditUrl,      // Edit WS URL
    WsEditMessage,  // Edit message input
    WsCharInput(char),
    WsBackspace,
    WsCursorLeft,
    WsCursorRight,
    
    // Popups
    ToggleHelp,
    CloseHelp,
    
    // System
    Quit,
}

/// Active panel in the UI (needed for context-aware event mapping)
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Panel {
    Url,
    Body,
    Headers,
    Auth,
    Response,
    Workspace,
}

impl Panel {
    pub fn next(&self) -> Panel {
        match self {
            Panel::Url => Panel::Body,
            Panel::Body => Panel::Headers,
            Panel::Headers => Panel::Auth,
            Panel::Auth => Panel::Response,
            Panel::Response => Panel::Workspace,
            Panel::Workspace => Panel::Url,
        }
    }

    pub fn prev(&self) -> Panel {
        match self {
            Panel::Url => Panel::Workspace,
            Panel::Body => Panel::Url,
            Panel::Headers => Panel::Body,
            Panel::Auth => Panel::Headers,
            Panel::Response => Panel::Auth,
            Panel::Workspace => Panel::Response,
        }
    }
}

/// Input mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

/// Auth editing field
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AuthField {
    Token,
    Username,
    Password,
}

/// Convert a key event to a UiEvent based on current UI context
pub fn key_to_ui_event(
    key: KeyEvent,
    active_tab: AppTab,
    active_panel: Panel,
    input_mode: InputMode,
    show_help: bool,
    show_curl_import: bool,
    show_workspace_input: bool,
) -> Option<UiEvent> {
    use crossterm::event::KeyEventKind;
    
    if key.kind != KeyEventKind::Press {
        return None;
    }
    
    // Global Ctrl shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('x') => return Some(UiEvent::CancelRequest),
            KeyCode::Char('c') => return Some(UiEvent::Quit),
            _ => {}
        }
    }
    
    // Tab switching: 1 and 2 keys (only in normal mode, not editing)
    if input_mode == InputMode::Normal && !show_help && !show_curl_import && !show_workspace_input {
        match key.code {
            KeyCode::Char('1') => return Some(UiEvent::SwitchTab(AppTab::Http)),
            KeyCode::Char('2') => return Some(UiEvent::SwitchTab(AppTab::WebSocket)),
            _ => {}
        }
    }
    
    // Handle popups first (same for all tabs)
    if show_help {
        return Some(UiEvent::CloseHelp);
    }
    
    if show_curl_import {
        return match key.code {
            KeyCode::Esc => Some(UiEvent::CancelCurlImport),
            KeyCode::Enter => Some(UiEvent::ImportCurl),
            KeyCode::Backspace => Some(UiEvent::CurlImportBackspace),
            KeyCode::Char(c) => Some(UiEvent::CurlImportChar(c)),
            _ => None,
        };
    }
    
    if show_workspace_input {
        return match key.code {
            KeyCode::Esc => Some(UiEvent::CancelWorkspaceInput),
            KeyCode::Enter => Some(UiEvent::LoadWorkspace),
            KeyCode::Tab => Some(UiEvent::WorkspacePathAutocomplete),
            KeyCode::Backspace => Some(UiEvent::WorkspacePathBackspace),
            KeyCode::Char(c) => Some(UiEvent::WorkspacePathChar(c)),
            _ => None,
        };
    }
    
    // Tab-specific key handling
    match active_tab {
        AppTab::Http => handle_http_tab_keys(key, active_panel, input_mode),
        AppTab::WebSocket => handle_ws_tab_keys(key, input_mode),
    }
}

/// Handle keys for HTTP tab
fn handle_http_tab_keys(key: KeyEvent, active_panel: Panel, input_mode: InputMode) -> Option<UiEvent> {
    match input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => Some(UiEvent::Quit),
            KeyCode::Char('?') => Some(UiEvent::ToggleHelp),
            KeyCode::Char('i') if active_panel == Panel::Url => Some(UiEvent::ShowCurlImport),
            KeyCode::Char('c') => Some(UiEvent::ExportCurl),
            KeyCode::Tab => Some(UiEvent::NextPanel),
            KeyCode::BackTab => Some(UiEvent::PrevPanel),
            KeyCode::Char('e') | KeyCode::Enter => match active_panel {
                Panel::Url | Panel::Body | Panel::Auth => Some(UiEvent::StartEditing),
                Panel::Headers => Some(UiEvent::ToggleHeader),
                Panel::Workspace => Some(UiEvent::SelectEndpoint),
                Panel::Response => None,
            },
            KeyCode::Char('m') => Some(UiEvent::CycleMethod),
            KeyCode::Char('s') => Some(UiEvent::SendRequest),
            KeyCode::Up => match active_panel {
                Panel::Headers => Some(UiEvent::PrevHeader),
                Panel::Response => Some(UiEvent::ScrollUp),
                Panel::Workspace => Some(UiEvent::PrevEndpoint),
                _ => None,
            },
            KeyCode::Down => match active_panel {
                Panel::Headers => Some(UiEvent::NextHeader),
                Panel::Response => Some(UiEvent::ScrollDown),
                Panel::Workspace => Some(UiEvent::NextEndpoint),
                _ => None,
            },
            KeyCode::Char('w') => Some(UiEvent::FocusWorkspace),
            KeyCode::Char('o') => Some(UiEvent::OpenWorkspaceInput),
            KeyCode::Char('a') if active_panel == Panel::Headers => Some(UiEvent::AddHeader),
            KeyCode::Char('d') if active_panel == Panel::Headers => Some(UiEvent::DeleteHeader),
            KeyCode::Char('t') if active_panel == Panel::Auth => Some(UiEvent::CycleAuth),
            _ => None,
        },
        InputMode::Editing => match key.code {
            KeyCode::Esc => Some(UiEvent::StopEditing),
            KeyCode::Left => Some(UiEvent::CursorLeft),
            KeyCode::Right => Some(UiEvent::CursorRight),
            KeyCode::Backspace => Some(UiEvent::Backspace),
            KeyCode::Char(c) => Some(UiEvent::CharInput(c)),
            KeyCode::Tab if active_panel == Panel::Auth => Some(UiEvent::NextAuthField),
            KeyCode::Enter => {
                if active_panel == Panel::Url {
                    Some(UiEvent::SendRequest)
                } else {
                    Some(UiEvent::StopEditing)
                }
            }
            _ => None,
        },
    }
}

/// Handle keys for WebSocket tab
fn handle_ws_tab_keys(key: KeyEvent, input_mode: InputMode) -> Option<UiEvent> {
    match input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => Some(UiEvent::Quit),
            KeyCode::Char('?') => Some(UiEvent::ToggleHelp),
            KeyCode::Char('c') => Some(UiEvent::WsConnect),
            KeyCode::Char('d') => Some(UiEvent::WsDisconnect),
            KeyCode::Char('u') => Some(UiEvent::WsEditUrl),      // Edit URL
            KeyCode::Char('e') => Some(UiEvent::WsEditMessage),  // Edit message
            KeyCode::Char('s') | KeyCode::Enter => Some(UiEvent::WsSend),
            KeyCode::Up => Some(UiEvent::ScrollUp),
            KeyCode::Down => Some(UiEvent::ScrollDown),
            _ => None,
        },
        InputMode::Editing => match key.code {
            KeyCode::Esc => Some(UiEvent::StopEditing),
            KeyCode::Left => Some(UiEvent::WsCursorLeft),
            KeyCode::Right => Some(UiEvent::WsCursorRight),
            KeyCode::Backspace => Some(UiEvent::WsBackspace),
            KeyCode::Char(c) => Some(UiEvent::WsCharInput(c)),
            KeyCode::Enter => Some(UiEvent::StopEditing),
            _ => None,
        },
    }
}
