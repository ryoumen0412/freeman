//! Reusable UI widgets and utility functions.
//!
//! Contains shared rendering primitives used across all tabs:
//! text inputs, header lists, tab bars, JSON highlighting, and color helpers.

use ratatui::{prelude::*, widgets::*};

use crate::models::Header;

// ============================================================================
// Input Widget
// ============================================================================

/// Renders a reusable text input field with editing/focus state styling.
///
/// # Arguments
/// * `content` - The text to display in the input
/// * `title` - Block title (e.g. " URL ", " Query (e=edit) ")
/// * `is_editing` - Whether the field is in edit mode (yellow border)
/// * `is_focused` - Whether the field has focus (uses `base_color`)
/// * `base_color` - The accent color when focused but not editing
/// * `wrap` - Whether to enable text wrapping
pub fn render_input<'a>(
    content: &'a str,
    title: &'a str,
    is_editing: bool,
    is_focused: bool,
    base_color: Color,
    wrap: bool,
) -> Paragraph<'a> {
    let border_style = if is_editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(base_color)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let p = Paragraph::new(content).block(block);
    if wrap {
        p.wrap(Wrap { trim: false })
    } else {
        p
    }
}

// ============================================================================
// Header List Widget
// ============================================================================

/// Renders a key-value list widget for HTTP headers.
///
/// Displays each header as `[x] Key: Value` with styling based on
/// enabled state and selection.
pub fn render_header_list<'a>(
    headers: &'a [Header],
    title: &'a str,
    selected: usize,
    is_focused: bool,
) -> List<'a> {
    let items: Vec<ListItem> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let style = if !h.enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_focused && i == selected {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if h.enabled { "[x]" } else { "[ ]" };
            ListItem::new(format!("{} {}: {}", prefix, h.key, h.value)).style(style)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    )
}

// ============================================================================
// Tab Bar Widget
// ============================================================================

/// Renders a tab bar with highlighted selection.
pub fn render_tabs<'a>(titles: &[&'a str], selected: usize) -> Tabs<'a> {
    let titles: Vec<Line> = titles.iter().map(|t| Line::from(*t)).collect();

    Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).bold())
        .divider("|")
}

// ============================================================================
// JSON Syntax Highlighting
// ============================================================================

/// Simple JSON syntax highlighting for response bodies.
///
/// Colors: keys=Cyan, strings=Green, numbers=Yellow, booleans/null=Magenta,
/// braces/brackets=Yellow, colons=White.
#[allow(unused_mut)]
pub fn highlight_json(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line in text.lines() {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut is_key = false;

        for c in line.chars() {
            match c {
                '"' => {
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }

                    if in_string {
                        // End of string
                        current.push(c);
                        let color = if is_key { Color::Cyan } else { Color::Green };
                        spans.push(Span::styled(current.clone(), Style::default().fg(color)));
                        current.clear();
                        in_string = false;
                        is_key = false;
                    } else {
                        // Start of string
                        in_string = true;
                        current.push(c);
                        // Check if this is a key (followed by :)
                        is_key = line[line.find('"').unwrap_or(0)..].contains("\":");
                    }
                }
                ':' if !in_string => {
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    spans.push(Span::styled(":", Style::default().fg(Color::White)));
                }
                '{' | '}' | '[' | ']' if !in_string => {
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                '0'..='9' | '-' | '.' if !in_string => {
                    if !current.is_empty()
                        && !current
                            .chars()
                            .all(|x| x.is_ascii_digit() || x == '-' || x == '.')
                    {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    current.push(c);
                }
                't' | 'r' | 'u' | 'e' | 'f' | 'a' | 'l' | 's' | 'n' if !in_string => {
                    current.push(c);
                    // Check for true, false, null
                    if current == "true" || current == "false" || current == "null" {
                        spans.push(Span::styled(
                            current.clone(),
                            Style::default().fg(Color::Magenta),
                        ));
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            // Color numbers
            if current
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
            {
                spans.push(Span::styled(current, Style::default().fg(Color::Yellow)));
            } else {
                spans.push(Span::raw(current));
            }
        }

        lines.push(Line::from(spans));
    }

    lines
}

// ============================================================================
// Color Helpers
// ============================================================================

/// Returns the display color for an HTTP status code.
pub fn status_color(code: u16) -> Color {
    match code {
        200..=299 => Color::Green,
        300..=399 => Color::Cyan,
        400..=499 => Color::Red,
        500..=599 => Color::Magenta,
        _ => Color::Yellow,
    }
}

/// Returns the display color for an HTTP method.
pub fn method_color(method: &str) -> Color {
    match method {
        "GET" => Color::Green,
        "POST" => Color::Yellow,
        "PUT" => Color::Blue,
        "PATCH" => Color::Cyan,
        "DELETE" => Color::Red,
        _ => Color::White,
    }
}
