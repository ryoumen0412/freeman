//! App layer - central state management and command processing
//!
//! The App actor receives UI events and network responses,
//! updates state, and emits network commands and render state.

pub mod state;
pub mod actor;
pub mod commands;

pub use state::AppState;
pub use actor::AppActor;
