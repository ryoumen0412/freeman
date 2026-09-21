use crate::app::AppState;
use crate::discovery::{self, detector, openapi, DiscoveredEndpoint};
use crate::messages::ui_events::Panel;
use crate::models::AuthType;
use crate::models::HttpMethod;
use std::path::PathBuf;

impl AppState {
    // ========================
    // Workspace
    // ========================

    pub fn open_workspace_input(&mut self) {
        self.ui.show_workspace_input = true;
    }

    pub fn workspace_path_char(&mut self, c: char) {
        self.workspace.path_input.push(c);
    }

    pub fn workspace_path_backspace(&mut self) {
        self.workspace.path_input.pop();
    }

    pub fn cancel_workspace_input(&mut self) {
        self.ui.show_workspace_input = false;
        self.workspace.path_input.clear();
    }

    pub fn workspace_path_autocomplete(&mut self) {
        use std::fs;

        // Expand ~ to home directory
        let input = if self.workspace.path_input.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                self.workspace.path_input
                    .replacen("~", &home.to_string_lossy(), 1)
            } else {
                return;
            }
        } else {
            self.workspace.path_input.clone()
        };

        let path = PathBuf::from(&input);

        // If it's already a valid directory, try to complete further
        if path.is_dir() && !input.ends_with('/') {
            self.workspace.path_input = format!("{}/", input);
            return;
        }

        // Get parent directory and prefix to match
        let (parent, prefix) = if input.ends_with('/') {
            (PathBuf::from(&input), String::new())
        } else if let Some(parent) = path.parent() {
            let prefix = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), prefix)
        } else {
            return;
        };

        // Read directory and find matches
        if let Ok(entries) = fs::read_dir(&parent) {
            let mut matches: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name.starts_with(&prefix) && !name.starts_with('.'))
                .collect();

            matches.sort();

            if matches.len() == 1 {
                // Single match - complete it
                let completed = parent.join(&matches[0]);
                self.workspace.path_input = format!("{}/", completed.to_string_lossy());
            } else if matches.len() > 1 {
                // Multiple matches - complete common prefix
                if let Some(common) = common_prefix(&matches) {
                    if common.len() > prefix.len() {
                        let completed = parent.join(&common);
                        self.workspace.path_input = completed.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    pub fn load_workspace(&mut self) {
        let path = self.workspace.path_input.clone();

        // Expand ~ to home directory
        let expanded = if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                path.replacen("~", &home.to_string_lossy(), 1)
            } else {
                path.clone()
            }
        } else {
            path.clone()
        };
        let path_buf = PathBuf::from(&expanded);

        // Try to find and parse OpenAPI spec first
        if let Some(spec_path) = detector::find_openapi_spec(&path_buf) {
            match openapi::parse_openapi(&spec_path) {
                Ok(project) => {
                    let count = project.endpoints.len();
                    self.workspace.project = Some(project);
                    self.workspace.selected_endpoint = 0;
                    self.http.response.body = format!("✓ Loaded {} endpoints from OpenAPI spec", count);
                    self.ui.show_workspace_input = false;
                    self.workspace.path_input.clear();
                    return;
                }
                Err(e) => {
                    self.http.response.body = format!("Error parsing OpenAPI: {}", e);
                }
            }
        }

        // Fallback to source code parsing based on detected framework
        let framework = detector::detect_framework(&path_buf);

        let project = match framework {
            discovery::Framework::FastAPI | discovery::Framework::Flask => {
                Some(discovery::load_python_project(&path_buf, framework))
            }
            discovery::Framework::Django => Some(discovery::load_django_project(&path_buf)),
            discovery::Framework::Express => Some(discovery::load_express_project(&path_buf)),
            discovery::Framework::NestJS => Some(discovery::load_nestjs_project(&path_buf)),
            discovery::Framework::SpringBoot => Some(discovery::load_java_project(&path_buf)),
            discovery::Framework::Laravel => Some(discovery::load_laravel_project(&path_buf)),
            discovery::Framework::Actix | discovery::Framework::Axum => {
                Some(discovery::load_rust_project(&path_buf, framework))
            }
            discovery::Framework::Gin => Some(discovery::load_go_project(&path_buf, framework)),
            _ => None,
        };

        if let Some(proj) = project {
            let count = proj.endpoints.len();
            let fw_name = proj.framework.as_str().to_string();
            self.workspace.project = Some(proj);
            self.workspace.selected_endpoint = 0;
            self.http.response.body =
                format!("✓ Loaded {} endpoints from {} source code", count, fw_name);
        } else {
            self.http.response.body = format!(
                "No supported framework detected in {}\n\nSupported: OpenAPI, FastAPI, Flask, Django, Express.js, NestJS, Spring Boot, Laravel, Actix Web, Axum, Gin",
                expanded
            );
        }

        self.ui.show_workspace_input = false;
        self.workspace.path_input.clear();
    }

    pub fn next_endpoint(&mut self) {
        if let Some(ws) = &self.workspace.project {
            if !ws.endpoints.is_empty() {
                self.workspace.selected_endpoint = (self.workspace.selected_endpoint + 1) % ws.endpoints.len();
            }
        }
    }

    pub fn prev_endpoint(&mut self) {
        if let Some(ws) = &self.workspace.project {
            if !ws.endpoints.is_empty() {
                self.workspace.selected_endpoint = self
                    .workspace.selected_endpoint
                    .checked_sub(1)
                    .unwrap_or(ws.endpoints.len() - 1);
            }
        }
    }

    pub fn select_endpoint(&mut self) {
        // Clone endpoint to avoid borrow conflict
        let endpoint_opt = self
            .workspace.project
            .as_ref()
            .and_then(|ws| ws.endpoints.get(self.workspace.selected_endpoint).cloned());

        if let Some(endpoint) = endpoint_opt {
            self.load_endpoint(&endpoint);
            self.navigation.active_panel = Panel::Url;
        }
    }

    fn load_endpoint(&mut self, endpoint: &DiscoveredEndpoint) {
        // Set method
        self.http.request.method = match endpoint.method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        };

        // Set URL (combine base URL with path)
        let base = self
            .workspace.project
            .as_ref()
            .and_then(|w| w.base_url.clone())
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        self.http.request.url = format!("{}{}", base.trim_end_matches('/'), endpoint.path);
        self.ui.cursor_position = self.http.request.url.len();

        // Set auth
        self.http.request.auth = match &endpoint.auth {
            discovery::AuthRequirement::Bearer => AuthType::Bearer(String::new()),
            discovery::AuthRequirement::Basic => AuthType::Basic {
                username: String::new(),
                password: String::new(),
            },
            _ => AuthType::None,
        };

        // Set body example if available
        if let Some(body) = &endpoint.body {
            if let Some(example) = &body.example {
                self.http.request.body = example.clone();
            }
        }

        // Clear previous response
        self.http.response.body = format!(
            "Loaded: {} {}\n\nAuth: {}",
            endpoint.method,
            endpoint.path,
            endpoint.auth.as_str()
        );
        self.http.response.status_code = None;
    }

    // ========================
    // Request sending
    // ========================
}

