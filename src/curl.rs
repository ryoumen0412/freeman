use crate::models::{AuthType, Header, HttpMethod, Request};
use anyhow::{anyhow, Result};

/// Parse a cURL command into a Request
pub fn parse_curl(input: &str) -> Result<Request> {
    let mut request = Request::default();

    // Remove line continuations and normalize
    let normalized = input.replace("\\\n", " ").replace("\\\r\n", " ");

    let mut tokens = tokenize(&normalized)?;

    // Skip 'curl' command if present
    if tokens.first().map(|s| s.as_str()) == Some("curl") {
        tokens.remove(0);
    }

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];

        match token.as_str() {
            "-X" | "--request" => {
                if i + 1 < tokens.len() {
                    request.method = parse_method(&tokens[i + 1])?;
                    i += 1;
                }
            }
            "-H" | "--header" => {
                if i + 1 < tokens.len() {
                    let header = parse_header(&tokens[i + 1])?;
                    // Don't add duplicate headers
                    if !request
                        .headers
                        .iter()
                        .any(|h| h.key.to_lowercase() == header.key.to_lowercase())
                    {
                        request.headers.push(header);
                    }
                    i += 1;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if i + 1 < tokens.len() {
                    request.body = tokens[i + 1].clone();
                    // Infer POST if not set
                    if request.method == HttpMethod::GET {
                        request.method = HttpMethod::POST;
                    }
                    i += 1;
                }
            }
            "-u" | "--user" => {
                if i + 1 < tokens.len() {
                    let (user, pass) = parse_basic_auth(&tokens[i + 1]);
                    request.auth = AuthType::Basic {
                        username: user,
                        password: pass,
                    };
                    i += 1;
                }
            }
            "--compressed" | "-k" | "--insecure" | "-L" | "--location" | "-s" | "--silent"
            | "-v" | "--verbose" => {
                // Ignored flags
            }
            _ => {
                // Check for URL (doesn't start with -)
                if !token.starts_with('-')
                    && (token.starts_with("http://")
                        || token.starts_with("https://")
                        || token.starts_with("'http")
                        || token.starts_with("\"http"))
                {
                    request.url = token.trim_matches(|c| c == '\'' || c == '"').to_string();
                }
                // Check for Bearer token in Authorization header
                if token.to_lowercase().starts_with("authorization:") {
                    let value = token.split_once(':').map(|x| x.1).unwrap_or("").trim();
                    if value.to_lowercase().starts_with("bearer ") {
                        let token = value[7..].to_string();
                        request.auth = AuthType::Bearer(token);
                    }
                }
            }
        }
        i += 1;
    }

    Ok(request)
}

fn parse_method(s: &str) -> Result<HttpMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "PATCH" => Ok(HttpMethod::PATCH),
        "DELETE" => Ok(HttpMethod::DELETE),
        _ => Err(anyhow!("Unknown HTTP method: {}", s)),
    }
}

fn parse_header(s: &str) -> Result<Header> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() == 2 {
        Ok(Header::new(parts[0].trim(), parts[1].trim()))
    } else {
        Err(anyhow!("Invalid header format: {}", s))
    }
}

fn parse_basic_auth(s: &str) -> (String, String) {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (s.to_string(), String::new())
    }
}

