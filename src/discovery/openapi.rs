//! OpenAPI/Swagger specification parser

use std::path::Path;
use std::fs;
use anyhow::Result;
use serde_json::Value;

use crate::discovery::models::{
    AuthRequirement, BodySchema, DiscoveredEndpoint, Parameter, 
    ParameterLocation, WorkspaceProject, Framework,
};

/// Parse an OpenAPI spec file and return a WorkspaceProject
pub fn parse_openapi(spec_path: &Path) -> Result<WorkspaceProject> {
    let content = fs::read_to_string(spec_path)?;
    
    // Determine if JSON or YAML
    let spec: Value = if spec_path.extension().map(|e| e == "json").unwrap_or(false) {
        serde_json::from_str(&content)?
    } else {
        serde_yaml::from_str(&content)?
    };

    let root = spec_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut project = WorkspaceProject::new(root);
    project.framework = Framework::OpenAPI;

    // Extract info
    if let Some(info) = spec.get("info") {
        project.title = info.get("title").and_then(|v| v.as_str()).map(String::from);
        project.version = info.get("version").and_then(|v| v.as_str()).map(String::from);
    }

    // Extract base URL from servers
    if let Some(servers) = spec.get("servers").and_then(|s| s.as_array()) {
        if let Some(first) = servers.first() {
            project.base_url = first.get("url").and_then(|v| v.as_str()).map(String::from);
        }
    }

    // Detect global security schemes
    let security_schemes = extract_security_schemes(&spec);
    let global_security = extract_security_requirement(&spec, &security_schemes);

    // Parse paths
    if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
        for (path, methods) in paths {
            if let Some(methods_obj) = methods.as_object() {
                for (method, operation) in methods_obj {
                    // Skip non-HTTP method keys like "parameters"
                    if !is_http_method(method) {
                        continue;
                    }

                    let mut endpoint = DiscoveredEndpoint::new(method, path);

                    // Extract operation details
                    if let Some(op) = operation.as_object() {
                        endpoint.operation_id = op.get("operationId")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        
                        endpoint.summary = op.get("summary")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        
                        endpoint.description = op.get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        endpoint.deprecated = op.get("deprecated")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        // Tags
                        if let Some(tags) = op.get("tags").and_then(|t| t.as_array()) {
                            endpoint.tags = tags.iter()
                                .filter_map(|t| t.as_str().map(String::from))
                                .collect();
                        }

                        // Parameters
                        if let Some(params) = op.get("parameters").and_then(|p| p.as_array()) {
                            for param in params {
                                if let Some(p) = parse_parameter(param) {
                                    endpoint.parameters.push(p);
                                }
                            }
                        }

                        // Also check path-level parameters
                        if let Some(params) = methods.get("parameters").and_then(|p| p.as_array()) {
                            for param in params {
                                if let Some(p) = parse_parameter(param) {
                                    // Don't duplicate
                                    if !endpoint.parameters.iter().any(|ep| ep.name == p.name) {
                                        endpoint.parameters.push(p);
                                    }
                                }
                            }
                        }

                        // Request body
                        if let Some(body) = op.get("requestBody") {
                            endpoint.body = parse_request_body(body);
                        }

                        // Security (operation-level overrides global)
                        if let Some(security) = op.get("security") {
                            endpoint.auth = extract_security_requirement_from(security, &security_schemes);
                        } else {
                            endpoint.auth = global_security.clone();
                        }
                    }

                    project.endpoints.push(endpoint);
                }
            }
        }
    }

    Ok(project)
}

fn is_http_method(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "get" | "post" | "put" | "patch" | "delete" | "head" | "options")
}

