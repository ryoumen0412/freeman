use crate::models::{AuthType, Header, HttpMethod, Request};
use anyhow::{anyhow, Result};

/// Parse a cURL command into a Request
pub fn parse_curl(input: &str) -> Result<Request> {
    let mut request = Request::default();
    
    // Remove line continuations and normalize
    let normalized = input
        .replace("\\\n", " ")
        .replace("\\\r\n", " ");
    
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
                    if !request.headers.iter().any(|h| h.key.to_lowercase() == header.key.to_lowercase()) {
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
            "--compressed" | "-k" | "--insecure" | "-L" | "--location" | "-s" | "--silent" | "-v" | "--verbose" => {
                // Ignored flags
            }
            _ => {
                // Check for URL (doesn't start with -)
                if !token.starts_with('-') && (token.starts_with("http://") || token.starts_with("https://") || token.starts_with("'http") || token.starts_with("\"http")) {
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
}
