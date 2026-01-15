//! FastAPI and Flask source code parser (Python)

use std::path::Path;
use std::fs;
use regex::Regex;
use crate::discovery::models::{
    AuthRequirement, DiscoveredEndpoint, WorkspaceProject, Framework,
};

/// Parse FastAPI/Flask source files for route definitions
pub fn parse_python_routes(project_root: &Path, framework: Framework) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    
    // Find Python files
    let python_files = find_python_files(project_root);
    
    for file_path in python_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let file_endpoints = match framework {
                Framework::FastAPI => parse_fastapi_file(&content, &file_path),
                Framework::Flask => parse_flask_file(&content, &file_path),
                _ => Vec::new(),
            };
            endpoints.extend(file_endpoints);
        }
    }
    
    endpoints
}

fn find_python_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            
            // Skip common non-source directories
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(name, "venv" | ".venv" | "__pycache__" | "node_modules" | ".git" | ".mypy_cache") {
                    files.extend(find_python_files(&path));
                }
            } else if path.extension().map(|e| e == "py").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    
    files
}

/// Parse FastAPI decorators: @app.get("/path"), @router.post("/path")
fn parse_fastapi_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    
    // Match FastAPI route decorators
    // @app.get("/users"), @router.post("/items/{id}"), etc.
    let route_pattern = Regex::new(
        r#"@(?:app|router)\.(get|post|put|patch|delete)\s*\(\s*["']([^"']+)["']"#
    ).unwrap();
    
    // Match Depends for auth detection
    let auth_pattern = Regex::new(
        r#"Depends\s*\(\s*(get_current_user|oauth2_scheme|HTTPBearer|verify_token)"#
    ).unwrap();
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            let method = caps.get(1).unwrap().as_str().to_uppercase();
            let path = caps.get(2).unwrap().as_str().to_string();
            
            // Look ahead for auth dependencies
            let next_lines: String = content.lines()
                .skip(line_num)
                .take(10)
                .collect::<Vec<_>>()
                .join("\n");
            
            let auth = if auth_pattern.is_match(&next_lines) {
                AuthRequirement::Bearer
            } else {
                AuthRequirement::None
            };
            
            let mut endpoint = DiscoveredEndpoint::new(&method, &path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            endpoint.auth = auth;
            
            endpoints.push(endpoint);
        }
    }
    
    endpoints
}

/// Parse Flask decorators: @app.route("/path", methods=["GET"])
fn parse_flask_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    
    // Match Flask route decorators
    // @app.route("/users", methods=["GET", "POST"])
    // @blueprint.route("/items/<id>")
    let route_pattern = Regex::new(
        r#"@(?:app|blueprint|\w+)\.route\s*\(\s*["']([^"']+)["'](?:.*?methods\s*=\s*\[([^\]]+)\])?"#
    ).unwrap();
    
    // Match login_required decorator
    let auth_pattern = Regex::new(
        r#"@(?:login_required|jwt_required|token_required|auth\.login_required)"#
    ).unwrap();
    
    let lines: Vec<&str> = content.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            let path = caps.get(1).unwrap().as_str().to_string();
            
            // Extract methods or default to GET
            let methods: Vec<String> = if let Some(methods_match) = caps.get(2) {
                methods_match.as_str()
                    .split(',')
                    .map(|m| m.trim().trim_matches(|c| c == '"' || c == '\'').to_uppercase())
                    .collect()
            } else {
                vec!["GET".to_string()]
            };
            
            // Check for auth decorator in previous lines
            let has_auth = line_num > 0 && 
                lines[line_num.saturating_sub(3)..line_num]
                    .iter()
                    .any(|l| auth_pattern.is_match(l));
            
            for method in methods {
                let mut endpoint = DiscoveredEndpoint::new(&method, &path);
                endpoint.source_file = Some(file_path.to_path_buf());
                endpoint.line_number = Some(line_num + 1);
                endpoint.auth = if has_auth { AuthRequirement::Bearer } else { AuthRequirement::None };
                
                endpoints.push(endpoint);
            }
        }
    }
    
    endpoints
}

/// Load a Python project (FastAPI or Flask)
pub fn load_python_project(project_root: &Path, framework: Framework) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = framework.clone();
    
    project.endpoints = parse_python_routes(project_root, framework);
    
    // Try to detect base URL from common config patterns
    for config_file in ["config.py", ".env", "settings.py", "app/config.py"] {
        let config_path = project_root.join(config_file);
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Some(url) = extract_base_url(&content) {
                project.base_url = Some(url);
                break;
            }
        }
    }
    
    project
}

fn extract_base_url(content: &str) -> Option<String> {
    let patterns = [
        r#"(?:BASE_URL|API_URL|SERVER_URL)\s*=\s*["']([^"']+)["']"#,
        r#"(?:host|HOST)\s*=\s*["']([^"']+)["']"#,
    ];
    
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(content) {
                return Some(caps.get(1)?.as_str().to_string());
            }
        }
    }
    None
}
