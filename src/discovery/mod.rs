//! Workspace discovery module - detects API endpoints from project source code

pub mod detector;
pub mod django;
pub mod express;
pub mod java;
pub mod models;
pub mod nestjs;
pub mod openapi;
pub mod php;
pub mod python;

pub use django::load_django_project;
pub use express::load_express_project;
pub use java::load_java_project;
pub use models::*;
pub use nestjs::load_nestjs_project;
pub use php::load_laravel_project;
pub use python::load_python_project;
