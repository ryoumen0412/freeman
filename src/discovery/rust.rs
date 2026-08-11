//! Rust (Actix Web, Axum) source code parser for route discovery

use crate::discovery::models::{DiscoveredEndpoint, Framework, WorkspaceProject};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Load a Rust project and extract discovered endpoints
pub fn load_rust_project(project_root: &Path, framework: Framework) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = framework.clone();
    project.title = Some(
        project_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Rust API".to_string()),
    );
    project.base_url = Some("http://localhost:8080".to_string());
    project.endpoints = parse_rust_routes(project_root, framework);
    project
}

/// Parse Rust source files for route definitions
pub fn parse_rust_routes(project_root: &Path, framework: Framework) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    let rust_files = find_rust_files(project_root);

    for file_path in rust_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let file_endpoints = match framework {
                Framework::Actix => parse_actix_file(&content, &file_path),
                Framework::Axum => parse_axum_file(&content, &file_path),
                _ => Vec::new(),
            };
            endpoints.extend(file_endpoints);
        }
    }

    endpoints
}

fn find_rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(name, "target" | "node_modules" | ".git" | ".venv" | "venv") {
                    files.extend(find_rust_files(&path));
                }
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }

    files
}

/// Parse Actix Web macro attributes: #[get("/path")], #[post("/path")]
fn parse_actix_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static MACRO_REGEX: OnceLock<Regex> = OnceLock::new();
    static RESOURCE_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();

    let macro_pattern = MACRO_REGEX.get_or_init(|| {
        Regex::new(r#"#\[(get|post|put|patch|delete)\s*\(\s*["']([^"']+)["']\s*\)\]"#)
            .expect("Invalid Actix macro regex")
    });

    let resource_pattern = RESOURCE_REGEX.get_or_init(|| {
        Regex::new(r#"web::resource\s*\(\s*["']([^"']+)["']\s*\)"#)
            .expect("Invalid Actix resource regex")
    });

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = macro_pattern.captures(line) {
            if let (Some(m), Some(p)) = (caps.get(1), caps.get(2)) {
                let mut endpoint = DiscoveredEndpoint::new(m.as_str().to_uppercase(), p.as_str());
                endpoint.source_file = Some(file_path.to_path_buf());
                endpoint.line_number = Some(line_num + 1);
                endpoints.push(endpoint);
            }
        } else if let Some(caps) = resource_pattern.captures(line) {
            if let Some(p) = caps.get(1) {
                let mut endpoint = DiscoveredEndpoint::new("GET", p.as_str());
                endpoint.source_file = Some(file_path.to_path_buf());
                endpoint.line_number = Some(line_num + 1);
                endpoints.push(endpoint);
            }
        }
    }

    endpoints
}

/// Parse Axum route declarations: .route("/path", get(handler))
fn parse_axum_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();

    let route_pattern = ROUTE_REGEX.get_or_init(|| {
        Regex::new(r#"\.route\s*\(\s*["']([^"']+)["']\s*,\s*(get|post|put|patch|delete)\s*\("#)
            .expect("Invalid Axum route regex")
    });

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            if let (Some(p), Some(m)) = (caps.get(1), caps.get(2)) {
                let mut endpoint = DiscoveredEndpoint::new(m.as_str().to_uppercase(), p.as_str());
                endpoint.source_file = Some(file_path.to_path_buf());
                endpoint.line_number = Some(line_num + 1);
                endpoints.push(endpoint);
            }
        }
    }

    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_actix_routes() {
        let code = r#"
            #[get("/api/users")]
            async fn get_users() -> impl Responder { "users" }

            #[post("/api/users")]
            async fn create_user() -> impl Responder { "created" }
        "#;

        let path = Path::new("src/main.rs");
        let endpoints = parse_actix_file(code, path);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].path, "/api/users");
        assert_eq!(endpoints[1].method, "POST");
        assert_eq!(endpoints[1].path, "/api/users");
    }

    #[test]
    fn test_parse_axum_routes() {
        let code = r#"
            let app = Router::new()
                .route("/api/items", get(list_items))
                .route("/api/items", post(create_item));
        "#;

        let path = Path::new("src/main.rs");
        let endpoints = parse_axum_file(code, path);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].path, "/api/items");
        assert_eq!(endpoints[1].method, "POST");
        assert_eq!(endpoints[1].path, "/api/items");
    }
}
