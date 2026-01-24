use ratatui::{prelude::*, widgets::*};

/// Renders a text input field with cursor
#[allow(dead_code)] // Prepared for future dynamic input rendering
pub fn render_input<'a>(
    content: &'a str,
    title: &'a str,
    is_focused: bool,
    _cursor_pos: Option<usize>,
) -> Paragraph<'a> {
    let style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title);

    Paragraph::new(content).block(block)
}

/// Renders a key-value list (for headers)
#[allow(dead_code)] // Prepared for future header list rendering
pub fn render_key_value_list<'a>(
    items: &'a [(String, String, bool)], // key, value, enabled
    title: &'a str,
    selected: Option<usize>,
    is_focused: bool,
) -> List<'a> {
    let items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (key, value, enabled))| {
            let style = if !enabled {
                Style::default().fg(Color::DarkGray)
            } else if Some(i) == selected {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };

            let prefix = if *enabled { "[x]" } else { "[ ]" };
            ListItem::new(format!("{} {}: {}", prefix, key, value)).style(style)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
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

/// Renders tabs
pub fn render_tabs<'a>(titles: &[&'a str], selected: usize) -> Tabs<'a> {
    let titles: Vec<Line> = titles.iter().map(|t| Line::from(*t)).collect();

    Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).bold())
        .divider("|")
}

/// Simple JSON syntax highlighting
#[allow(unused_mut, dead_code)]
pub fn highlight_json(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line in text.lines() {
        let mut spans = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current = String::new();
        let mut in_string = false;
        let mut is_key = false;

        for c in chars {
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

/// Status code color
pub fn status_color(code: u16) -> Color {
    match code {
        200..=299 => Color::Green,
        300..=399 => Color::Cyan,
        400..=499 => Color::Red,
        500..=599 => Color::Magenta,
        _ => Color::Yellow,
    }
}

/// Method color
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
