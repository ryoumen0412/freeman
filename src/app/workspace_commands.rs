//! Workspace orchestration — methods that coordinate across sub-states.
//!
//! Pure workspace logic (next/prev endpoint, path editing) lives in
//! `state.rs` (impl WorkspaceState). This file contains methods that
//! touch workspace + http + ui + navigation.

use crate::app::AppState;
use crate::discovery::{
    AuthRequirement, DiscoveredEndpoint, DiscoveryError, DiscoverySource, WorkspaceLoadResult,
};
use crate::messages::ui_events::Panel;
use crate::models::AuthType;
use crate::models::HttpMethod;

impl AppState {
    // ========================
    // Workspace input (cross-state: ui + workspace)
    // ========================

    pub fn open_workspace_input(&mut self) {
        self.ui.open_workspace_input();
    }

    pub fn cancel_workspace_input(&mut self) {
        self.ui.close_workspace_input();
        self.workspace.path_input.clear();
    }

    // ========================
    // Apply workspace load results (pure data mutation: workspace + http + ui)
    // ========================

    pub fn apply_workspace_result(&mut self, result: WorkspaceLoadResult) {
        let count = result.project.endpoints.len();
        self.workspace.project = Some(result.project);
        self.workspace.selected_endpoint = 0;
        self.ui.close_workspace_input();
        self.workspace.path_input.clear();

        self.http.response.body = match result.source {
            DiscoverySource::OpenApi(_) => {
                format!("✓ Loaded {} endpoints from OpenAPI spec", count)
            }
            DiscoverySource::SourceCode(framework) => {
                format!(
                    "✓ Loaded {} endpoints from {} source code",
                    count,
                    framework.as_str()
                )
            }
        };
    }

    pub fn apply_workspace_error(&mut self, error: &DiscoveryError) {
        self.ui.close_workspace_input();
        self.workspace.path_input.clear();
        self.http.response.body = error.to_string();
    }

    // ========================
    // Select endpoint (cross-state: workspace + http + ui + navigation)
    // ========================

    pub fn select_endpoint(&mut self) {
        let endpoint_opt = self
            .workspace
            .project
            .as_ref()
            .and_then(|ws| ws.endpoints.get(self.workspace.selected_endpoint).cloned());

        if let Some(endpoint) = endpoint_opt {
            self.load_endpoint(&endpoint);
            self.navigation.active_panel = Panel::Url;
        }
    }

    fn load_endpoint(&mut self, endpoint: &DiscoveredEndpoint) {
        self.http.request.method = match endpoint.method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        };

        let base = self
            .workspace
            .project
            .as_ref()
            .and_then(|w| w.base_url.clone())
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        self.http.request.url = format!("{}{}", base.trim_end_matches('/'), endpoint.path);
        self.ui.cursor_position = self.http.request.url.len();

        self.http.request.auth = match &endpoint.auth {
            AuthRequirement::Bearer => AuthType::Bearer(String::new()),
            AuthRequirement::Basic => AuthType::Basic {
                username: String::new(),
                password: String::new(),
            },
            _ => AuthType::None,
        };

        if let Some(body) = &endpoint.body {
            if let Some(example) = &body.example {
                self.http.request.body = example.clone();
            }
        }

        self.http.response.body = format!(
            "Loaded: {} {}\n\nAuth: {}",
            endpoint.method,
            endpoint.path,
            endpoint.auth.as_str()
        );
        self.http.response.status_code = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::discovery::models::{
        AuthRequirement, DiscoveredEndpoint, Framework, WorkspaceProject,
    };
    use crate::messages::ui_events::Panel;
    use crate::models::{AuthType, HttpMethod};
    use std::path::PathBuf;

