//! Workspace discovery module - detects API endpoints from project source code

pub mod models;
pub mod detector;
pub mod openapi;
pub mod python;
pub mod express;
pub mod java;
pub mod php;
pub mod nestjs;


pub use models::*;
pub use python::load_python_project;
pub use express::load_express_project;
pub use java::load_java_project;
pub use php::load_laravel_project;
pub use nestjs::load_nestjs_project;
