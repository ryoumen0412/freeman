//! Django URL parser - extracts endpoints from Django and Django REST Framework projects

use crate::discovery::models::{AuthRequirement, DiscoveredEndpoint, Framework, WorkspaceProject};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Parse Django project for URL patterns and DRF viewsets
pub fn parse_django_routes(project_root: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();

    // Find Python files
    let python_files = find_python_files(project_root);

    for file_path in python_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Parse urls.py files for urlpatterns
            if file_name == "urls.py" {
                let url_endpoints = parse_urls_file(&content, &file_path);
                endpoints.extend(url_endpoints);
            }

            // Parse views files for DRF decorators
            if file_name.contains("view")
                || content.contains("@api_view")
                || content.contains("APIView")
            {
                let view_endpoints = parse_drf_views_file(&content, &file_path);
                endpoints.extend(view_endpoints);
            }
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
                if !matches!(
                    name,
                    "venv"
                        | ".venv"
                        | "__pycache__"
                        | "node_modules"
                        | ".git"
                        | ".mypy_cache"
                        | "migrations"
                ) {
                    files.extend(find_python_files(&path));
                }
            } else if path.extension().map(|e| e == "py").unwrap_or(false) {
                files.push(path);
            }
        }
    }

    files
}

/// Parse Django urls.py file for urlpatterns
///
/// Handles patterns like:
/// - path('users/', views.UserListView.as_view(), name='user-list')
/// - path('api/', include('api.urls'))
/// - re_path(r'^items/(?P<id>\d+)/$', views.item_detail)
fn parse_urls_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    use std::sync::OnceLock;
    static PATH_REGEX: OnceLock<Regex> = OnceLock::new();
    static REPATH_REGEX: OnceLock<Regex> = OnceLock::new();
    static ROUTER_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();

    // Match path('route', ...) - standard Django
    let path_pattern =
        PATH_REGEX.get_or_init(|| Regex::new(r#"path\s*\(\s*['\"]([^'\"]*)['\"]"#).unwrap());

    // Match re_path(r'^route$', ...) - regex patterns
    let repath_pattern =
        REPATH_REGEX.get_or_init(|| Regex::new(r#"re_path\s*\(\s*r?['\"]([^'\"]+)['\"]"#).unwrap());

    // Match DRF router.register('prefix', ViewSet)
    let router_pattern = ROUTER_REGEX
        .get_or_init(|| Regex::new(r#"router\.register\s*\(\s*r?['\"]([^'\"]+)['\"]"#).unwrap());

    for (line_num, line) in content.lines().enumerate() {
        // Skip include() patterns - they just reference other url files
        if line.contains("include(") {
            continue;
        }

        // Parse path() patterns
        if let Some(caps) = path_pattern.captures(line) {
            if let Some(path_match) = caps.get(1) {
                let path_str = path_match.as_str();
                // Skip empty paths (usually root includes)
                if !path_str.is_empty() {
                    let normalized_path = normalize_django_path(path_str);
                    let auth = detect_auth_in_line(line);

                    // Django paths don't specify methods - they're determined by the view
                    // We'll create endpoints for common CRUD methods
                    let methods = infer_methods_from_context(line, &normalized_path);

                    for method in methods {
                        let mut endpoint = DiscoveredEndpoint::new(&method, &normalized_path);
                        endpoint.source_file = Some(file_path.to_path_buf());
                        endpoint.line_number = Some(line_num + 1);
                        endpoint.auth = auth.clone();
                        endpoints.push(endpoint);
                    }
                }
            }
        }

        // Parse re_path() patterns
        if let Some(caps) = repath_pattern.captures(line) {
            if let Some(path_match) = caps.get(1) {
                let regex_path = path_match.as_str();
                let normalized_path = normalize_regex_path(regex_path);
                let auth = detect_auth_in_line(line);

                let methods = infer_methods_from_context(line, &normalized_path);

                for method in methods {
                    let mut endpoint = DiscoveredEndpoint::new(&method, &normalized_path);
                    endpoint.source_file = Some(file_path.to_path_buf());
                    endpoint.line_number = Some(line_num + 1);
                    endpoint.auth = auth.clone();
                    endpoints.push(endpoint);
                }
            }
        }

        // Parse DRF router.register() - these create CRUD endpoints
        if let Some(caps) = router_pattern.captures(line) {
            if let Some(prefix_match) = caps.get(1) {
                let prefix = prefix_match.as_str();
                let base_path = format!("/{}/", prefix.trim_matches('/'));
                let detail_path = format!("/{}/:id/", prefix.trim_matches('/'));

                // ViewSets create standard CRUD routes
                for (method, path) in [
                    ("GET", base_path.as_str()),      // list
                    ("POST", base_path.as_str()),     // create
                    ("GET", detail_path.as_str()),    // retrieve
                    ("PUT", detail_path.as_str()),    // update
                    ("PATCH", detail_path.as_str()),  // partial_update
                    ("DELETE", detail_path.as_str()), // destroy
                ] {
                    let mut endpoint = DiscoveredEndpoint::new(method, path);
                    endpoint.source_file = Some(file_path.to_path_buf());
                    endpoint.line_number = Some(line_num + 1);
                    endpoint.description = Some("DRF ViewSet".to_string());
                    endpoints.push(endpoint);
                }
            }
        }
    }

    endpoints
}

/// Parse DRF views for @api_view decorators and APIView classes
fn parse_drf_views_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    use std::sync::OnceLock;
    static API_VIEW_REGEX: OnceLock<Regex> = OnceLock::new();
    static CLASS_VIEW_REGEX: OnceLock<Regex> = OnceLock::new();
    static PERMISSION_REGEX: OnceLock<Regex> = OnceLock::new();

    let mut endpoints = Vec::new();

    // Match @api_view(['GET', 'POST'])
    let api_view_pattern =
        API_VIEW_REGEX.get_or_init(|| Regex::new(r#"@api_view\s*\(\s*\[([^\]]+)\]"#).unwrap());

    // Match class SomeView(APIView): or class SomeViewSet(ViewSet):
    let class_view_pattern = CLASS_VIEW_REGEX.get_or_init(|| {
        Regex::new(
            r#"class\s+(\w+)\s*\([^)]*(?:APIView|ViewSet|ModelViewSet|GenericAPIView)[^)]*\)"#,
        )
        .unwrap()
    });

    // Match permission classes for auth detection
    let permission_pattern = PERMISSION_REGEX
        .get_or_init(|| Regex::new(r#"permission_classes\s*=\s*\[([^\]]+)\]"#).unwrap());

    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Parse @api_view decorator
        if let Some(caps) = api_view_pattern.captures(line) {
            if let Some(methods_match) = caps.get(1) {
                let methods: Vec<String> = methods_match
                    .as_str()
                    .split(',')
                    .map(|m| {
                        m.trim()
                            .trim_matches(|c| c == '"' || c == '\'')
                            .to_uppercase()
                    })
                    .filter(|m| !m.is_empty())
                    .collect();

                // Look for function name in next lines
                let func_name = find_function_name(&lines, line_num);

                // Check for auth requirements in surrounding lines
                let context_start = line_num.saturating_sub(3);
                let context_end = (line_num + 10).min(lines.len());
                let context: String = lines[context_start..context_end].join("\n");

                let auth = if permission_pattern.is_match(&context)
                    && (context.contains("IsAuthenticated") || context.contains("IsAdminUser"))
                {
                    AuthRequirement::Bearer
                } else {
                    AuthRequirement::None
                };

                for method in methods {
                    // Path is unknown from views alone - use function name as hint
                    let path = format!("/{}???", func_name.as_deref().unwrap_or("endpoint"));
                    let mut endpoint = DiscoveredEndpoint::new(&method, &path);
                    endpoint.source_file = Some(file_path.to_path_buf());
                    endpoint.line_number = Some(line_num + 1);
                    endpoint.description = Some(format!(
                        "DRF view: {}",
                        func_name.as_deref().unwrap_or("unknown")
                    ));
                    endpoint.auth = auth.clone();
                    endpoints.push(endpoint);
                }
            }
        }

        // Parse APIView/ViewSet classes
        if let Some(caps) = class_view_pattern.captures(line) {
            if let Some(class_name) = caps.get(1) {
                let name = class_name.as_str();

                // Look for defined methods in the class
                let class_methods = find_class_http_methods(&lines, line_num);

                // Check for permission classes
                let context_end = (line_num + 30).min(lines.len());
                let class_context: String = lines[line_num..context_end].join("\n");

                let auth = if permission_pattern.is_match(&class_context)
                    && (class_context.contains("IsAuthenticated")
                        || class_context.contains("IsAdminUser"))
                {
                    AuthRequirement::Bearer
                } else {
                    AuthRequirement::None
                };

                for method in class_methods {
                    let path = format!(
                        "/{}???",
                        name.to_lowercase().replace("view", "").replace("set", "")
                    );
                    let mut endpoint = DiscoveredEndpoint::new(&method, &path);
                    endpoint.source_file = Some(file_path.to_path_buf());
                    endpoint.line_number = Some(line_num + 1);
                    endpoint.description = Some(format!("DRF class: {}", name));
                    endpoint.auth = auth.clone();
                    endpoints.push(endpoint);
                }
            }
        }
    }

    endpoints
}

/// Normalize Django path to standard URL format
fn normalize_django_path(path: &str) -> String {
    let mut result = path.to_string();

    // Ensure leading slash
    if !result.starts_with('/') {
        result = format!("/{}", result);
    }

    // Convert Django path params <param> to :param
    let param_re = Regex::new(r"<(?:int:|str:|slug:|uuid:|path:)?(\w+)>").unwrap();
    result = param_re.replace_all(&result, ":$1").to_string();

    result
}

/// Convert regex path to readable format
fn normalize_regex_path(regex_path: &str) -> String {
    let mut result = regex_path.to_string();

    // Remove regex anchors
    result = result
        .trim_start_matches('^')
        .trim_end_matches('$')
        .to_string();

    // Convert named groups (?P<name>pattern) to :name
    let named_group_re = Regex::new(r"\(\?P<(\w+)>[^)]+\)").unwrap();
    result = named_group_re.replace_all(&result, ":$1").to_string();

    // Ensure leading slash
    if !result.starts_with('/') {
        result = format!("/{}", result);
    }

    result
}

/// Detect authentication requirements from line context
fn detect_auth_in_line(line: &str) -> AuthRequirement {
    let lower = line.to_lowercase();
    if lower.contains("login") || lower.contains("auth") || lower.contains("protected") {
        AuthRequirement::Bearer
    } else {
        AuthRequirement::None
    }
}

/// Infer HTTP methods from URL pattern and view name context
fn infer_methods_from_context(line: &str, path: &str) -> Vec<String> {
    let lower = line.to_lowercase();

    // Check for specific view patterns
    if lower.contains("create") {
        return vec!["POST".to_string()];
    }
    if lower.contains("update") {
        return vec!["PUT".to_string(), "PATCH".to_string()];
    }
    if lower.contains("delete") || lower.contains("destroy") {
        return vec!["DELETE".to_string()];
    }
    if lower.contains("list") {
        return vec!["GET".to_string()];
    }
    if lower.contains("detail") || lower.contains("retrieve") {
        return vec!["GET".to_string()];
    }

    // Check path patterns
    if path.contains(":") || path.contains("<") {
        // Detail view - likely GET, PUT, PATCH, DELETE
        return vec!["GET".to_string()];
    }

    // Default to GET for unknown patterns
    vec!["GET".to_string()]
}

/// Find function name after @api_view decorator
fn find_function_name(lines: &[&str], decorator_line: usize) -> Option<String> {
    let func_re = Regex::new(r"^\s*(?:async\s+)?def\s+(\w+)").unwrap();

    for i in (decorator_line + 1)..lines.len().min(decorator_line + 5) {
        if let Some(caps) = func_re.captures(lines[i]) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }
    None
}

/// Find HTTP method definitions in a class-based view
fn find_class_http_methods(lines: &[&str], class_line: usize) -> Vec<String> {
    let method_re = Regex::new(r"^\s+def\s+(get|post|put|patch|delete|head|options)\s*\(").unwrap();
    let mut methods = Vec::new();

    // Look through class body (next ~50 lines or until next class/function at indent 0)
    for i in (class_line + 1)..lines.len().min(class_line + 50) {
        let line = lines[i];

        // Stop if we hit another top-level definition
        if line.starts_with("class ") || line.starts_with("def ") || line.starts_with("@") {
            break;
        }

        if let Some(caps) = method_re.captures(line) {
            if let Some(method) = caps.get(1) {
                methods.push(method.as_str().to_uppercase());
            }
        }
    }

    // If no methods found but it's a ViewSet, assume CRUD
    if methods.is_empty() {
        methods = vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
        ];
    }

    methods
}

/// Load a Django project
pub fn load_django_project(project_root: &Path) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = Framework::Django;

    project.endpoints = parse_django_routes(project_root);

    // Try to detect base URL from settings
    for settings_file in [
        "settings.py",
        "config/settings.py",
        "config/settings/base.py",
        ".env",
    ] {
        let settings_path = project_root.join(settings_file);
        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Some(url) = extract_django_base_url(&content) {
                project.base_url = Some(url);
                break;
            }
        }
    }

    // Default to Django's default port
    if project.base_url.is_none() {
        project.base_url = Some("http://localhost:8000".to_string());
    }

    project
}

fn extract_django_base_url(content: &str) -> Option<String> {
    use std::sync::OnceLock;
    static URL_RE: OnceLock<Regex> = OnceLock::new();

    let url_pattern = URL_RE.get_or_init(|| {
        Regex::new(r#"(?:SITE_URL|BASE_URL|API_URL|ALLOWED_HOSTS)\s*=\s*['\"\[]?([^'\"'\]\n]+)"#)
            .unwrap()
    });

    if let Some(caps) = url_pattern.captures(content) {
        let value = caps.get(1)?.as_str().trim();
        if value.starts_with("http") {
            return Some(value.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_django_urls() {
        let content = r#"
from django.urls import path
from . import views

urlpatterns = [
    path('users/', views.UserListView.as_view(), name='user-list'),
    path('users/<int:pk>/', views.UserDetailView.as_view(), name='user-detail'),
    path('items/<slug:slug>/', views.item_detail, name='item-detail'),
]
        "#;

        let path = PathBuf::from("urls.py");
        let endpoints = parse_urls_file(content, &path);

        assert!(!endpoints.is_empty());

        let user_list = endpoints.iter().find(|e| e.path == "/users/").unwrap();
        assert_eq!(user_list.method, "GET");

        let user_detail = endpoints.iter().find(|e| e.path == "/users/:pk/").unwrap();
        assert!(user_detail.path.contains(":pk"));
    }

    #[test]
    fn test_parse_drf_api_view() {
        let content = r#"
from rest_framework.decorators import api_view
from rest_framework.response import Response

@api_view(['GET', 'POST'])
def user_list(request):
    return Response([])

@api_view(['GET', 'PUT', 'DELETE'])
def user_detail(request, pk):
    return Response({})
        "#;

        let path = PathBuf::from("views.py");
        let endpoints = parse_drf_views_file(content, &path);

        // Should find 5 endpoints: GET, POST for user_list and GET, PUT, DELETE for user_detail
        assert_eq!(endpoints.len(), 5);

        let get_endpoints: Vec<_> = endpoints.iter().filter(|e| e.method == "GET").collect();
        assert_eq!(get_endpoints.len(), 2);

        let post_endpoint = endpoints.iter().find(|e| e.method == "POST").unwrap();
        assert!(post_endpoint
            .description
            .as_ref()
            .unwrap()
            .contains("user_list"));
    }

    #[test]
    fn test_parse_drf_router() {
        let content = r#"
from rest_framework.routers import DefaultRouter
from .views import UserViewSet, ItemViewSet

router = DefaultRouter()
router.register('users', UserViewSet)
router.register('items', ItemViewSet, basename='item')
        "#;

        let path = PathBuf::from("urls.py");
        let endpoints = parse_urls_file(content, &path);

        // Each router.register creates 6 CRUD endpoints
        assert_eq!(endpoints.len(), 12);

        let user_list = endpoints
            .iter()
            .find(|e| e.path == "/users/" && e.method == "GET")
            .unwrap();
        assert!(user_list.description.as_ref().unwrap().contains("ViewSet"));
    }

    #[test]
    fn test_normalize_django_path() {
        assert_eq!(normalize_django_path("users/"), "/users/");
        assert_eq!(normalize_django_path("users/<int:pk>/"), "/users/:pk/");
        assert_eq!(
            normalize_django_path("items/<slug:slug>/details/"),
            "/items/:slug/details/"
        );
        assert_eq!(normalize_django_path("<uuid:id>/"), "/:id/");
    }

    #[test]
    fn test_normalize_regex_path() {
        assert_eq!(normalize_regex_path("^users/$"), "/users/");
        assert_eq!(normalize_regex_path(r"^items/(?P<id>\d+)/$"), "/items/:id/");
    }
}
