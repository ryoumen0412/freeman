//! OpenAPI/Swagger specification parser

use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::discovery::models::{
    AuthRequirement, BodySchema, DiscoveredEndpoint, Framework, Parameter, ParameterLocation,
    WorkspaceProject,
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
        project.version = info
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from);
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
                        endpoint.operation_id = op
                            .get("operationId")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        endpoint.summary =
                            op.get("summary").and_then(|v| v.as_str()).map(String::from);

                        endpoint.description = op
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        endpoint.deprecated = op
                            .get("deprecated")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        // Tags
                        if let Some(tags) = op.get("tags").and_then(|t| t.as_array()) {
                            endpoint.tags = tags
                                .iter()
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
                            endpoint.auth =
                                extract_security_requirement_from(security, &security_schemes);
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
    matches!(
        s.to_lowercase().as_str(),
        "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
    )
}

fn extract_security_schemes(spec: &Value) -> Vec<(String, AuthRequirement)> {
    let mut schemes = Vec::new();

    let components = spec
        .get("components")
        .or_else(|| spec.get("securityDefinitions")); // OpenAPI 2.0

    if let Some(sec_schemes) = components
        .and_then(|c| c.get("securitySchemes"))
        .or(components)
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
                "apiKey" => AuthRequirement::ApiKey {
                    header: scheme_name.to_string(),
                },
                "oauth2" => AuthRequirement::OAuth2,
                "openIdConnect" => AuthRequirement::OAuth2,
                _ => AuthRequirement::Custom(scheme_type.to_string()),
            };

            schemes.push((name.clone(), auth));
        }
    }

    schemes
}

fn extract_security_requirement(
    spec: &Value,
    schemes: &[(String, AuthRequirement)],
) -> AuthRequirement {
    if let Some(security) = spec.get("security") {
        extract_security_requirement_from(security, schemes)
    } else {
        AuthRequirement::None
    }
}

fn extract_security_requirement_from(
    security: &Value,
    schemes: &[(String, AuthRequirement)],
) -> AuthRequirement {
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

    let required = param
        .get("required")
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    let param_type = param
        .get("schema")
        .and_then(|s| s.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("string")
        .to_string();

    let description = param
        .get("description")
        .and_then(|d| d.as_str())
        .map(String::from);

    let default = param
        .get("schema")
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
    let required = body
        .get("required")
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

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

        let schema_name = schema_obj
            .get("schema")
            .and_then(|s| s.get("$ref"))
            .and_then(|r| r.as_str())
            .map(|r| r.split('/').next_back().unwrap_or("").to_string());

        let example = schema_obj
            .get("example")
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
    use crate::discovery::models::{AuthRequirement, ParameterLocation};

    fn write_spec(dir: &std::path::Path, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(filename);
        std::fs::write(&path, content).unwrap();
        path
    }

    // ── Basic parsing ─────────────────────────────────────────────────────────

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
        let spec_path = write_spec(temp_dir.path(), "openapi.yaml", yaml);

        let project = parse_openapi(&spec_path).unwrap();
        assert_eq!(project.title, Some("Test API".to_string()));
        assert_eq!(project.endpoints.len(), 2);
    }

    #[test]
    fn test_parse_version_extracted() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: My API
  version: 2.3.1
paths: {}
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();
        assert_eq!(project.version, Some("2.3.1".to_string()));
    }

    // ── Servers → base_url ────────────────────────────────────────────────────

    #[test]
    fn test_parse_servers_sets_base_url() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
servers:
  - url: https://api.production.com/v1
  - url: https://api.staging.com/v1