    fn make_state() -> AppState {
        AppState::new()
    }

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
        state.workspace.path_char('/');
        state.workspace.path_char('t');
        state.workspace.path_char('m');
        state.workspace.path_char('p');
        assert_eq!(state.workspace.path_input, "/tmp");
    }

    #[test]
    fn test_workspace_path_backspace_removes_last_char() {
        let mut state = make_state();
        state.workspace.path_input = "/tmp/pro".to_string();
        state.workspace.path_backspace();
        assert_eq!(state.workspace.path_input, "/tmp/pr");
        state.workspace.path_backspace();
        assert_eq!(state.workspace.path_input, "/tmp/p");
    }

    #[test]
    fn test_workspace_path_backspace_on_empty_does_not_panic() {
        let mut state = make_state();
        state.workspace.path_input = String::new();
        state.workspace.path_backspace();
        assert!(state.workspace.path_input.is_empty());
    }

    // ── Endpoint navigation (direct on sub-state) ────────────────────────────

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
        state.workspace.next_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 1);
        state.workspace.next_endpoint();
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
        state.workspace.selected_endpoint = 1;
        state.workspace.next_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 0);
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
        state.workspace.prev_endpoint();
        assert_eq!(state.workspace.selected_endpoint, 2);
    }

    #[test]
    fn test_next_endpoint_no_op_when_no_workspace() {
        let mut state = make_state();
        state.workspace.project = None;
        state.workspace.selected_endpoint = 0;
        state.workspace.next_endpoint();
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
        state.select_endpoint();
    }

    // ── Apply workspace load result / error ───────────────────────────────────

    #[test]
    fn test_apply_workspace_result_openapi() {
        let mut state = make_state();
        state.ui.show_workspace_input = true;
        state.workspace.path_input = "/some/spec/dir".to_string();

        let proj = make_workspace(
            "http://localhost:8000",
            vec![get_endpoint("GET", "/users", AuthRequirement::None)],
        );
        let result = crate::discovery::WorkspaceLoadResult {
            project: proj,
            source: crate::discovery::DiscoverySource::OpenApi(PathBuf::from(
                "/some/spec/dir/openapi.yaml",
            )),
        };

        state.apply_workspace_result(result);

        assert!(state.workspace.project.is_some());
        assert_eq!(state.workspace.selected_endpoint, 0);
        assert!(!state.ui.show_workspace_input);
        assert!(state.workspace.path_input.is_empty());
        assert_eq!(
            state.http.response.body,
            "✓ Loaded 1 endpoints from OpenAPI spec"
        );
    }

    #[test]
    fn test_apply_workspace_result_source_code() {
        let mut state = make_state();
        state.ui.show_workspace_input = true;
        state.workspace.path_input = "/some/project".to_string();

        let mut proj = make_workspace(
            "http://localhost:8000",
            vec![
                get_endpoint("GET", "/a", AuthRequirement::None),
                get_endpoint("POST", "/b", AuthRequirement::None),
            ],
        );
        proj.framework = Framework::FastAPI;

        let result = crate::discovery::WorkspaceLoadResult {
            project: proj,
            source: crate::discovery::DiscoverySource::SourceCode(Framework::FastAPI),
        };

        state.apply_workspace_result(result);

        assert!(state.workspace.project.is_some());
        assert_eq!(state.workspace.selected_endpoint, 0);
        assert!(!state.ui.show_workspace_input);
        assert!(state.workspace.path_input.is_empty());
        assert_eq!(
            state.http.response.body,
            "✓ Loaded 2 endpoints from FastAPI source code"
        );
    }

    #[test]
    fn test_apply_workspace_error() {
        let mut state = make_state();
        state.ui.show_workspace_input = true;
        state.workspace.path_input = "/some/nonexistent".to_string();

        let error = crate::discovery::DiscoveryError::NotFound(PathBuf::from("/some/nonexistent"));
        state.apply_workspace_error(&error);

        assert!(!state.ui.show_workspace_input);
        assert!(state.workspace.path_input.is_empty());
        assert_eq!(state.http.response.body, error.to_string());
    }

    #[test]
    fn test_applying_workspace_selects_first_endpoint() {
        let mut state = make_state();
        state.workspace.selected_endpoint = 5; // Previous index
        let proj = make_workspace(
            "http://localhost:8000",
            vec![
                get_endpoint("GET", "/first", AuthRequirement::None),
                get_endpoint("GET", "/second", AuthRequirement::None),
            ],
        );
        let result = crate::discovery::WorkspaceLoadResult {
            project: proj,
            source: crate::discovery::DiscoverySource::SourceCode(Framework::FastAPI),
        };

        state.apply_workspace_result(result);
        assert_eq!(state.workspace.selected_endpoint, 0);

        // Subsequent select_endpoint loads the first endpoint
        state.select_endpoint();
        assert_eq!(state.http.request.method, HttpMethod::GET);
        assert!(state.http.request.url.contains("/first"));
    }

    #[test]
    fn test_integration_discovery_loader_to_app_state() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("openapi.yaml");
        let spec_content = r#"
openapi: 3.0.0
info:
  title: Integration API
  version: 1.0.0
paths:
  /integration/test:
    post:
      summary: Post integration test
      responses:
        '200':
          description: OK
"#;
        std::fs::write(&spec_path, spec_content).unwrap();

        // 1. Discovery outside AppState
        let load_result = crate::discovery::load_workspace(dir.path()).unwrap();

        // 2. Pure state application inside AppState
        let mut state = make_state();
        state.apply_workspace_result(load_result);

        assert!(state.workspace.project.is_some());
        assert_eq!(state.workspace.selected_endpoint, 0);

        // 3. Selection and navigation
        state.select_endpoint();
        assert_eq!(state.http.request.method, HttpMethod::POST);
        assert!(state.http.request.url.contains("/integration/test"));
        assert_eq!(state.navigation.active_panel, Panel::Url);
    }
}
