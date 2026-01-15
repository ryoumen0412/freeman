//! Freeman TUI - Actor-based API testing tool
//!
//! Architecture:
//! - UI Layer (Ratatui) - synchronous terminal rendering
//! - App Layer - central state machine processing events
//! - Network Layer (Tokio) - async HTTP execution

mod models;
mod storage;
mod ui;
mod curl;
mod discovery;
mod messages;
mod app;
mod network;
mod constants;

use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::*,
};
use tokio::sync::mpsc;

use messages::{UiEvent, NetworkCommand, NetworkResponse, RenderState};
use messages::ui_events::{key_to_ui_event, InputMode, Panel};
use app::AppActor;
use network::NetworkActor;
use models::AuthType;
use ui::{highlight_json, method_color, status_color};
use discovery::AuthRequirement;

/// Terminal cleanup guard
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to file
    let file_appender = tracing_appender::rolling::never(".", "freeman.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();
        
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    let (net_cmd_tx, net_cmd_rx) = mpsc::unbounded_channel::<NetworkCommand>();
    let (net_resp_tx, net_resp_rx) = mpsc::unbounded_channel::<NetworkResponse>();
    let (render_tx, mut render_rx) = mpsc::unbounded_channel::<RenderState>();

    // Spawn network actor
    let network_actor = NetworkActor::new(net_resp_tx);
    tokio::spawn(network_actor.run(net_cmd_rx));

    // Spawn app actor
    let app_actor = AppActor::new(net_cmd_tx, render_tx);
    tokio::spawn(app_actor.run(ui_rx, net_resp_rx));

    // Run UI loop (synchronous with async polling)
    run_ui_loop(&mut terminal, ui_tx, &mut render_rx).await?;

    Ok(())
}

/// Run the synchronous UI rendering loop
async fn run_ui_loop(
    terminal: &mut Terminal<impl Backend>,
    ui_tx: mpsc::UnboundedSender<UiEvent>,
    render_rx: &mut mpsc::UnboundedReceiver<RenderState>,
) -> anyhow::Result<()> {
    let mut current_state = RenderState::default();

    loop {
        // Draw with current state
        terminal.draw(|f| draw_ui(f, &current_state))?;

        // Poll for events with timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(event) = key_to_ui_event(
                    key,
                    current_state.active_tab,
                    current_state.active_panel,
                    current_state.input_mode,
                    current_state.show_help,
                    current_state.show_curl_import,
                    current_state.show_workspace_input,
                ) {
                    if matches!(event, UiEvent::Quit) {
                        let _ = ui_tx.send(event);
                        break;
                    }
                    let _ = ui_tx.send(event);
                }
            }
        }

        // Check for state updates (non-blocking)
        while let Ok(state) = render_rx.try_recv() {
            current_state = state;
        }
    }

    Ok(())
}

// ============================================================================
// UI Drawing Functions
// ============================================================================

fn draw_ui(f: &mut Frame, state: &RenderState) {
    use crate::messages::ui_events::AppTab;
    
    let area = f.area();

    // Main layout with tab bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Tab bar
            Constraint::Min(0),     // Content
            Constraint::Length(1),  // Status bar
        ])
        .split(area);

    // Draw tab bar
    draw_tab_bar(f, state, main_chunks[0]);

    // Draw content based on active tab
    match state.active_tab {
        AppTab::Http => draw_http_tab(f, state, main_chunks[1]),
        AppTab::WebSocket => draw_ws_tab(f, state, main_chunks[1]),
    }

    // Status bar
    draw_status_bar(f, state, main_chunks[2]);

    // Popups
    if state.show_help {
        draw_help_popup(f, area);
    }

    if state.show_curl_import {
        draw_curl_import_popup(f, state, area);
    }

    if state.show_workspace_input {
        draw_workspace_input_popup(f, state, area);
    }
}

fn draw_tab_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    use crate::messages::ui_events::AppTab;
    
    let tabs = vec![
        Span::styled(
            " 1:HTTP ",
            if state.active_tab == AppTab::Http {
                Style::default().fg(Color::Black).bg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::Gray)
            }
        ),
        Span::raw(" "),
        Span::styled(
            " 2:WebSocket ",
            if state.active_tab == AppTab::WebSocket {
                Style::default().fg(Color::Black).bg(Color::Magenta).bold()
            } else {
                Style::default().fg(Color::Gray)
            }
        ),
        Span::styled(
            if state.ws_connected { " [*]" } else { "" },
            Style::default().fg(Color::Green)
        ),
    ];
    
    let tab_line = Line::from(tabs);
    f.render_widget(Paragraph::new(tab_line), area);
}

fn draw_http_tab(f: &mut Frame, state: &RenderState, area: Rect) {
    // HTTP tab layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Method + URL
            Constraint::Length(8),  // Panels (Body/Headers/Auth)
            Constraint::Min(5),     // Response
        ])
        .split(area);

    draw_url_bar(f, state, chunks[0]);
    draw_middle_panels(f, state, chunks[1]);
    draw_response(f, state, chunks[2]);
}

