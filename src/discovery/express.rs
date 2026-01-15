//! Express.js source code parser (JavaScript/TypeScript)

use std::path::Path;
use std::fs;
use regex::Regex;
use crate::discovery::models::{
    AuthRequirement, DiscoveredEndpoint, WorkspaceProject, Framework,
};

/// Parse Express.js source files for route definitions
pub fn parse_express_routes(project_root: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    
    // Find JS/TS files (excluding node_modules)
    let js_files = find_js_files(project_root);
    
    for file_path in js_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let file_endpoints = parse_express_file(&content, &file_path);
            endpoints.extend(file_endpoints);
        }
    }
    
    endpoints
}

fn find_js_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            
            // Skip non-source directories
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(name, "node_modules" | ".git" | "dist" | "build" | "coverage" | ".next") {
                    files.extend(find_js_files(&path));
                }
            } else {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "js" | "ts" | "mjs") {
                    files.push(path);
                }
            }
        }
    }
    
    files
}

/// Parse Express route patterns:
/// - app.get('/users', ...)
/// - router.post('/items/:id', ...)
/// - app.use('/api', router)
fn parse_express_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    
    // Match Express route definitions
    // app.get('/path', handler) or router.post('/path', middleware, handler)
    let route_pattern = Regex::new(
        r#"(?:app|router)\.(get|post|put|patch|delete)\s*\(\s*['"`]([^'"`]+)['"`]"#
    ).unwrap();
    
    // Match auth middleware patterns
    let auth_patterns = [
        r#"(?:authenticate|auth|verifyToken|isAuthenticated|passport\.authenticate|requireAuth|checkAuth|jwt)"#,
    ];
    
    let auth_regex = Regex::new(auth_patterns[0]).unwrap();
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            let method = caps.get(1).unwrap().as_str().to_uppercase();
            let path = caps.get(2).unwrap().as_str().to_string();
            
            // Check if line contains auth middleware
            let has_auth = auth_regex.is_match(line);
            
            let mut endpoint = DiscoveredEndpoint::new(&method, &path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            endpoint.auth = if has_auth { AuthRequirement::Bearer } else { AuthRequirement::None };
            
            endpoints.push(endpoint);
        }
    }
    
    // Also look for router.route('/path').get().post() pattern
    let chain_pattern = Regex::new(
        r#"\.route\s*\(\s*['"`]([^'"`]+)['"`]\s*\)\s*\.(get|post|put|patch|delete)"#
    ).unwrap();
    
    for (line_num, line) in content.lines().enumerate() {
        for caps in chain_pattern.captures_iter(line) {
            let path = caps.get(1).unwrap().as_str().to_string();
            let method = caps.get(2).unwrap().as_str().to_uppercase();
            
            let mut endpoint = DiscoveredEndpoint::new(&method, &path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            
            // Avoid duplicates
            if !endpoints.iter().any(|e| e.method == method && e.path == path) {
                endpoints.push(endpoint);
            }
        }
    }
    
    endpoints
}

/// Load an Express.js project
pub fn load_express_project(project_root: &Path) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = Framework::Express;
    
    project.endpoints = parse_express_routes(project_root);
    
    // Try to detect port/base URL from common patterns
    for config_file in [".env", "config.js", "config.ts", "src/config.js", "src/config.ts"] {
        let config_path = project_root.join(config_file);
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Some(port) = extract_port(&content) {
                project.base_url = Some(format!("http://localhost:{}", port));
                break;
            }
        }
    }
    
    // Default to common Express port
    if project.base_url.is_none() {
        project.base_url = Some("http://localhost:3000".to_string());
    }
    
    project
}

fn extract_port(content: &str) -> Option<String> {
    let patterns = [
        r#"(?:PORT|port)\s*[=:]\s*['"]?(\d+)['"]?"#,
        r#"listen\s*\(\s*(\d+)"#,
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
