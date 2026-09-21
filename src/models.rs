use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP Method enum
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
        }
    }

    pub fn next(&self) -> HttpMethod {
        match self {
            HttpMethod::GET => HttpMethod::POST,
            HttpMethod::POST => HttpMethod::PUT,
            HttpMethod::PUT => HttpMethod::PATCH,
            HttpMethod::PATCH => HttpMethod::DELETE,
            HttpMethod::DELETE => HttpMethod::GET,
        }
    }

    pub fn has_body(&self) -> bool {
        matches!(self, HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH)
    }
}

/// Authentication type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AuthType {
    #[default]
    None,
    Bearer(String),
    Basic {
        username: String,
        password: String,
    },
}

/// HTTP Header
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Header {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

impl Header {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Header {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

/// A single HTTP request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: String,
    pub auth: AuthType,
    /// When true, ignores SSL certificate errors (useful for testing environments)
    #[serde(default)]
    pub ignore_ssl_errors: bool,
}

impl Default for Request {
    fn default() -> Self {
        use crate::constants::DEFAULT_HTTP_URL;
        Request {
            name: String::from("New Request"),
            method: HttpMethod::GET,
            url: String::from(DEFAULT_HTTP_URL),
            headers: vec![
                Header::new("Content-Type", "application/json"),
                Header::new("Accept", "application/json"),
            ],
            body: String::new(),
            auth: AuthType::None,
            ignore_ssl_errors: false,
        }
    }
}

/// A collection of requests
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub requests: Vec<Request>,
}

#[allow(dead_code)] // Prepared for future collection feature
impl Collection {
    pub fn new(name: impl Into<String>) -> Self {
        Collection {
            name: name.into(),
            requests: Vec::new(),
        }
    }
}

/// Environment variables
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

#[allow(dead_code)] // Prepared for future environment feature
impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Environment {
            name: name.into(),
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Substitutes {{variable}} patterns in text
    pub fn substitute(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }
}

/// Response from HTTP request
#[derive(Clone, Debug)]
pub struct Response {
    pub status_code: Option<u16>,
    pub body: String,
    pub time_ms: u64,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status_code: None,
            body: String::from(
                r#"Quick Reference:
────────────────────────────
  s     Send request
  m     Change method
  Tab   Next panel
  e     Edit field
  w     Workspace
  o     Open project
  ?     Full help
  q     Quit
────────────────────────────
Press 's' to send your first request!"#,
            ),
            time_ms: 0,
        }
    }
}

/// History entry
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields stored for future history display feature
pub struct HistoryEntry {
    pub request: Request,
    pub response: Response,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HttpMethod ──────────────────────────────────────────────────────────

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::PUT.as_str(), "PUT");
        assert_eq!(HttpMethod::PATCH.as_str(), "PATCH");
        assert_eq!(HttpMethod::DELETE.as_str(), "DELETE");
    }

    #[test]
    fn test_http_method_cycle() {
        assert_eq!(HttpMethod::GET.next(), HttpMethod::POST);
        assert_eq!(HttpMethod::POST.next(), HttpMethod::PUT);
        assert_eq!(HttpMethod::PUT.next(), HttpMethod::PATCH);
        assert_eq!(HttpMethod::PATCH.next(), HttpMethod::DELETE);
        // Full cycle: DELETE wraps back to GET
        assert_eq!(HttpMethod::DELETE.next(), HttpMethod::GET);
    }

    #[test]
    fn test_http_method_has_body() {
        assert!(!HttpMethod::GET.has_body());
        assert!(!HttpMethod::DELETE.has_body());
        assert!(HttpMethod::POST.has_body());
        assert!(HttpMethod::PUT.has_body());
        assert!(HttpMethod::PATCH.has_body());
    }

    // ── AuthType ────────────────────────────────────────────────────────────

    #[test]
    fn test_auth_type_default_is_none() {
        assert_eq!(AuthType::default(), AuthType::None);
    }

    // ── Header ──────────────────────────────────────────────────────────────

    #[test]
    fn test_header_new_enabled_by_default() {
        let h = Header::new("Content-Type", "application/json");
        assert_eq!(h.key, "Content-Type");
        assert_eq!(h.value, "application/json");
        assert!(h.enabled, "new headers should be enabled by default");
    }

    // ── Request ─────────────────────────────────────────────────────────────

    #[test]
    fn test_request_default() {
        let req = Request::default();
        assert_eq!(req.method, HttpMethod::GET);
        assert!(!req.url.is_empty(), "default URL should not be empty");
        assert!(req.url.starts_with("http"), "default URL should start with http");
        // Should have at least the two default headers (Content-Type, Accept)
        assert!(req.headers.len() >= 2);
        assert_eq!(req.auth, AuthType::None);
        assert!(!req.ignore_ssl_errors);
        assert!(req.body.is_empty());
    }

    // ── Environment ─────────────────────────────────────────────────────────

    #[test]
    fn test_environment_set_and_get() {
        let mut env = Environment::new("test");
        assert_eq!(env.name, "test");
        env.set("base_url", "https://api.example.com");
        assert_eq!(env.get("base_url"), Some(&"https://api.example.com".to_string()));
    }

    #[test]
    fn test_environment_get_missing_key() {
        let env = Environment::new("empty");
        assert_eq!(env.get("nonexistent"), None);
    }

    #[test]
    fn test_environment_substitute() {
        let mut env = Environment::new("prod");
        env.set("base_url", "https://api.example.com");
        env.set("version", "v1");

        let result = env.substitute("{{base_url}}/{{version}}/users");
        assert_eq!(result, "https://api.example.com/v1/users");
    }

    #[test]
    fn test_environment_substitute_missing_var_unchanged() {
        let env = Environment::new("empty");
        // Variables not defined in env should remain as-is
        let result = env.substitute("{{undefined_var}}/path");
        assert_eq!(result, "{{undefined_var}}/path");
    }

    #[test]
    fn test_environment_substitute_no_variables() {
        let env = Environment::new("empty");
        let result = env.substitute("https://api.example.com/users");
        assert_eq!(result, "https://api.example.com/users");
    }

    // ── Collection ──────────────────────────────────────────────────────────

    #[test]
    fn test_collection_new() {
        let col = Collection::new("My API");
        assert_eq!(col.name, "My API");
        assert!(col.requests.is_empty());
    }
}