fn draw_ws_tab(f: &mut Frame, state: &RenderState, area: Rect) {
    // WebSocket tab layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // URL + Connect
            Constraint::Min(5),     // Messages log
            Constraint::Length(3),  // Send input
        ])
        .split(area);

    // URL bar
    let connected_indicator = if state.ws_connected { " [+] Connected" } else { " [-] Disconnected" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if state.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Magenta)
        })
        .title(format!(" WebSocket{} ", connected_indicator));
    
    let url_text = Paragraph::new(state.ws_url.as_str()).block(block);
    f.render_widget(url_text, chunks[0]);

    // Messages log
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .title(" Messages (↑/↓ scroll) ");
    
    let mut lines: Vec<Line> = Vec::new();
    for entry in &state.ws_messages {
        let style = match entry.direction {
            crate::app::state::WsDirection::Sent => Style::default().fg(Color::Cyan),
            crate::app::state::WsDirection::Received => Style::default().fg(Color::Green),
            crate::app::state::WsDirection::System => Style::default().fg(Color::Yellow),
        };
        let prefix = match entry.direction {
            crate::app::state::WsDirection::Sent => ">> ",
            crate::app::state::WsDirection::Received => "<< ",
            crate::app::state::WsDirection::System => "-- ",
        };
        lines.push(Line::from(Span::styled(format!("{}{}", prefix, entry.content), style)));
    }
    
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No messages yet. Press 'c' to connect, 's' to send.",
            Style::default().fg(Color::DarkGray)
        )));
    }
    
    let messages = Paragraph::new(lines)
        .block(messages_block)
        .scroll((state.ws_scroll, 0));
    f.render_widget(messages, chunks[1]);

    // Send input
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if state.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        })
        .title(" Send Message (e=edit, s=send) ");
    
    let input = Paragraph::new(state.ws_input.as_str()).block(input_block);
    f.render_widget(input, chunks[2]);
}