fn extract_security_schemes(spec: &Value) -> Vec<(String, AuthRequirement)> {
    let mut schemes = Vec::new();

    let components = spec.get("components")
        .or_else(|| spec.get("securityDefinitions")); // OpenAPI 2.0

    if let Some(sec_schemes) = components
        .and_then(|c| c.get("securitySchemes"))
        .or_else(|| components)
        .and_then(|s| s.as_object()) 
    {
        for (name, scheme) in sec_schemes {
            let scheme_type = scheme.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let _scheme_in = scheme.get("in").and_then(|i| i.as_str()).unwrap_or("");
            let scheme_name = scheme.get("name").and_then(|n| n.as_str()).unwrap_or("");

            let auth = match scheme_type {
                "http" => {
                    let http_scheme = scheme.get("scheme").and_then(|s| s.as_str()).unwrap_or("");
                    match http_scheme {
                        "bearer" => AuthRequirement::Bearer,
                        "basic" => AuthRequirement::Basic,
                        _ => AuthRequirement::Custom(http_scheme.to_string()),
                    }
                }
                "apiKey" => AuthRequirement::ApiKey { header: scheme_name.to_string() },
                "oauth2" => AuthRequirement::OAuth2,
                "openIdConnect" => AuthRequirement::OAuth2,
                _ => AuthRequirement::Custom(scheme_type.to_string()),
            };

            schemes.push((name.clone(), auth));
        }
    }

    schemes
}

fn extract_security_requirement(spec: &Value, schemes: &[(String, AuthRequirement)]) -> AuthRequirement {
    if let Some(security) = spec.get("security") {
        extract_security_requirement_from(security, schemes)
    } else {
        AuthRequirement::None
    }
}

fn extract_security_requirement_from(security: &Value, schemes: &[(String, AuthRequirement)]) -> AuthRequirement {
    if let Some(arr) = security.as_array() {
        if arr.is_empty() {
            return AuthRequirement::None;
        }
        // Take first security requirement
        if let Some(first) = arr.first().and_then(|v| v.as_object()) {
            if let Some(scheme_name) = first.keys().next() {
                // Look up the scheme
                for (name, auth) in schemes {
                    if name == scheme_name {
                        return auth.clone();
                    }
                }
                return AuthRequirement::Custom(scheme_name.clone());
            }
        }
    }
    AuthRequirement::None
}

fn parse_parameter(param: &Value) -> Option<Parameter> {
    let name = param.get("name")?.as_str()?.to_string();
    let location = match param.get("in")?.as_str()? {
        "path" => ParameterLocation::Path,
        "query" => ParameterLocation::Query,
        "header" => ParameterLocation::Header,
        "cookie" => ParameterLocation::Cookie,
        _ => return None,
    };

    let required = param.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
    
    let param_type = param.get("schema")
        .and_then(|s| s.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("string")
        .to_string();

    let description = param.get("description")
        .and_then(|d| d.as_str())
        .map(String::from);

    let default = param.get("schema")
        .and_then(|s| s.get("default"))
        .map(|d| d.to_string());

    Some(Parameter {
        name,
        location,
        required,
        param_type,
        description,
        default,
    })
}

fn parse_request_body(body: &Value) -> Option<BodySchema> {
    let required = body.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
    
    // Get content types
    if let Some(content) = body.get("content").and_then(|c| c.as_object()) {
        // Prefer application/json
        let (content_type, schema_obj) = if let Some(json) = content.get("application/json") {
            ("application/json", json)
        } else if let Some((ct, obj)) = content.iter().next() {
            (ct.as_str(), obj)
        } else {
            return None;
        };

        let schema_name = schema_obj.get("schema")
            .and_then(|s| s.get("$ref"))
            .and_then(|r| r.as_str())
            .map(|r| r.split('/').last().unwrap_or("").to_string());

        let example = schema_obj.get("example")
            .or_else(|| schema_obj.get("schema").and_then(|s| s.get("example")))
            .map(|e| serde_json::to_string_pretty(e).unwrap_or_default());

        return Some(BodySchema {
            content_type: content_type.to_string(),
            schema_name,
            required,
            example,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_openapi() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      summary: Get all users
      responses:
        200:
          description: OK
    post:
      summary: Create user
      responses:
        201:
          description: Created
"#;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let spec_path = temp_dir.path().join("openapi.yaml");
        std::fs::write(&spec_path, yaml).unwrap();
        
        let project = parse_openapi(&spec_path).unwrap();
        assert_eq!(project.title, Some("Test API".to_string()));
        assert_eq!(project.endpoints.len(), 2);
    }
}
