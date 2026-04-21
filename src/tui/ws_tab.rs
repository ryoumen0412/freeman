//! WebSocket tab rendering.

use ratatui::{prelude::*, widgets::*};

use crate::messages::ui_events::InputMode;
use crate::messages::RenderState;
use crate::tui::widgets::render_input;

/// Draw the WebSocket tab content.
pub fn draw_ws_tab(f: &mut Frame, state: &RenderState, area: Rect) {
    let ws = &state.ws;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // URL + Connect
            Constraint::Min(5),    // Messages log
            Constraint::Length(3), // Send input
        ])
        .split(area);

    // URL bar
    let connected_indicator = if ws.connected {
        " [+] Connected"
    } else {
        " [-] Disconnected"
    };
    let title = format!(" WebSocket{} ", connected_indicator);
    let url_widget = render_input(
        ws.url.as_str(),
        &title,
        state.input_mode == InputMode::Editing,
        true,
        Color::Magenta,
        false,
    );
    f.render_widget(url_widget, chunks[0]);

    // Messages log
    draw_messages_log(f, state, chunks[1]);

    // Send input
    let send_widget = render_input(
        ws.input.as_str(),
        " Send Message (e=edit, s=send) ",
        state.input_mode == InputMode::Editing,
        false,
        Color::White,
        false,
    );
    f.render_widget(send_widget, chunks[2]);
}

/// Draw the WebSocket messages log.
fn draw_messages_log(f: &mut Frame, state: &RenderState, area: Rect) {
    let ws = &state.ws;

    let messages_block = Block::default()
        .borders(Borders::ALL)
        .title(" Messages (↑/↓ scroll) ");

    let mut lines: Vec<Line> = Vec::new();
    for entry in &ws.messages {
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
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, entry.content),
            style,
        )));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No messages yet. Press 'c' to connect, 's' to send.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let messages = Paragraph::new(lines)
        .block(messages_block)
        .scroll((ws.scroll, 0));
    f.render_widget(messages, area);
}