fn draw_url_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Url;
    let mcolor = method_color(state.method.as_str());

    let border_style = if is_focused && state.input_mode == InputMode::Editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let loading = if state.is_loading { " [...]" } else { "" };
    let history_indicator = state.history_index.map(|i| format!(" [{}]", i + 1)).unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" {}{}{} ", state.method.as_str(), loading, history_indicator))
        .title_style(Style::default().fg(mcolor).bold());

    let input = Paragraph::new(state.url.as_str()).block(block);
    f.render_widget(input, area);

    // Cursor
    if is_focused && state.input_mode == InputMode::Editing {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + state.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

fn draw_middle_panels(f: &mut Frame, state: &RenderState, area: Rect) {
    let tabs_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Tab bar
    let tab_titles = vec!["Body", "Headers", "Auth"];
    let selected_tab = match state.active_panel {
        Panel::Body => 0,
        Panel::Headers => 1,
        Panel::Auth => 2,
        _ => 0,
    };

    let tabs = ui::render_tabs(&tab_titles, selected_tab);
    f.render_widget(tabs, tabs_area[0]);

    // Panel content
    let content_area = tabs_area[1];

    match state.active_panel {
        Panel::Body | Panel::Url | Panel::Response => {
            draw_body_panel(f, state, content_area);
        }
        Panel::Headers => {
            draw_headers_panel(f, state, content_area);
        }
        Panel::Auth => {
            draw_auth_panel(f, state, content_area);
        }
        Panel::Workspace => {
            draw_workspace_panel(f, state, content_area);
        }
    }
}

fn draw_body_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Body;
    let border_style = if is_focused && state.input_mode == InputMode::Editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = if state.method.has_body() {
        " Body (JSON) "
    } else {
        " Body (disabled for GET/DELETE) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let content = if state.method.has_body() {
        state.body.as_str()
    } else {
        ""
    };

    let body = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(body, area);

    if is_focused && state.input_mode == InputMode::Editing && state.method.has_body() {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + state.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

fn draw_headers_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Headers;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = state.headers.iter()
        .enumerate()
        .map(|(i, h)| {
            let style = if !h.enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_focused && i == state.selected_header {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if h.enabled { "[x]" } else { "[ ]" };
            ListItem::new(format!("{} {}: {}", prefix, h.key, h.value)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Headers (a:add d:del Enter:toggle) "));
    f.render_widget(list, area);
}

fn draw_auth_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Auth;
    let border_style = if is_focused && state.input_mode == InputMode::Editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (auth_type, content) = match &state.auth {
        AuthType::None => ("None", String::from("Press 't' to cycle auth type")),
        AuthType::Bearer(token) => ("Bearer", format!("Token: {}", if token.is_empty() { "<empty>" } else { token })),
        AuthType::Basic { username, password } => {
            let pass_display = if password.is_empty() { 
                "<empty>".to_string() 
            } else { 
                "*".repeat(password.len()) 
            };
            ("Basic", format!("User: {}  Pass: {}", 
                if username.is_empty() { "<empty>" } else { username },
                pass_display
            ))
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Auth: {} (t:cycle) ", auth_type));

    let auth = Paragraph::new(content).block(block);
    f.render_widget(auth, area);

    if is_focused && state.input_mode == InputMode::Editing {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + state.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

fn draw_workspace_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Workspace;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    match &state.workspace {
        Some(ws) => {
            let title = format!(" 📂 {} ({}) - {} endpoints ", 
                ws.title.as_deref().unwrap_or("Workspace"),
                ws.framework.as_str(),
                ws.endpoints.len()
            );

            let items: Vec<ListItem> = ws.endpoints.iter()
                .map(|ep| {
                    let mcolor = match ep.method.as_str() {
                        "GET" => Color::Green,
                        "POST" => Color::Yellow,
                        "PUT" => Color::Blue,
                        "PATCH" => Color::Cyan,
                        "DELETE" => Color::Red,
                        _ => Color::White,
                    };
                    
                    let auth_indicator = match &ep.auth {
                        AuthRequirement::None => "",
                        AuthRequirement::Bearer => " 🔑",
                        AuthRequirement::Basic => " 🔐",
                        _ => " 🔒",
                    };

                    let method_span = Span::styled(
                        format!("{:6}", ep.method),
                        Style::default().fg(mcolor).bold()
                    );
                    let path_span = Span::raw(format!(" {}{}", ep.path, auth_indicator));
                    
                    ListItem::new(Line::from(vec![method_span, path_span]))
                })
                .collect();

            let highlight_style = if is_focused {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title))
                .highlight_style(highlight_style);

            let mut list_state = ListState::default();
            list_state.select(Some(state.selected_endpoint));
            
            f.render_stateful_widget(list, area, &mut list_state);
        }
        Option::None => {
            let content = "No workspace loaded.\n\nPress 'o' to open a project directory.";
            let paragraph = Paragraph::new(content)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" 📂 Workspace "))
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        }
    }
}

fn draw_response(f: &mut Frame, state: &RenderState, area: Rect) {
    let is_focused = state.active_panel == Panel::Response;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let status_text = match state.response.status_code {
        Some(code) => {
            let color = status_color(code);
            Span::styled(format!(" {} ", code), Style::default().fg(color).bold())
        }
        None => Span::raw(" Response "),
    };

    let time_text = if state.response.time_ms > 0 {
        format!(" {}ms ", state.response.time_ms)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(status_text)
        .title_bottom(Line::from(time_text).right_aligned());

    // Use syntax highlighting for JSON
    let lines = highlight_json(&state.response.body);
    let response = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((state.response_scroll, 0));
    f.render_widget(response, area);
}

fn draw_status_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    let status = if state.is_loading {
        " Loading... "
    } else if state.input_mode == InputMode::Editing {
        " ESC:stop editing | arrows:move | Tab:next field "
    } else {
        " Tab:panel | e:edit | m:method | s:send | ?:help | q:quit "
    };

    let bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(bar, area);
}

fn draw_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    let help_text = r#"
 FREEMAN TUI - Keyboard Shortcuts

 NAVIGATION
   Tab / Shift+Tab    Switch panels
   ↑ / ↓              Scroll response / navigate headers
   Ctrl+↑ / Ctrl+↓    Navigate history

 REQUEST
   m                  Cycle HTTP method
   s / Enter          Send request
   e                  Edit current field
   i                  Import cURL (URL panel)
   c                  Copy as cURL

 HEADERS
   a                  Add new header
   d                  Delete selected header
   Enter              Toggle header enabled

 AUTH
   t                  Cycle auth type (None/Bearer/Basic)
   Tab                Switch between username/password

 GENERAL
   ?                  Toggle this help
   q / Ctrl+C         Quit

 Press any key to close...
"#;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));

    let help = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);
}

fn draw_curl_import_popup(f: &mut Frame, state: &RenderState, area: Rect) {
    let popup_area = centered_rect(80, 30, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Import cURL (Enter to import, Esc to cancel) ")
        .style(Style::default().bg(Color::Black));

    let content = if state.curl_import_buffer.is_empty() {
        "Paste cURL command here..."
    } else {
        &state.curl_import_buffer
    };

    let input = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);
}

fn draw_workspace_input_popup(f: &mut Frame, state: &RenderState, area: Rect) {
    let popup_area = centered_rect(60, 20, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 📂 Open Workspace (Enter to load, Esc to cancel) ")
        .style(Style::default().bg(Color::Black));

    let content = if state.workspace_path_input.is_empty() {
        "Enter project directory path...\n\nExample: ~/projects/my-api"
    } else {
        &state.workspace_path_input
    };

    let input = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}