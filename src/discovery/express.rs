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
    use std::sync::OnceLock;

    static ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();
    static AUTH_REGEX: OnceLock<Regex> = OnceLock::new();
    static CHAIN_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();
    
    // Match Express route definitions
    // app.get('/path', handler) or router.post('/path', middleware, handler)
    let route_pattern = ROUTE_REGEX.get_or_init(|| {
        Regex::new(r#"(?:app|router)\.(get|post|put|patch|delete)\s*\(\s*['"`]([^'"`]+)['"`]"#)
            .expect("Invalid route regex")
    });
    
    // Match auth middleware patterns
    let auth_pattern = AUTH_REGEX.get_or_init(|| {
        Regex::new(r#"(?:authenticate|auth|verifyToken|isAuthenticated|passport\.authenticate|requireAuth|checkAuth|jwt)"#)
            .expect("Invalid auth regex")
    });
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            let method = match caps.get(1) {
                Some(m) => m.as_str().to_uppercase(),
                None => continue,
            };
            let path = match caps.get(2) {
                Some(p) => p.as_str().to_string(),
                None => continue,
            };
            
            // Check if line contains auth middleware
            let has_auth = auth_pattern.is_match(line);
            
            let mut endpoint = DiscoveredEndpoint::new(&method, &path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            endpoint.auth = if has_auth { AuthRequirement::Bearer } else { AuthRequirement::None };
            
            endpoints.push(endpoint);
        }
    }
    
    // Also look for router.route('/path').get().post() pattern
    let chain_pattern = CHAIN_REGEX.get_or_init(|| {
        Regex::new(r#"\.route\s*\(\s*['"`]([^'"`]+)['"`]\s*\)\s*\.(get|post|put|patch|delete)"#)
            .expect("Invalid chain regex")
    });
    
    for (line_num, line) in content.lines().enumerate() {
        for caps in chain_pattern.captures_iter(line) {
            let path = match caps.get(1) {
                Some(p) => p.as_str().to_string(),
                None => continue,
            };
            let method = match caps.get(2) {
                Some(m) => m.as_str().to_uppercase(),
                None => continue,
            };
            
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
    use std::sync::OnceLock;

    static PORT_RE_1: OnceLock<Regex> = OnceLock::new();
    static PORT_RE_2: OnceLock<Regex> = OnceLock::new();

    let regexes = [
        PORT_RE_1.get_or_init(|| Regex::new(r#"(?:PORT|port)\s*[=:]\s*['"]?(\d+)['"]?"#).unwrap()),
        PORT_RE_2.get_or_init(|| Regex::new(r#"listen\s*\(\s*(\d+)"#).unwrap()),
    ];
    
    for re in regexes {
        if let Some(caps) = re.captures(content) {
            return Some(caps.get(1)?.as_str().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_express_routes() {
        let content = r#"
            const express = require('express');
            const app = express();
            
            app.get('/users', (req, res) => {});
            app.post('/api/data', auth, (req, res) => {});
            
            router.put('/items/:id', (req, res) => {});
            
            app.route('/chain').get((req, res) => {});
            app.route('/chain').post((req, res) => {});
        "#;
        
        let path = PathBuf::from("test.js");
        let endpoints = parse_express_file(content, &path);
        
        assert_eq!(endpoints.len(), 5);
        
        let get_users = endpoints.iter().find(|e| e.path == "/users").unwrap();
        assert_eq!(get_users.method, "GET");
        assert!(matches!(get_users.auth, AuthRequirement::None));
        
        let post_data = endpoints.iter().find(|e| e.path == "/api/data").unwrap();
        assert_eq!(post_data.method, "POST");
        assert!(matches!(post_data.auth, AuthRequirement::Bearer));
        
        let put_items = endpoints.iter().find(|e| e.path == "/items/:id").unwrap();
        assert_eq!(put_items.method, "PUT");
        
        let chain_get = endpoints.iter().find(|e| e.path == "/chain" && e.method == "GET").unwrap();
        let chain_post = endpoints.iter().find(|e| e.path == "/chain" && e.method == "POST").unwrap();
        assert!(chain_get.line_number.is_some());
        assert!(chain_post.line_number.is_some());
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(extract_port("const PORT = 3000;"), Some("3000".to_string()));
        assert_eq!(extract_port("app.listen(8080)"), Some("8080".to_string()));
        assert_eq!(extract_port("server.listen(  5000  )"), Some("5000".to_string()));
        assert_eq!(extract_port("val port = process.env.PORT || 3001"), None); // Too complex for simple regex
    }
}
