//! Data models for discovered endpoints and workspace projects

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Detected API framework
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Framework {
    FastAPI,
    Flask,
    Django,
    Express,
    NestJS,
    SpringBoot,
    Laravel,
    Actix,
    Axum,
    Gin,
    OpenAPI,  // From spec file
    Unknown,
}

impl Framework {
    pub fn as_str(&self) -> &str {
        match self {
            Framework::FastAPI => "FastAPI",
            Framework::Flask => "Flask",
            Framework::Django => "Django",
            Framework::Express => "Express",
            Framework::NestJS => "NestJS",
            Framework::SpringBoot => "Spring Boot",
            Framework::Laravel => "Laravel",
            Framework::Actix => "Actix",
            Framework::Axum => "Axum",
            Framework::Gin => "Gin",
            Framework::OpenAPI => "OpenAPI",
            Framework::Unknown => "Unknown",
        }
    }
}

/// Authentication requirement detected for an endpoint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AuthRequirement {
    None,
    Bearer,
    Basic,
    ApiKey { header: String },
    OAuth2,
    Custom(String),
}

impl AuthRequirement {
    pub fn as_str(&self) -> &str {
        match self {
            AuthRequirement::None => "None",
            AuthRequirement::Bearer => "Bearer",
            AuthRequirement::Basic => "Basic",
            AuthRequirement::ApiKey { .. } => "API Key",
            AuthRequirement::OAuth2 => "OAuth2",
            AuthRequirement::Custom(s) => s,
        }
    }
}

/// Parameter location in request
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

/// A discovered parameter
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub param_type: String,
    pub description: Option<String>,
    pub default: Option<String>,
}

/// Body schema information
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodySchema {
    pub content_type: String,
    pub schema_name: Option<String>,
    pub required: bool,
    pub example: Option<String>,
}

/// A discovered API endpoint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredEndpoint {
    /// HTTP method
    pub method: String,
    /// URL path (e.g., "/api/users/{id}")
    pub path: String,
    /// Operation ID or name
    pub operation_id: Option<String>,
    /// Human-readable summary
    pub summary: Option<String>,
    /// Detailed description
    pub description: Option<String>,
    /// Source file (if from code)
    pub source_file: Option<PathBuf>,
    /// Line number in source file
    pub line_number: Option<usize>,
    /// Path and query parameters
    pub parameters: Vec<Parameter>,
    /// Request body schema
    pub body: Option<BodySchema>,
    /// Authentication requirement
    pub auth: AuthRequirement,
    /// Tags for grouping
    pub tags: Vec<String>,
    /// Deprecated flag
    pub deprecated: bool,
}

impl DiscoveredEndpoint {
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        DiscoveredEndpoint {
            method: method.into().to_uppercase(),
            path: path.into(),
            operation_id: None,
            summary: None,
            description: None,
            source_file: None,
            line_number: None,
            parameters: Vec::new(),
            body: None,
            auth: AuthRequirement::None,
            tags: Vec::new(),
            deprecated: false,
        }
    }

    /// Returns display title for the endpoint
    #[allow(dead_code)]  // Prepared for future endpoint display feature
    pub fn display_title(&self) -> String {
        self.summary.clone()
            .or_else(|| self.operation_id.clone())
            .unwrap_or_else(|| self.path.clone())
    }
}

/// A workspace project with discovered endpoints
#[derive(Clone, Debug)]
#[allow(dead_code)]  // Some fields stored for future features
pub struct WorkspaceProject {
    /// Project root directory
    pub root: PathBuf,
    /// Detected framework
    pub framework: Framework,
    /// Base URL (from config or spec)
    pub base_url: Option<String>,
    /// API title/name
    pub title: Option<String>,
    /// API version
    pub version: Option<String>,
    /// All discovered endpoints
    pub endpoints: Vec<DiscoveredEndpoint>,
}

impl WorkspaceProject {
    pub fn new(root: PathBuf) -> Self {
        WorkspaceProject {
            root,
            framework: Framework::Unknown,
            base_url: None,
            title: None,
            version: None,
            endpoints: Vec::new(),
        }
    }

    /// Get endpoints grouped by first path segment or tags
    #[allow(dead_code)]  // Prepared for future grouped endpoints view
    pub fn grouped_endpoints(&self) -> Vec<(String, Vec<&DiscoveredEndpoint>)> {
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<String, Vec<&DiscoveredEndpoint>> = BTreeMap::new();

        for endpoint in &self.endpoints {
            let group = if !endpoint.tags.is_empty() {
                endpoint.tags[0].clone()
            } else {
                // Extract first path segment
                endpoint.path
                    .trim_start_matches('/')
                    .split('/')
                    .next()
                    .unwrap_or("root")
                    .to_string()
            };
            groups.entry(group).or_default().push(endpoint);
        }

        groups.into_iter().collect()
    }
}
