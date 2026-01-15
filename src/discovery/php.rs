//! Laravel source code parser (PHP)

use crate::discovery::models::{AuthRequirement, DiscoveredEndpoint, Framework, WorkspaceProject};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub fn parse_php_routes(project_root: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();

    // Check main route files
    for route_file in ["routes/api.php", "routes/web.php"] {
        let path = project_root.join(route_file);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                endpoints.extend(parse_laravel_file(&content, &path));
            }
        }
    }

    endpoints
}

fn parse_laravel_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static ROUTE_RE: OnceLock<Regex> = OnceLock::new();

    // Route::get(...) or ->get(...)
    let route_re = ROUTE_RE.get_or_init(|| {
        Regex::new(r#"(?:Route::|->)(get|post|put|patch|delete|any)\s*\(\s*['"]([^'"]+)['"]"#)
            .unwrap()
    });

    let mut endpoints = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = route_re.captures(line) {
            let method = caps
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or("GET".to_string());
            let path = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let mut endpoint = DiscoveredEndpoint::new(&method, path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);

            // Basic auth detection
            if line.contains("auth:sanctum") || line.contains("auth:api") {
                endpoint.auth = AuthRequirement::Bearer;
            }

            endpoints.push(endpoint);
        }
    }

    endpoints
}

pub fn load_laravel_project(project_root: &Path) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = Framework::Laravel;
    project.endpoints = parse_php_routes(project_root);
    project.base_url = Some("http://localhost:8000".to_string());
    project
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_laravel() {
        let content = r#"
            Route::get('/user', function (Request $request) {
                return $request->user();
            });
            Route::post('login', [AuthController::class, 'login']);
            Route::middleware('auth:sanctum')->get('/profile', [ProfileController::class, 'show']);
        "#;

        let endpoints = parse_laravel_file(content, Path::new("routes/api.php"));
        assert_eq!(endpoints.len(), 3);

        let get_user = endpoints.iter().find(|e| e.path == "/user").unwrap();
        assert_eq!(get_user.method, "GET");

        let login = endpoints.iter().find(|e| e.path == "login").unwrap();
        assert_eq!(login.method, "POST");

        let profile = endpoints.iter().find(|e| e.path == "/profile").unwrap();
        assert_eq!(profile.method, "GET");
        assert!(matches!(profile.auth, AuthRequirement::Bearer));
    }
}
