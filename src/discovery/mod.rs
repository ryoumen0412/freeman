//! Workspace discovery module - detects API endpoints from project source code

pub mod models;
pub mod detector;
pub mod openapi;
pub mod python;
pub mod express;

pub use models::*;
pub use python::load_python_project;
pub use express::load_express_project;
