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
        self.show_workspace_input = true;
    }

    pub fn workspace_path_char(&mut self, c: char) {
        self.workspace_path_input.push(c);
    }

    pub fn workspace_path_backspace(&mut self) {
        self.workspace_path_input.pop();
    }

    pub fn cancel_workspace_input(&mut self) {
        self.show_workspace_input = false;
        self.workspace_path_input.clear();
    }

    pub fn workspace_path_autocomplete(&mut self) {
        use std::fs;

        // Expand ~ to home directory
        let input = if self.workspace_path_input.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                self.workspace_path_input
                    .replacen("~", &home.to_string_lossy(), 1)
            } else {
                return;
            }
        } else {
            self.workspace_path_input.clone()
        };

        let path = PathBuf::from(&input);

        // If it's already a valid directory, try to complete further
        if path.is_dir() && !input.ends_with('/') {
            self.workspace_path_input = format!("{}/", input);
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
                self.workspace_path_input = format!("{}/", completed.to_string_lossy());
            } else if matches.len() > 1 {
                // Multiple matches - complete common prefix
                if let Some(common) = common_prefix(&matches) {
                    if common.len() > prefix.len() {
                        let completed = parent.join(&common);
                        self.workspace_path_input = completed.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    pub fn load_workspace(&mut self) {
        let path = self.workspace_path_input.clone();

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
                    self.workspace = Some(project);
                    self.selected_endpoint = 0;
                    self.response.body = format!("✓ Loaded {} endpoints from OpenAPI spec", count);
                    self.show_workspace_input = false;
                    self.workspace_path_input.clear();
                    return;
                }
                Err(e) => {
                    self.response.body = format!("Error parsing OpenAPI: {}", e);
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
            _ => None,
        };

        if let Some(proj) = project {
            let count = proj.endpoints.len();
            let fw_name = proj.framework.as_str().to_string();
            self.workspace = Some(proj);
            self.selected_endpoint = 0;
            self.response.body =
                format!("✓ Loaded {} endpoints from {} source code", count, fw_name);
        } else {
            self.response.body = format!(
                "No supported framework detected in {}\n\nSupported: OpenAPI, FastAPI, Flask, Django, Express.js, NestJS, Spring Boot, Laravel",
                expanded
            );
        }

        self.show_workspace_input = false;
        self.workspace_path_input.clear();
    }

    pub fn next_endpoint(&mut self) {
        if let Some(ws) = &self.workspace {
            if !ws.endpoints.is_empty() {
                self.selected_endpoint = (self.selected_endpoint + 1) % ws.endpoints.len();
            }
        }
    }

    pub fn prev_endpoint(&mut self) {
        if let Some(ws) = &self.workspace {
            if !ws.endpoints.is_empty() {
                self.selected_endpoint = self
                    .selected_endpoint
                    .checked_sub(1)
                    .unwrap_or(ws.endpoints.len() - 1);
            }
        }
    }

    pub fn select_endpoint(&mut self) {
        // Clone endpoint to avoid borrow conflict
        let endpoint_opt = self
            .workspace
            .as_ref()
            .and_then(|ws| ws.endpoints.get(self.selected_endpoint).cloned());

        if let Some(endpoint) = endpoint_opt {
            self.load_endpoint(&endpoint);
            self.active_panel = Panel::Url;
        }
    }

    fn load_endpoint(&mut self, endpoint: &DiscoveredEndpoint) {
        // Set method
        self.request.method = match endpoint.method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        };

        // Set URL (combine base URL with path)
        let base = self
            .workspace
            .as_ref()
            .and_then(|w| w.base_url.clone())
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        self.request.url = format!("{}{}", base.trim_end_matches('/'), endpoint.path);
        self.cursor_position = self.request.url.len();

        // Set auth
        self.request.auth = match &endpoint.auth {
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
                self.request.body = example.clone();
            }
        }

        // Clear previous response
        self.response.body = format!(
            "Loaded: {} {}\n\nAuth: {}",
            endpoint.method,
            endpoint.path,
            endpoint.auth.as_str()
        );
        self.response.status_code = None;
    }

    // ========================
    // Request sending
    // ========================
}

/// Find common prefix among strings (UTF-8 safe)
pub(crate) fn common_prefix(strings: &[String]) -> Option<String> {
    crate::app::text_utils::safe_common_prefix(strings)
}