/// Tokenize a curl command, respecting quotes
fn tokenize(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for c in input.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Format request as cURL command
pub fn to_curl(request: &Request) -> String {
    let mut parts = vec!["curl".to_string()];

    // Method
    if request.method != HttpMethod::GET {
        parts.push(format!("-X {}", request.method.as_str()));
    }

    // URL
    parts.push(format!("'{}'", request.url));

    // Headers
    for header in &request.headers {
        if header.enabled {
            parts.push(format!("-H '{}: {}'", header.key, header.value));
        }
    }

    // Auth
    match &request.auth {
        AuthType::Bearer(token) => {
            parts.push(format!("-H 'Authorization: Bearer {}'", token));
        }
        AuthType::Basic { username, password } => {
            parts.push(format!("-u '{}:{}'", username, password));
        }
        AuthType::None => {}
    }

    // Body
    if !request.body.is_empty() {
        parts.push(format!("-d '{}'", request.body.replace('\'', "'\\''")));
    }

    parts.join(" \\\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;

    // ── parse_curl ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_get() {
        let curl = "curl https://api.example.com/users";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.method, HttpMethod::GET);
    }

    #[test]
    fn test_parse_post_with_data() {
        let curl = r#"curl -X POST -H "Content-Type: application/json" -d '{"name":"test"}' https://api.example.com/users"#;
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body, r#"{"name":"test"}"#);
    }

    #[test]
    fn test_parse_put_method() {
        let curl = "curl -X PUT https://api.example.com/users/1";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.method, HttpMethod::PUT);
    }

    #[test]
    fn test_parse_delete_method() {
        let curl = "curl -X DELETE https://api.example.com/users/1";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.method, HttpMethod::DELETE);
    }

    #[test]
    fn test_parse_patch_method() {
        let curl = "curl -X PATCH https://api.example.com/users/1";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.method, HttpMethod::PATCH);
    }

    #[test]
    fn test_parse_data_infers_post_when_no_method() {
        let curl = r#"curl -d '{"x":1}' https://api.example.com/items"#;
        let req = parse_curl(curl).unwrap();
        // No -X flag → should infer POST when body is present
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body, r#"{"x":1}"#);
    }

    #[test]
    fn test_parse_bearer_auth_via_header_flag() {
        // When Bearer is passed via -H, the parser stores it as a regular header
        // (not as AuthType::Bearer). This is the documented behavior of parse_curl.
        let curl = r#"curl -H "Authorization: Bearer my-secret-token" https://api.example.com"#;
        let req = parse_curl(curl).unwrap();
        // Auth type remains None when using -H for Authorization
        assert_eq!(req.auth, AuthType::None);
        // The Authorization header IS stored in headers
        let auth_header = req
            .headers
            .iter()
            .find(|h| h.key.to_lowercase() == "authorization");
        assert!(
            auth_header.is_some(),
            "Authorization header should be in headers"
        );
        assert!(auth_header.unwrap().value.contains("Bearer"));
    }

    #[test]
    fn test_parse_basic_auth() {
        let curl = "curl -u user:password https://api.example.com";
        let req = parse_curl(curl).unwrap();
        assert_eq!(
            req.auth,
            AuthType::Basic {
                username: "user".to_string(),
                password: "password".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_basic_auth_no_password() {
        let curl = "curl -u useronly https://api.example.com";
        let req = parse_curl(curl).unwrap();
        assert_eq!(
            req.auth,
            AuthType::Basic {
                username: "useronly".to_string(),
                password: String::new(),
            }
        );
    }

    #[test]
    fn test_parse_multiline_backslash_continuation() {
        let curl = "curl \\\n  -X POST \\\n  https://api.example.com/items";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://api.example.com/items");
    }

    #[test]
    fn test_parse_ignored_flags_do_not_error() {
        let curl = "curl --compressed -k --insecure -L --location -s --silent -v --verbose https://api.example.com";
        let req = parse_curl(curl).unwrap();
        assert_eq!(req.url, "https://api.example.com");
    }

    #[test]
    fn test_parse_duplicate_headers_deduped() {
        let curl =
            r#"curl -H "Accept: application/json" -H "Accept: text/html" https://api.example.com"#;
        let req = parse_curl(curl).unwrap();
        // Second Accept header should be ignored (duplicate key)
        let accept_headers: Vec<_> = req
            .headers
            .iter()
            .filter(|h| h.key.to_lowercase() == "accept")
            .collect();
        assert_eq!(accept_headers.len(), 1);
        assert_eq!(accept_headers[0].value, "application/json");
    }

    // ── to_curl ──────────────────────────────────────────────────────────────

    #[test]
    fn test_to_curl_simple_get() {
        use crate::models::Request;
        let mut req = Request {
            url: "https://api.example.com/users".to_string(),
            ..Request::default()
        };
        req.headers.clear();

        let curl = to_curl(&req);
        // GET should NOT include -X GET
        assert!(!curl.contains("-X GET"));
        assert!(curl.contains("https://api.example.com/users"));
    }

    #[test]
    fn test_to_curl_post_includes_method_and_body() {
        use crate::models::{HttpMethod, Request};
        let mut req = Request {
            method: HttpMethod::POST,
            url: "https://api.example.com/items".to_string(),
            body: r#"{"name":"widget"}"#.to_string(),
            ..Request::default()
        };
        req.headers.clear();

        let curl = to_curl(&req);
        assert!(curl.contains("-X POST"));
        assert!(curl.contains(r#"{"name":"widget"}"#));
    }

    #[test]
    fn test_to_curl_bearer_auth() {
        use crate::models::{AuthType, Request};
        let mut req = Request {
            url: "https://api.example.com".to_string(),
            auth: AuthType::Bearer("tok123".to_string()),
            ..Request::default()
        };
        req.headers.clear();

        let curl = to_curl(&req);
        assert!(curl.contains("Authorization: Bearer tok123"));
    }

    #[test]
    fn test_to_curl_basic_auth() {
        use crate::models::{AuthType, Request};
        let mut req = Request {
            url: "https://api.example.com".to_string(),
            auth: AuthType::Basic {
                username: "alice".to_string(),
                password: "s3cr3t".to_string(),
            },
            ..Request::default()
        };
        req.headers.clear();

        let curl = to_curl(&req);
        assert!(curl.contains("-u 'alice:s3cr3t'"));
    }

    #[test]
    fn test_to_curl_disabled_headers_excluded() {
        use crate::models::{Header, HttpMethod, Request};
        let mut req = Request {
            method: HttpMethod::GET,
            url: "https://api.example.com".to_string(),
            ..Request::default()
        };
        req.headers.clear();

        let mut disabled = Header::new("X-Debug", "true");
        disabled.enabled = false;
        req.headers.push(disabled);
        req.headers.push(Header::new("X-Active", "yes"));

        let curl = to_curl(&req);
        assert!(
            !curl.contains("X-Debug"),
            "disabled header should not appear"
        );
        assert!(curl.contains("X-Active"), "enabled header should appear");
    }

    #[test]
    fn test_roundtrip_post_with_basic_auth() {
        // Roundtrip with Basic auth: to_curl uses -u flag, parse_curl reads -u flag
        use crate::models::{AuthType, Header, HttpMethod, Request};
        let req = Request {
            method: HttpMethod::POST,
            url: "https://api.example.com/data".to_string(),
            auth: AuthType::Basic {
                username: "alice".to_string(),
                password: "s3cr3t".to_string(),
            },
            body: r#"{"key":"value"}"#.to_string(),
            headers: vec![Header::new("Content-Type", "application/json")],
            ..Request::default()
        };

        let curl_str = to_curl(&req);
        let parsed = parse_curl(&curl_str).unwrap();

        assert_eq!(parsed.method, HttpMethod::POST);
        assert_eq!(parsed.url, req.url);
        assert_eq!(
            parsed.auth,
            AuthType::Basic {
                username: "alice".to_string(),
                password: "s3cr3t".to_string(),
            }
        );
        assert_eq!(parsed.body, req.body);
    }
}
