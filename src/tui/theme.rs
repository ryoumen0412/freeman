//! Design tokens and centralized theme for the TUI.

use ratatui::style::Color;

/// Centralized design tokens for the application.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub border_normal: Color,
    pub border_focus: Color,
    pub border_editing: Color,
    pub text_muted: Color,
    pub text_normal: Color,
    pub error: Color,
    pub success: Color,
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            border_normal: Color::DarkGray,
            border_focus: Color::Cyan,
            border_editing: Color::Yellow,
            text_muted: Color::DarkGray,
            text_normal: Color::Reset,
            error: Color::Red,
            success: Color::Green,
            highlight: Color::Yellow,
        }
    }
}

#[allow(dead_code)]
impl Theme {
    /// Helper to get the correct border color based on focus and editing state.
    pub fn input_border(&self, is_focused: bool, is_editing: bool) -> Color {
        if is_editing {
            self.border_editing
        } else if is_focused {
            self.border_focus
        } else {
            self.border_normal
        }
    }
}
