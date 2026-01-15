//! Framework auto-detection from project files

use std::path::Path;
use std::fs;
use crate::discovery::models::Framework;

/// Detect the API framework used in a project directory
pub fn detect_framework(project_root: &Path) -> Framework {
    // Check for OpenAPI/Swagger specs first (highest priority)
    if has_openapi_spec(project_root) {
        return Framework::OpenAPI;
    }

    // Check Python frameworks
    if let Some(framework) = detect_python_framework(project_root) {
        return framework;
    }

    // Check Node.js frameworks
    if let Some(framework) = detect_node_framework(project_root) {
        return framework;
    }

    // Check Rust frameworks
    if let Some(framework) = detect_rust_framework(project_root) {
        return framework;
    }

    // Check Go frameworks
    if let Some(framework) = detect_go_framework(project_root) {
        return framework;
    }

    Framework::Unknown
}

/// Check if project has OpenAPI/Swagger spec
fn has_openapi_spec(root: &Path) -> bool {
    let spec_files = [
        "openapi.yaml",
        "openapi.yml",
        "openapi.json",
        "swagger.yaml",
        "swagger.yml",
        "swagger.json",
        "api/openapi.yaml",
        "api/openapi.yml",
        "docs/openapi.yaml",
        "docs/swagger.yaml",
    ];

    for file in &spec_files {
        if root.join(file).exists() {
            return true;
        }
    }
    false
}

/// Find OpenAPI spec file path
pub fn find_openapi_spec(root: &Path) -> Option<std::path::PathBuf> {
    let spec_files = [
        "openapi.yaml",
        "openapi.yml",
        "openapi.json",
        "swagger.yaml",
        "swagger.yml",
        "swagger.json",
        "api/openapi.yaml",
        "api/openapi.yml",
        "docs/openapi.yaml",
        "docs/swagger.yaml",
    ];

    for file in &spec_files {
        let path = root.join(file);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Detect Python framework from requirements or pyproject.toml
fn detect_python_framework(root: &Path) -> Option<Framework> {
    let files_to_check = ["requirements.txt", "pyproject.toml", "setup.py", "Pipfile"];
    
    for file in &files_to_check {
        let path = root.join(file);
        if let Ok(content) = fs::read_to_string(&path) {
            let lower = content.to_lowercase();
            if lower.contains("fastapi") {
                return Some(Framework::FastAPI);
            }
            if lower.contains("flask") {
                return Some(Framework::Flask);
            }
        }
    }

    // Also check main Python files
    for entry in ["main.py", "app.py", "app/__init__.py", "src/main.py"].iter() {
        let path = root.join(entry);
        if let Ok(content) = fs::read_to_string(&path) {
            if content.contains("FastAPI") || content.contains("from fastapi") {
                return Some(Framework::FastAPI);
            }
            if content.contains("Flask") || content.contains("from flask") {
                return Some(Framework::Flask);
            }
        }
    }

    None
}

/// Detect Node.js framework from package.json
fn detect_node_framework(root: &Path) -> Option<Framework> {
    let package_json = root.join("package.json");
    
    if let Ok(content) = fs::read_to_string(&package_json) {
        let lower = content.to_lowercase();
        
        if lower.contains("@nestjs") {
            return Some(Framework::NestJS);
        }
        if lower.contains("\"express\"") {
            return Some(Framework::Express);
        }
    }

    None
}

/// Detect Rust framework from Cargo.toml
fn detect_rust_framework(root: &Path) -> Option<Framework> {
    let cargo_toml = root.join("Cargo.toml");
    
    if let Ok(content) = fs::read_to_string(&cargo_toml) {
        let lower = content.to_lowercase();
        
        if lower.contains("actix-web") {
            return Some(Framework::Actix);
        }
        if lower.contains("axum") {
            return Some(Framework::Axum);
        }
    }

    None
}

/// Detect Go framework from go.mod
fn detect_go_framework(root: &Path) -> Option<Framework> {
    let go_mod = root.join("go.mod");
    
    if let Ok(content) = fs::read_to_string(&go_mod) {
        if content.contains("github.com/gin-gonic/gin") {
            return Some(Framework::Gin);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_detect_openapi() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("openapi.yaml")).unwrap();
        assert_eq!(detect_framework(dir.path()), Framework::OpenAPI);
    }
}
