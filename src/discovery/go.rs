//! Go (Gin) source code parser for route discovery

use crate::discovery::models::{DiscoveredEndpoint, Framework, WorkspaceProject};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Load a Go project and extract discovered endpoints
pub fn load_go_project(project_root: &Path, framework: Framework) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = framework.clone();
    project.title = Some(
        project_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Go API".to_string()),
    );
    project.base_url = Some("http://localhost:8080".to_string());
    project.endpoints = parse_go_routes(project_root, framework);
    project
}

/// Parse Go source files for route definitions
pub fn parse_go_routes(project_root: &Path, framework: Framework) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    let go_files = find_go_files(project_root);

    for file_path in go_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let file_endpoints = match framework {
                Framework::Gin => parse_gin_file(&content, &file_path),
                _ => Vec::new(),
            };
            endpoints.extend(file_endpoints);
        }
    }

    endpoints
}

fn find_go_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(name, "vendor" | "node_modules" | ".git" | ".venv" | "venv") {
                    files.extend(find_go_files(&path));
                }
            } else if path.extension().map(|e| e == "go").unwrap_or(false) {
                files.push(path);
            }
        }
    }

    files
}

/// Parse Gin framework routes and route groups: r.GET("/path"), v1.POST("/path")
fn parse_gin_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static GROUP_REGEX: OnceLock<Regex> = OnceLock::new();
    static ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();
    let mut groups: HashMap<String, String> = HashMap::new();

    let group_pattern = GROUP_REGEX.get_or_init(|| {
        Regex::new(r#"([a-zA-Z0-9_]+)\s*:=\s*.*\.Group\s*\(\s*["']([^"']+)["']\s*\)"#)
            .expect("Invalid Gin group regex")
    });

    let route_pattern = ROUTE_REGEX.get_or_init(|| {
        Regex::new(r#"([a-zA-Z0-9_]+)\.(GET|POST|PUT|PATCH|DELETE)\s*\(\s*["']([^"']+)["']"#)
            .expect("Invalid Gin route regex")
    });

    for (line_num, line) in content.lines().enumerate() {
        // Detect route groups
        if let Some(caps) = group_pattern.captures(line) {
            if let (Some(var_name), Some(prefix)) = (caps.get(1), caps.get(2)) {
                groups.insert(var_name.as_str().to_string(), prefix.as_str().to_string());
            }
        }

        // Detect routes
        if let Some(caps) = route_pattern.captures(line) {
            if let (Some(router_var), Some(method), Some(path)) =
                (caps.get(1), caps.get(2), caps.get(3))
            {
                let var_name = router_var.as_str();
                let route_path = path.as_str();

                // Combine with group prefix if available
                let full_path = if let Some(prefix) = groups.get(var_name) {
                    format!("{}{}", prefix.trim_end_matches('/'), route_path)
                } else {
                    route_path.to_string()
                };

                let mut endpoint =
                    DiscoveredEndpoint::new(method.as_str().to_uppercase(), full_path);
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
    fn test_parse_gin_routes() {
        let code = r#"
            r := gin.Default()
            r.GET("/ping", pingHandler)

            v1 := r.Group("/api/v1")
            v1.GET("/users", getUsers)
            v1.POST("/users", createUser)
        "#;

        let path = Path::new("main.go");
        let endpoints = parse_gin_file(code, path);
        assert_eq!(endpoints.len(), 3);
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].path, "/ping");
        assert_eq!(endpoints[1].method, "GET");
        assert_eq!(endpoints[1].path, "/api/v1/users");
        assert_eq!(endpoints[2].method, "POST");
        assert_eq!(endpoints[2].path, "/api/v1/users");
    }
}