/// Find common prefix among strings (UTF-8 safe)
pub(crate) fn common_prefix(strings: &[String]) -> Option<String> {
    crate::app::text_utils::safe_common_prefix(strings)
}

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::discovery::models::{AuthRequirement, DiscoveredEndpoint, Framework, WorkspaceProject};
    use crate::messages::ui_events::Panel;
    use crate::models::{AuthType, HttpMethod};
    use std::path::PathBuf;

    fn make_state() -> AppState {
        AppState::new()
    }

    /// Build a WorkspaceProject with the given endpoints for testing.
    fn make_workspace(base_url: &str, endpoints: Vec<DiscoveredEndpoint>) -> WorkspaceProject {
        let mut proj = WorkspaceProject::new(PathBuf::from("/tmp/test-proj"));
        proj.framework = Framework::OpenAPI;
        proj.base_url = Some(base_url.to_string());
        proj.endpoints = endpoints;
        proj
    }

    fn get_endpoint(method: &str, path: &str, auth: AuthRequirement) -> DiscoveredEndpoint {
        let mut ep = DiscoveredEndpoint::new(method, path);
        ep.auth = auth;
        ep
    }

    // ── Workspace input buffer ────────────────────────────────────────────────

    #[test]
    fn test_open_workspace_input_shows_dialog() {
        let mut state = make_state();
        assert!(!state.ui.show_workspace_input);
        state.open_workspace_input();
        assert!(state.ui.show_workspace_input);
    }

    #[test]
    fn test_cancel_workspace_input_hides_and_clears() {
        let mut state = make_state();
        state.ui.show_workspace_input = true;
        state.workspace.path_input = "/some/path".to_string();
        state.cancel_workspace_input();
        assert!(!state.ui.show_workspace_input);
        assert!(state.workspace.path_input.is_empty());
    }

    #[test]
    fn test_workspace_path_char_appends_to_buffer() {
        let mut state = make_state();
        state.workspace_path_char('/');
        state.workspace_path_char('t');
        state.workspace_path_char('m');
        state.workspace_path_char('p');
        assert_eq!(state.workspace.path_input, "/tmp");
    }

    #[test]
    fn test_workspace_path_backspace_removes_last_char() {
        let mut state = make_state();
        state.workspace.path_input = "/tmp/pro".to_string();
        state.workspace_path_backspace();
        assert_eq!(state.workspace.path_input, "/tmp/pr");
        state.workspace_path_backspace();
        assert_eq!(state.workspace.path_input, "/tmp/p");
    }

    #[test]
    fn test_workspace_path_backspace_on_empty_does_not_panic() {
        let mut state = make_state();
        state.workspace.path_input = String::new();
        state.workspace_path_backspace(); // must not panic
        assert!(state.workspace.path_input.is_empty());
    }

    // ── Endpoint navigation ───────────────────────────────────────────────────

    #[test]
    fn test_next_endpoint_advances_selection() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost:8000",
            vec![
                get_endpoint("GET", "/a", AuthRequirement::None),
                get_endpoint("POST", "/b", AuthRequirement::None),
                get_endpoint("DELETE", "/c", AuthRequirement::None),
            ],
        ));
        state.workspace.selected_endpoint = 0;
        state.next_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 1);
        state.next_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 2);
    }

    #[test]
    fn test_next_endpoint_wraps_to_zero() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost:8000",
            vec![
                get_endpoint("GET", "/a", AuthRequirement::None),
                get_endpoint("POST", "/b", AuthRequirement::None),
            ],
        ));
        state.workspace.selected_endpoint = 1; // last
        state.next_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 0); // wraps
    }

    #[test]
    fn test_prev_endpoint_wraps_to_last() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost:8000",
            vec![
                get_endpoint("GET", "/a", AuthRequirement::None),
                get_endpoint("POST", "/b", AuthRequirement::None),
                get_endpoint("PUT", "/c", AuthRequirement::None),
            ],
        ));
        state.workspace.selected_endpoint = 0;
        state.prev_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 2); // wraps to last
    }

    #[test]
    fn test_next_endpoint_no_op_when_no_workspace() {
        let mut state = make_state();
        state.workspace.project = None;
        state.workspace.selected_endpoint = 0;
        state.next_endpoint(); // must not panic
        assert_eq!(state.workspace.selected_endpoint, 0);
    }

    // ── Select endpoint → load into request ──────────────────────────────────

    #[test]
    fn test_select_endpoint_loads_method_and_url() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost:8000",
            vec![get_endpoint("GET", "/users", AuthRequirement::None)],
        ));
        state.workspace.selected_endpoint = 0;
        state.select_endpoint();

        assert_eq!(state.http.request.method, HttpMethod::GET);
        assert!(
            state.http.request.url.contains("/users"),
            "URL should include path: {}",
            state.http.request.url
        );
        assert!(
            state.http.request.url.starts_with("http://"),
            "URL should include base: {}",
            state.http.request.url
        );
        assert_eq!(state.navigation.active_panel, Panel::Url);
    }

    #[test]
    fn test_select_endpoint_post_method() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "https://api.example.com",
            vec![get_endpoint("POST", "/items", AuthRequirement::None)],
        ));
        state.workspace.selected_endpoint = 0;
        state.select_endpoint();
        assert_eq!(state.http.request.method, HttpMethod::POST);
    }

    #[test]
    fn test_select_endpoint_loads_bearer_auth() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost",
            vec![get_endpoint("GET", "/secure", AuthRequirement::Bearer)],
        ));
        state.workspace.selected_endpoint = 0;
        state.select_endpoint();

        assert!(
            matches!(state.http.request.auth, AuthType::Bearer(_)),
            "Auth should be Bearer, got: {:?}",
            state.http.request.auth
        );
    }

    #[test]
    fn test_select_endpoint_loads_basic_auth() {
        let mut state = make_state();
        state.workspace.project = Some(make_workspace(
            "http://localhost",
            vec![get_endpoint("GET", "/admin", AuthRequirement::Basic)],
        ));
        state.workspace.selected_endpoint = 0;
        state.select_endpoint();

        assert!(
            matches!(state.http.request.auth, AuthType::Basic { .. }),
            "Auth should be Basic, got: {:?}",
            state.http.request.auth
        );
    }

    #[test]
    fn test_select_endpoint_no_op_when_no_workspace() {
        let mut state = make_state();
        state.workspace.project = None;
        state.select_endpoint(); // must not panic
    }
}
