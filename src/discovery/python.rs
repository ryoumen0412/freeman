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
                Framework::Django => parse_django_file(&content, &file_path),
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
    use std::sync::OnceLock;
    static ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();
    static AUTH_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();
    
    // Match FastAPI route decorators
    let route_pattern = ROUTE_REGEX.get_or_init(|| {
        Regex::new(r#"@(?:app|router)\.(get|post|put|patch|delete)\s*\(\s*["']([^"']+)["']"#)
            .expect("Invalid route regex")
    });
    
    // Match Depends for auth detection
    let auth_pattern = AUTH_REGEX.get_or_init(|| {
        Regex::new(r#"Depends\s*\(\s*(get_current_user|oauth2_scheme|HTTPBearer|verify_token)"#)
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
    use std::sync::OnceLock;
    static ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();
    static AUTH_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();
    
    // Match Flask route decorators
    let route_pattern = ROUTE_REGEX.get_or_init(|| {
        Regex::new(r#"@(?:app|blueprint|\w+)\.route\s*\(\s*["']([^"']+)["'](?:.*?methods\s*=\s*\[([^\]]+)\])?"#)
            .expect("Invalid route regex")
    });
    
    // Match login_required decorator
    let auth_pattern = AUTH_REGEX.get_or_init(|| {
        Regex::new(r#"@(?:login_required|jwt_required|token_required|auth\.login_required)"#)
            .expect("Invalid auth regex")
    });
    
    let lines: Vec<&str> = content.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        if let Some(caps) = route_pattern.captures(line) {
            let path = match caps.get(1) {
                Some(p) => p.as_str().to_string(),
                None => continue,
            };
            
            // Extract methods or default to GET
            let methods: Vec<String> = if let Some(methods_match) = caps.get(2) {
                methods_match.as_str()
                    .split(',')
                    .map(|m| m.trim().trim_matches(|c| c == '"' || c == '\'').to_uppercase())
                    .collect()
            } else {
                vec!["GET".to_string()]
            };
            
            // Check for auth decorator in next few lines (standard Flask order is @route then @login_required)
            // or previous lines (if @login_required is first)
            let start_check = line_num.saturating_sub(2);
            let end_check = (line_num + 5).min(lines.len());
            
            let has_auth = lines[start_check..end_check]
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
    use std::sync::OnceLock;
    static BASE_URL_RE: OnceLock<Regex> = OnceLock::new();
    static HOST_RE: OnceLock<Regex> = OnceLock::new();

    let regexes = [
        BASE_URL_RE.get_or_init(|| Regex::new(r#"(?:BASE_URL|API_URL|SERVER_URL)\s*=\s*["']([^"']+)["']"#).unwrap()),
        HOST_RE.get_or_init(|| Regex::new(r#"(?:host|HOST)\s*=\s*["']([^"']+)["']"#).unwrap()),
    ];
    
    for re in regexes {
        if let Some(caps) = re.captures(content) {
            return Some(caps.get(1)?.as_str().to_string());
        }
    }
    None
}

/// Parse Django urls.py and views
fn parse_django_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    use std::sync::OnceLock;
    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    static DECORATOR_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();
    
    // Match django.urls.path('route', ...)
    let url_pattern = URL_REGEX.get_or_init(|| {
        Regex::new(r#"(?:path|re_path)\s*\(\s*['"]([^'"]+)['"]"#).unwrap()
    });
    
    // Match DRF @api_view(['GET', 'POST'])
    let decorator_pattern = DECORATOR_REGEX.get_or_init(|| {
        Regex::new(r#"@api_view\s*\(\s*\[(.*?)]\s*\)"#).unwrap()
    });
    
    // If it's a urls.py file, parse routes
    if file_path.to_string_lossy().contains("urls.py") {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = url_pattern.captures(line) {
                if let Some(path) = caps.get(1) {
                    let mut endpoint = DiscoveredEndpoint::new("ANY", path.as_str());
                    endpoint.source_file = Some(file_path.to_path_buf());
                    endpoint.line_number = Some(line_num + 1);
                     endpoints.push(endpoint);
                }
            }
        }
    } 
    // If it's a views file, check for DRF decorators
    else {
         for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = decorator_pattern.captures(line) {
                if let Some(methods_str) = caps.get(1) {
                    let methods: Vec<String> = methods_str.as_str()
                        .split(',')
                        .map(|m| m.trim().trim_matches(|c| c == '"' || c == '\'').to_uppercase())
                        .collect();
                    
                    for method in methods {
                        let mut endpoint = DiscoveredEndpoint::new(&method, "/???"); 
                        endpoint.source_file = Some(file_path.to_path_buf());
                        endpoint.line_number = Some(line_num + 1);
                        endpoint.description = Some("Detected DRF view".to_string());
                        endpoints.push(endpoint);
                    }
                }
            }
        }
    }
    
    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_fastapi() {
        let content = r#"
            @app.get("/users")
            def get_users(): pass

            @router.post("/items/{id}")
            def create_item(id: int): pass

            @app.put("/secure", dependencies=[Depends(get_current_user)])
            def secure_endpoint(): pass
        "#;
        
        let path = PathBuf::from("main.py");
        let endpoints = parse_fastapi_file(content, &path);
        
        assert_eq!(endpoints.len(), 3);
        
        let get_users = endpoints.iter().find(|e| e.path == "/users").unwrap();
        assert_eq!(get_users.method, "GET");
        
        let post_item = endpoints.iter().find(|e| e.path == "/items/{id}").unwrap();
        assert_eq!(post_item.method, "POST");
        
        let secure = endpoints.iter().find(|e| e.path == "/secure").unwrap();
        assert_eq!(secure.method, "PUT");
        // Note: Our regex looks for 'Depends ( ... )' but the sample has 'dependencies=[Depends(...)]'
        // The regex `Depends\s*\(\s*...` matches `Depends(get_current_user` inside that string
        assert!(matches!(secure.auth, AuthRequirement::Bearer));
    }

    #[test]
    fn test_parse_flask() {
        let content = r#"
            @app.route("/hello", methods=["GET"])
            def hello(): return "Hello"

            @app.route("/submit", methods=["POST", "PUT"])
            @login_required
            def submit(): return "OK"
        "#;
        
        let path = PathBuf::from("app.py");
        let endpoints = parse_flask_file(content, &path);
        
        assert_eq!(endpoints.len(), 3); // GET hello, POST submit, PUT submit
        
        let hello = endpoints.iter().find(|e| e.path == "/hello").unwrap();
        assert_eq!(hello.method, "GET");
        
        let post_submit = endpoints.iter().find(|e| e.path == "/submit" && e.method == "POST").unwrap();
        assert!(matches!(post_submit.auth, AuthRequirement::Bearer));
    }

    #[test]
    fn test_extract_base_url() {
        assert_eq!(extract_base_url("BASE_URL = 'https://api.test.com'"), Some("https://api.test.com".to_string()));
        assert_eq!(extract_base_url("HOST = \"http://localhost:8000\""), Some("http://localhost:8000".to_string()));
    }
}
