//! App layer - central state management and command processing
//!
//! The App actor receives UI events and network responses,
//! updates state, and emits network commands and render state.

pub mod actor;
pub mod commands;
pub mod gql_commands;
pub mod http_commands;
pub mod state;
pub mod workspace_commands;
pub mod ws_commands;

pub use actor::AppActor;
pub use state::AppState;
