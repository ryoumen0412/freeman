//! TUI rendering layer - all drawing functions organized by component.
//!
//! This module contains pure rendering functions that read [`RenderState`]
//! and draw widgets to a [`Frame`]. No state mutation occurs here.

pub mod chrome;
pub mod draw;
pub mod gql_tab;
pub mod http_tab;
pub mod popups;
pub mod theme;
pub mod widgets;
pub mod ws_tab;
