//! UI helper re-exports for backward compatibility.
//!
//! All rendering primitives have been moved to [`crate::tui::widgets`].

pub use crate::tui::widgets::{
    highlight_json, method_color, render_header_list, render_input, render_tabs, status_color,
};