paths: {}
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();
        // First server wins
        assert_eq!(
            project.base_url,
            Some("https://api.production.com/v1".to_string())
        );
    }

    // ── Parameters ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_path_and_query_parameters() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
        - name: include_deleted
          in: query
          required: false
          schema:
            type: boolean
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        let endpoint = &project.endpoints[0];
        assert_eq!(endpoint.parameters.len(), 2);

        let path_param = endpoint
            .parameters
            .iter()
            .find(|p| p.name == "id")
            .unwrap();
        assert_eq!(path_param.location, ParameterLocation::Path);
        assert!(path_param.required);
        assert_eq!(path_param.param_type, "integer");

        let query_param = endpoint
            .parameters
            .iter()
            .find(|p| p.name == "include_deleted")
            .unwrap();
        assert_eq!(query_param.location, ParameterLocation::Query);
        assert!(!query_param.required);
    }

    // ── Request body ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_request_body_with_ref() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateUserRequest'
      responses:
        201:
          description: Created
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        let body = project.endpoints[0].body.as_ref().unwrap();
        assert_eq!(body.content_type, "application/json");
        assert_eq!(body.schema_name, Some("CreateUserRequest".to_string()));
        assert!(body.required);
    }

    // ── Security schemes ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_bearer_security_scheme() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
security:
  - BearerAuth: []
paths:
  /secure:
    get:
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        assert_eq!(project.endpoints[0].auth, AuthRequirement::Bearer);
    }

    #[test]
    fn test_parse_apikey_security_scheme() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
components:
  securitySchemes:
    ApiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key
security:
  - ApiKeyAuth: []
paths:
  /items:
    get:
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        assert!(matches!(
            project.endpoints[0].auth,
            AuthRequirement::ApiKey { .. }
        ));
    }

    #[test]
    fn test_global_security_propagates_to_all_endpoints() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
security:
  - BearerAuth: []
paths:
  /a:
    get:
      responses:
        200:
          description: OK
  /b:
    post:
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        for endpoint in &project.endpoints {
            assert_eq!(
                endpoint.auth,
                AuthRequirement::Bearer,
                "endpoint {} should inherit global Bearer auth",
                endpoint.path
            );
        }
    }

    // ── Deprecated ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_deprecated_endpoint() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths:
  /old-endpoint:
    get:
      deprecated: true
      summary: Old way to get data
      responses:
        200:
          description: OK
  /new-endpoint:
    get:
      summary: New way to get data
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        let deprecated = project
            .endpoints
            .iter()
            .find(|e| e.path == "/old-endpoint")
            .unwrap();
        let active = project
            .endpoints
            .iter()
            .find(|e| e.path == "/new-endpoint")
            .unwrap();

        assert!(deprecated.deprecated);
        assert!(!active.deprecated);
    }

    // ── JSON format ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_json_format_spec() {
        let json = r#"{
  "openapi": "3.0.0",
  "info": { "title": "JSON API", "version": "1.0.0" },
  "paths": {
    "/items": {
      "get": {
        "summary": "List items",
        "responses": { "200": { "description": "OK" } }
      }
    }
  }
}"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.json", json);
        let project = parse_openapi(&path).unwrap();

        assert_eq!(project.title, Some("JSON API".to_string()));
        assert_eq!(project.endpoints.len(), 1);
        assert_eq!(project.endpoints[0].path, "/items");
    }

    // ── Tags ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_endpoint_tags() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths:
  /users:
    get:
      tags:
        - users
        - public
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();

        assert_eq!(project.endpoints[0].tags, vec!["users", "public"]);
    }

    // ── non-HTTP keys skipped ─────────────────────────────────────────────────

    #[test]
    fn test_path_level_parameters_key_not_counted_as_endpoint() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths:
  /users/{id}:
    parameters:
      - name: id
        in: path
        required: true
        schema:
          type: integer
    get:
      responses:
        200:
          description: OK
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec(dir.path(), "openapi.yaml", yaml);
        let project = parse_openapi(&path).unwrap();
        // Only GET should be an endpoint — "parameters" key must be skipped
        assert_eq!(project.endpoints.len(), 1);
        assert_eq!(project.endpoints[0].method, "GET");
    }
}
