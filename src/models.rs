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
