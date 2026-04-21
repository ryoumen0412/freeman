//! Chrome elements - tab bar, status bar, and layout utilities.

use ratatui::{prelude::*, widgets::*};

use crate::messages::ui_events::{AppTab, InputMode};
use crate::messages::RenderState;
use crate::tui::theme::Theme;

/// Draw the application tab bar (HTTP / WebSocket / GraphQL).
pub fn draw_tab_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    let theme = Theme::default();
    let tabs = vec![
        Span::styled(
            " 1:HTTP ",
            if state.active_tab == AppTab::Http {
                Style::default().fg(Color::Black).bg(theme.primary).bold()
            } else {
                Style::default().fg(theme.text_muted)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " 2:WebSocket ",
            if state.active_tab == AppTab::WebSocket {
                Style::default().fg(Color::Black).bg(theme.secondary).bold()
            } else {
                Style::default().fg(theme.text_muted)
            },
        ),
        Span::styled(
            if state.ws.connected { " [*]" } else { "" },
            Style::default().fg(Color::Green),
        ),
        Span::raw(" "),
        Span::styled(
            " 3:GraphQL ",
            if state.active_tab == AppTab::GraphQL {
                Style::default().fg(Color::Black).bg(theme.success).bold()
            } else {
                Style::default().fg(theme.text_muted)
            },
        ),
        Span::styled(
            if state.gql.is_loading { " [...]" } else { "" },
            Style::default().fg(Color::Yellow),
        ),
    ];

    let tab_line = Line::from(tabs);
    f.render_widget(Paragraph::new(tab_line), area);
}

/// Draw the bottom status bar with context-sensitive keybindings.
pub fn draw_status_bar(f: &mut Frame, state: &RenderState, area: Rect) {
    let ssl_warning = if state.active_tab == AppTab::Http && state.http.ignore_ssl_errors {
        " [⚠ SSL OFF] "
    } else {
        ""
    };

    let status = if state.http.is_loading {
        format!("{}Loading... ", ssl_warning)
    } else if state.input_mode == InputMode::Editing {
        format!(
            "{}ESC:stop editing | arrows:move | Tab:next field ",
            ssl_warning
        )
    } else {
        format!(
            "{}Tab:panel | e:edit | m:method | s:send | k:ssl | ?:help | q:quit ",
            ssl_warning
        )
    };

    let theme = Theme::default();
    let bar = Paragraph::new(status).style(Style::default().fg(theme.text_muted));
    f.render_widget(bar, area);
}

/// Calculate a centered rectangle within the given area.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
