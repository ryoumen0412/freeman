//! Workspace loader - coordinates filesystem inspection and discovery logic

use std::fmt;
use std::path::{Path, PathBuf};

use crate::discovery::detector;
use crate::discovery::models::{Framework, WorkspaceProject};
use crate::discovery::openapi;
use crate::discovery::{
    load_django_project, load_express_project, load_go_project, load_java_project,
    load_laravel_project, load_nestjs_project, load_python_project, load_rust_project,
};

/// The origin from which endpoints were discovered
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoverySource {
    OpenApi(PathBuf),
    SourceCode(Framework),
}

/// The result of loading a workspace, combining the project and its origin
#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceLoadResult {
    pub project: WorkspaceProject,
    pub source: DiscoverySource,
}

/// Errors that can occur during workspace discovery
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DiscoveryError {
    NotFound(PathBuf),
    OpenApi { path: PathBuf, reason: String },
    UnsupportedFramework(PathBuf),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::NotFound(path) => {
                write!(f, "Directory or file not found: {}", path.display())
            }
            DiscoveryError::OpenApi { reason, .. } => {
                write!(f, "Error parsing OpenAPI: {}", reason)
            }
            DiscoveryError::UnsupportedFramework(path) => {
                write!(
                    f,
                    "No supported framework detected in {}\n\nSupported: OpenAPI, FastAPI, Flask, Django, Express.js, NestJS, Spring Boot, Laravel, Actix Web, Axum, Gin",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Expands a leading tilde (~) in a path to the user's home directory
pub fn expand_tilde(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1));
        }
    }
    path.to_path_buf()
}

/// Discovers and loads an API workspace project from a directory or path
pub fn load_workspace(path: impl AsRef<Path>) -> Result<WorkspaceLoadResult, DiscoveryError> {
    let path = path.as_ref();
    let expanded = expand_tilde(path);

    if !expanded.exists() {
        return Err(DiscoveryError::NotFound(expanded));
    }

    // Try OpenAPI spec first
    if let Some(spec_path) = detector::find_openapi_spec(&expanded) {
        return match openapi::parse_openapi(&spec_path) {
            Ok(project) => Ok(WorkspaceLoadResult {
                project,
                source: DiscoverySource::OpenApi(spec_path),
            }),
            Err(e) => Err(DiscoveryError::OpenApi {
                path: spec_path,
                reason: e.to_string(),
            }),
        };
    }

    // Fallback to source code parsing
    let framework = detector::detect_framework(&expanded);

    let project = match framework {
        Framework::FastAPI | Framework::Flask => {
            Some(load_python_project(&expanded, framework.clone()))
        }
        Framework::Django => Some(load_django_project(&expanded)),
        Framework::Express => Some(load_express_project(&expanded)),
        Framework::NestJS => Some(load_nestjs_project(&expanded)),
        Framework::SpringBoot => Some(load_java_project(&expanded)),
        Framework::Laravel => Some(load_laravel_project(&expanded)),
        Framework::Actix | Framework::Axum => {
            Some(load_rust_project(&expanded, framework.clone()))
        }
        Framework::Gin => Some(load_go_project(&expanded, framework.clone())),
        _ => None,
    };

    if let Some(project) = project {
        Ok(WorkspaceLoadResult {
            project,
            source: DiscoverySource::SourceCode(framework),
        })
    } else {
        Err(DiscoveryError::UnsupportedFramework(expanded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_loads_openapi_workspace() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("openapi.yaml");
        let spec_content = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      summary: Get users
      responses:
        '200':
          description: OK
"#;
        fs::write(&spec_path, spec_content).unwrap();

        let result = load_workspace(dir.path()).expect("Should load openapi workspace");
        assert_eq!(result.source, DiscoverySource::OpenApi(spec_path));
        assert_eq!(result.project.framework, Framework::OpenAPI);
        assert_eq!(result.project.endpoints.len(), 1);
        assert_eq!(result.project.endpoints[0].path, "/users");
        assert_eq!(result.project.endpoints[0].method, "GET");
    }

    #[test]
    fn test_loads_fastapi_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "fastapi==0.100.0\n").unwrap();
        let main_py = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items")
def get_items():
    return []
"#;
        fs::write(dir.path().join("main.py"), main_py).unwrap();

        let result = load_workspace(dir.path()).expect("Should load FastAPI workspace");
        assert_eq!(
            result.source,
            DiscoverySource::SourceCode(Framework::FastAPI)
        );
        assert_eq!(result.project.framework, Framework::FastAPI);
        assert_eq!(result.project.endpoints.len(), 1);
        assert_eq!(result.project.endpoints[0].path, "/items");
        assert_eq!(result.project.endpoints[0].method, "GET");
    }

    #[test]
    fn test_returns_error_for_unknown_project() {
        let dir = tempdir().unwrap();
        let err = load_workspace(dir.path()).unwrap_err();
        assert_eq!(
            err,
            DiscoveryError::UnsupportedFramework(dir.path().to_path_buf())
        );
        let msg = err.to_string();
        assert!(msg.contains("No supported framework detected"));
    }

    #[test]
    fn test_returns_error_for_nonexistent_path() {
        let non_existent = PathBuf::from("/path/that/really/does/not/exist/anywhere");
        let err = load_workspace(&non_existent).unwrap_err();
        assert_eq!(err, DiscoveryError::NotFound(non_existent));
        let msg = err.to_string();
        assert!(msg.contains("Directory or file not found"));
    }

    #[test]
    fn test_returns_error_for_invalid_openapi() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("openapi.yaml");
        fs::write(&spec_path, "invalid: [yaml: broken").unwrap();

        let err = load_workspace(dir.path()).unwrap_err();
        match err {
            DiscoveryError::OpenApi { path, reason } => {
                assert_eq!(path, spec_path);
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected DiscoveryError::OpenApi, got {:?}", err),
        }
    }

    #[test]
    fn test_expand_tilde_without_tilde_is_unchanged() {
        let path = Path::new("/var/log");
        assert_eq!(expand_tilde(path), PathBuf::from("/var/log"));
    }
}
