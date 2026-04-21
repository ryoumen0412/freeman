//! GraphQL tab rendering.

use ratatui::{prelude::*, widgets::*};

use crate::messages::ui_events::{GqlField, InputMode};
use crate::messages::RenderState;
use crate::tui::theme::Theme;
use crate::tui::widgets::{highlight_json, render_input};

pub fn draw_gql_tab(f: &mut Frame, state: &RenderState, area: Rect) {
    let gql = &state.gql;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Min(5),
        ])
        .split(area);

    let theme = Theme::default();
    let ep_edit = gql.active_field == GqlField::Endpoint && state.input_mode == InputMode::Editing;
    f.render_widget(
        render_input(
            gql.endpoint.as_str(),
            " GraphQL Endpoint (u=edit) ",
            ep_edit,
            true,
            theme.success,
            false,
        ),
        chunks[0],
    );

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    let q_edit = gql.active_field == GqlField::Query && state.input_mode == InputMode::Editing;
    f.render_widget(
        render_input(
            gql.query.as_str(),
            " Query (e=edit) ",
            q_edit,
            true,
            theme.success,
            true,
        ),
        mid[0],
    );

    let v_edit = gql.active_field == GqlField::Variables && state.input_mode == InputMode::Editing;
    f.render_widget(
        render_input(
            gql.variables.as_str(),
            " Variables (v=edit) ",
            v_edit,
            true,
            theme.success,
            true,
        ),
        mid[1],
    );

    let time = if gql.time_ms > 0 {
        format!(" {}ms ", gql.time_ms)
    } else {
        String::new()
    };
    let blk = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title(" Response (s=execute, ↑/↓=scroll) ")
        .title_bottom(Line::from(time).right_aligned());
    let lines = highlight_json(&gql.response);
    f.render_widget(
        Paragraph::new(lines)
            .block(blk)
            .wrap(Wrap { trim: false })
            .scroll((gql.response_scroll, 0)),
        chunks[2],
    );
}
