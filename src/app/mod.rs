//! App layer - central state management and command processing
//!
//! The App actor receives UI events and network responses,
//! updates state, and emits network commands and render state.

pub mod state;
pub mod actor;
pub mod commands;
pub mod http_commands;
pub mod ws_commands;
pub mod gql_commands;
pub mod workspace_commands;

pub use state::AppState;
pub use actor::AppActor;
