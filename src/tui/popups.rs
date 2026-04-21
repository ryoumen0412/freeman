//! Popup overlay rendering.

use crate::messages::RenderState;
use crate::tui::chrome::centered_rect;
use crate::tui::theme::Theme;
use ratatui::{prelude::*, widgets::*};

pub fn draw_help_popup(f: &mut Frame, area: Rect) {
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
    let theme = Theme::default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .padding(Padding::horizontal(1))
        .title(" Help ")
        .style(Style::default().bg(Color::Black));
    let help = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);
}

pub fn draw_curl_import_popup(f: &mut Frame, state: &RenderState, area: Rect) {
    let popup_area = centered_rect(80, 30, area);
    let theme = Theme::default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .padding(Padding::horizontal(1))
        .title(" Import cURL (Enter to import, Esc to cancel) ")
        .style(Style::default().bg(Color::Black));
    let content = if state.http.curl_import_buffer.is_empty() {
        "Paste cURL command here..."
    } else {
        &state.http.curl_import_buffer
    };
    let input = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);
}

pub fn draw_workspace_input_popup(f: &mut Frame, state: &RenderState, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    let theme = Theme::default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .padding(Padding::horizontal(1))
        .title(" 📂 Open Workspace (Enter to load, Esc to cancel) ")
        .style(Style::default().bg(Color::Black));
    let content = if state.http.workspace_path_input.is_empty() {
        "Enter project directory path...\n\nExample: ~/projects/my-api"
    } else {
        &state.http.workspace_path_input
    };
    let input = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);
}
