//! HTTP tab rendering - URL bar, body, headers, auth, workspace, response panels.

use ratatui::{prelude::*, widgets::*};

use crate::discovery::AuthRequirement;
use crate::messages::ui_events::{InputMode, Panel};
use crate::messages::RenderState;
use crate::models::AuthType;
use crate::tui::widgets::{
    highlight_json, method_color, render_header_list, render_input, render_tabs, status_color,
};

/// Draw the HTTP tab content.
pub fn draw_http_tab(f: &mut Frame, state: &RenderState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Method + URL
            Constraint::Length(8), // Panels (Body/Headers/Auth)
            Constraint::Min(5),    // Response
        ])
        .split(area);

    draw_url_bar(f, state, chunks[0]);
    draw_middle_panels(f, state, chunks[1]);
    draw_response(f, state, chunks[2]);
}

// ============================================================================
// URL Bar
// ============================================================================

fn draw_url_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Url;
    let mcolor = method_color(http.method.as_str());

    let border_style = if is_focused && state.input_mode == InputMode::Editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let loading = if http.is_loading { " [...]" } else { "" };
    let history_indicator = http
        .history_index
        .map(|i| format!(" [{}]", i + 1))
        .unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(
            " {}{}{} ",
            http.method.as_str(),
            loading,
            history_indicator
        ))
        .title_style(Style::default().fg(mcolor).bold());

    let input = Paragraph::new(http.url.as_str()).block(block);
    f.render_widget(input, area);

    // Cursor
    if is_focused && state.input_mode == InputMode::Editing {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + http.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

// ============================================================================
// Middle Panels (Body / Headers / Auth / Workspace)
// ============================================================================

fn draw_middle_panels(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;

    let tabs_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Tab bar
    let tab_titles = vec!["Body", "Headers", "Auth"];
    let selected_tab = match http.active_panel {
        Panel::Body => 0,
        Panel::Headers => 1,
        Panel::Auth => 2,
        _ => 0,
    };

    let tabs = render_tabs(&tab_titles, selected_tab);
    f.render_widget(tabs, tabs_area[0]);

    // Panel content
    let content_area = tabs_area[1];

    match http.active_panel {
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

// ============================================================================
// Body Panel
// ============================================================================

fn draw_body_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Body;
    let is_editing = is_focused && state.input_mode == InputMode::Editing;

    let title = if http.method.has_body() {
        " Body (JSON) "
    } else {
        " Body (disabled for GET/DELETE) "
    };

    let content = if http.method.has_body() {
        http.body.as_str()
    } else {
        ""
    };

    let widget = render_input(content, title, is_editing, is_focused, Color::Cyan, true);
    f.render_widget(widget, area);

    if is_editing && http.method.has_body() {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + http.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

// ============================================================================
// Headers Panel
// ============================================================================

fn draw_headers_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Headers;
    let list = render_header_list(
        &http.headers,
        " Headers (a:add d:del Enter:toggle) ",
        http.selected_header,
        is_focused,
    );
    f.render_widget(list, area);
}

// ============================================================================
// Auth Panel
// ============================================================================

fn draw_auth_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Auth;
    let border_style = if is_focused && state.input_mode == InputMode::Editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (auth_type, content) = match &http.auth {
        AuthType::None => ("None", String::from("Press 't' to cycle auth type")),
        AuthType::Bearer(token) => (
            "Bearer",
            format!(
                "Token: {}",
                if token.is_empty() { "<empty>" } else { token }
            ),
        ),
        AuthType::Basic { username, password } => {
            let pass_display = if password.is_empty() {
                "<empty>".to_string()
            } else {
                "*".repeat(password.len())
            };
            (
                "Basic",
                format!(
                    "User: {}  Pass: {}",
                    if username.is_empty() {
                        "<empty>"
                    } else {
                        username
                    },
                    pass_display
                ),
            )
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
        let cursor_x = (area.x + http.cursor_position as u16 + 1).min(max_x);
        f.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

// ============================================================================
// Workspace Panel
// ============================================================================

fn draw_workspace_panel(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Workspace;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    match &http.workspace {
        Some(ws) => {
            let title = format!(
                " 📂 {} ({}) - {} endpoints ",
                ws.title.as_deref().unwrap_or("Workspace"),
                ws.framework.as_str(),
                ws.endpoints.len()
            );

            let items: Vec<ListItem> = ws
                .endpoints
                .iter()
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
                        Style::default().fg(mcolor).bold(),
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
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .title(title),
                )
                .highlight_style(highlight_style);

            let mut list_state = ListState::default();
            list_state.select(Some(http.selected_endpoint));

            f.render_stateful_widget(list, area, &mut list_state);
        }
        Option::None => {
            let content = "No workspace loaded.\n\nPress 'o' to open a project directory.";
            let paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .title(" 📂 Workspace "),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        }
    }
}

// ============================================================================
// Response Panel
// ============================================================================

fn draw_response(f: &mut Frame, state: &RenderState, area: Rect) {
    let http = &state.http;
    let is_focused = http.active_panel == Panel::Response;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let status_text = match http.response.status_code {
        Some(code) => {
            let color = status_color(code);
            Span::styled(format!(" {} ", code), Style::default().fg(color).bold())
        }
        None => Span::raw(" Response "),
    };

    let time_text = if http.response.time_ms > 0 {
        format!(" {}ms ", http.response.time_ms)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(status_text)
        .title_bottom(Line::from(time_text).right_aligned());

    // Use syntax highlighting for JSON
    let lines = highlight_json(&http.response.body);
    let response = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((http.response_scroll, 0));
    f.render_widget(response, area);
}
