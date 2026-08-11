//! HTTP client wrapper - executes requests and formats responses

use base64::Engine;
use futures_util::StreamExt;
use std::error::Error;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use crate::messages::NetworkResponse;
use crate::models::{AuthType, Environment, HttpMethod, Request};

/// Format detailed error messages for HTTP request failures
fn format_request_error(e: &reqwest::Error, url: &str) -> String {
    let mut lines = Vec::new();

    // Main error classification
    if e.is_timeout() {
        lines.push("⏱ TIMEOUT: Request timed out after 30 seconds".to_string());
        lines.push("  → The server took too long to respond".to_string());
        lines.push("  → Check if the server is running and accessible".to_string());
    } else if e.is_connect() {
        lines.push("🔌 CONNECTION FAILED: Unable to connect to server".to_string());

        // Try to extract more specific connection error info
        let error_str = e.to_string().to_lowercase();

        if error_str.contains("dns")
            || error_str.contains("name resolution")
            || error_str.contains("getaddrinfo")
        {
            lines.push("  → DNS resolution failed - hostname not found".to_string());
            lines.push("  → Verify the hostname is correct".to_string());
        } else if error_str.contains("refused") || error_str.contains("111") {
            lines.push("  → Connection refused by the server".to_string());
            lines.push("  → The server may not be running on this port".to_string());
        } else if error_str.contains("no route") || error_str.contains("network unreachable") {
            lines.push("  → Network unreachable".to_string());
            lines.push("  → Check your network connection".to_string());
        } else if error_str.contains("reset") {
            lines.push("  → Connection was reset by the server".to_string());
            lines.push("  → The server forcibly closed the connection".to_string());
        } else {
            lines.push(format!("  → {}", e));
        }
    } else if e.is_request() {
        lines.push("📝 REQUEST ERROR: Failed to build/send request".to_string());

        // Check for specific request issues
        if e.is_body() {
            lines.push("  → Problem with request body".to_string());
        }

        if let Some(source) = e.source() {
            lines.push(format!("  → {}", source));
        }
    } else if e.is_redirect() {
        lines.push("↪ REDIRECT ERROR: Too many redirects or redirect loop".to_string());
        lines.push("  → The server redirected too many times".to_string());
        lines.push("  → Check for redirect loops in server config".to_string());
    } else if e.is_status() {
        // This shouldn't normally happen here since we handle status in Ok branch
        if let Some(status) = e.status() {
            lines.push(format!("❌ HTTP ERROR: Status {}", status.as_u16()));
            lines.push(format!(
                "  → {}",
                status.canonical_reason().unwrap_or("Unknown")
            ));
        }
    } else if e.is_decode() {
        lines.push("📦 DECODE ERROR: Failed to decode response".to_string());
        lines.push("  → Response body could not be parsed".to_string());
    } else {
        // Generic error with full details
        lines.push("❓ REQUEST FAILED".to_string());

        let error_str = e.to_string().to_lowercase();

        // Check for TLS/SSL errors
        if error_str.contains("ssl")
            || error_str.contains("tls")
            || error_str.contains("certificate")
        {
            lines.push("  → TLS/SSL error detected".to_string());
            if error_str.contains("certificate") && error_str.contains("expired") {
                lines.push("  → Server certificate may be expired".to_string());
            } else if error_str.contains("self-signed") || error_str.contains("unknown ca") {
                lines.push("  → Server has an untrusted certificate".to_string());
            } else if error_str.contains("handshake") {
                lines.push("  → TLS handshake failed".to_string());
            }
        }

        lines.push(format!("  → {}", e));
    }

    // Add URL info for context
    lines.push(String::new());
    lines.push(format!("URL: {}", url));

    // Check for common URL issues
    if url.starts_with("https://localhost") || url.starts_with("https://127.0.0.1") {
        lines.push("  ⚠ TIP: Local servers often run on HTTP, not HTTPS".to_string());
    }

    if !url.contains("://") {
        lines.push("  ⚠ TIP: URL may be missing the protocol (http:// or https://)".to_string());
    }

    lines.join("\n")
}

/// Build a request from the given parameters
fn build_request(
    client: &reqwest::Client,
    request: &Request,
    environment: &Option<Environment>,
) -> reqwest::RequestBuilder {
    // Apply environment variable substitution
    let url = if let Some(env) = environment {
        env.substitute(&request.url)
    } else {
        request.url.clone()
    };

    let mut req_builder = match request.method {
        HttpMethod::GET => client.get(&url),
        HttpMethod::POST => client.post(&url),
        HttpMethod::PUT => client.put(&url),
        HttpMethod::PATCH => client.patch(&url),
        HttpMethod::DELETE => client.delete(&url),
    };

    // Add headers
    for header in &request.headers {
        if header.enabled {
            let value = if let Some(env) = environment {
                env.substitute(&header.value)
            } else {
                header.value.clone()
            };
            req_builder = req_builder.header(&header.key, value);
        }
    }

    // Add auth
    match &request.auth {
        AuthType::Bearer(token) => {
            let token = if let Some(env) = environment {
                env.substitute(token)
            } else {
                token.clone()
            };
            req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
        }
        AuthType::Basic { username, password } => {
            let credentials = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
            req_builder = req_builder.header("Authorization", format!("Basic {}", encoded));
        }
        AuthType::None => {}
    }

    // Add body
    if request.method.has_body() && !request.body.is_empty() {
        let body = if let Some(env) = environment {
            env.substitute(&request.body)
        } else {
            request.body.clone()
        };
        req_builder = req_builder.body(body);
    }

    req_builder
}

/// Execute an HTTP request and return the response (buffered)
pub async fn execute_request(
    client: &reqwest::Client,
    request: Request,
    environment: Option<Environment>,
    request_id: u64,
) -> NetworkResponse {
    let start = Instant::now();
    let req_builder = build_request(client, &request, &environment);

    let result = req_builder.send().await;
    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match resp.text().await {
                Ok(body) => {
                    let formatted =
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                            serde_json::to_string_pretty(&json).unwrap_or(body)
                        } else {
                            body
                        };
                    NetworkResponse::Success {
                        id: request_id,
                        status,
                        body: formatted,
                        time_ms: elapsed,
                    }
                }
                Err(e) => NetworkResponse::Error {
                    id: request_id,
                    message: format!("Error reading body: {}", e),
                    time_ms: elapsed,
                },
            }
        }
        Err(e) => {
            let msg = format_request_error(&e, &request.url);
            NetworkResponse::Error {
                id: request_id,
                message: msg,
                time_ms: elapsed,
            }
        }
    }
}

/// Execute an HTTP request with streaming response
pub async fn execute_streaming_request(
    client: &reqwest::Client,
    request: Request,
    environment: Option<Environment>,
    request_id: u64,
    response_tx: mpsc::UnboundedSender<NetworkResponse>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let start = Instant::now();
    let req_builder = build_request(client, &request, &environment);

    let result = req_builder.send().await;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let mut stream = resp.bytes_stream();
            let mut total_bytes = 0usize;
            let mut body = String::new();
            let mut utf8_buffer = Vec::<u8>::new();

            loop {
                tokio::select! {
                    biased;

                    _ = &mut cancel_rx => {
                        // Request was cancelled
                        return;
                    }
                    chunk = stream.next() => {
                        match chunk {
                            Some(Ok(bytes)) => {
                                total_bytes += bytes.len();
                                utf8_buffer.extend_from_slice(&bytes);

                                match std::str::from_utf8(&utf8_buffer) {
                                    Ok(valid_str) => {
                                        let chunk_text = valid_str.to_string();
                                        body.push_str(&chunk_text);
                                        utf8_buffer.clear();
                                        if !chunk_text.is_empty() {
                                            let _ = response_tx.send(NetworkResponse::StreamChunk {
                                                id: request_id,
                                                chunk: chunk_text,
                                                bytes_received: total_bytes,
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        let valid_up_to = e.valid_up_to();
                                        if valid_up_to > 0 {
                                            if let Ok(valid_str) = std::str::from_utf8(&utf8_buffer[..valid_up_to]) {
                                                let chunk_text = valid_str.to_string();
                                                body.push_str(&chunk_text);
                                                utf8_buffer.drain(..valid_up_to);
                                                if !chunk_text.is_empty() {
                                                    let _ = response_tx.send(NetworkResponse::StreamChunk {
                                                        id: request_id,
                                                        chunk: chunk_text,
                                                        bytes_received: total_bytes,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                let _ = response_tx.send(NetworkResponse::Error {
                                    id: request_id,
                                    message: format!("Stream error: {}", e),
                                    time_ms: start.elapsed().as_millis() as u64,
                                });
                                return;
                            }
                            None => {
                                // Flush any remaining bytes
                                if !utf8_buffer.is_empty() {
                                    body.push_str(&String::from_utf8_lossy(&utf8_buffer));
                                    utf8_buffer.clear();
                                }

                                // Stream complete - try to format as JSON
                                let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.clone())
                                } else {
                                    body
                                };

                                // Send final formatted body as success
                                let _ = response_tx.send(NetworkResponse::Success {
                                    id: request_id,
                                    status,
                                    body: formatted,
                                    time_ms: start.elapsed().as_millis() as u64,
                                });
                                return;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            let msg = format_request_error(&e, &request.url);
            let _ = response_tx.send(NetworkResponse::Error {
                id: request_id,
                message: msg,
                time_ms: start.elapsed().as_millis() as u64,
            });
        }
    }
}

/// Execute a GraphQL query
pub async fn execute_graphql(
    client: &reqwest::Client,
    endpoint: String,
    query: String,
    variables: Option<String>,
    headers: Vec<crate::models::Header>,
    auth: crate::models::AuthType,
    request_id: u64,
) -> NetworkResponse {
    use base64::Engine;
    use std::time::Instant;

    let start = Instant::now();

    // Build GraphQL request body
    let body = if let Some(vars) = &variables {
        // Try to parse variables as JSON
        match serde_json::from_str::<serde_json::Value>(vars) {
            Ok(vars_json) => serde_json::json!({
                "query": query,
                "variables": vars_json
            }),
            Err(_) => serde_json::json!({
                "query": query
            }),
        }
    } else {
        serde_json::json!({
            "query": query
        })
    };

    // Build request
    let mut req_builder = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .json(&body);

    // Add custom headers
    for header in &headers {
        if header.enabled {
            req_builder = req_builder.header(&header.key, &header.value);
        }
    }

    // Add auth
    match &auth {
        crate::models::AuthType::Bearer(token) => {
            if !token.is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
            }
        }
        crate::models::AuthType::Basic { username, password } => {
            let credentials = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
            req_builder = req_builder.header("Authorization", format!("Basic {}", encoded));
        }
        crate::models::AuthType::None => {}
    }

    let result = req_builder.send().await;
    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match resp.text().await {
                Ok(body) => {
                    // Pretty-print JSON response
                    let formatted =
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                            serde_json::to_string_pretty(&json).unwrap_or(body)
                        } else {
                            body
                        };
                    NetworkResponse::Success {
                        id: request_id,
                        status,
                        body: formatted,
                        time_ms: elapsed,
                    }
                }
                Err(e) => NetworkResponse::Error {
                    id: request_id,
                    message: format!("Error reading response: {}", e),
                    time_ms: elapsed,
                },
            }
        }
        Err(e) => {
            let msg = format_request_error(&e, &endpoint);
            NetworkResponse::Error {
                id: request_id,
                message: msg,
                time_ms: elapsed,
            }
        }
    }
}

/// Create an HTTP client with default configuration
pub fn create_client() -> reqwest::Client {
    use std::time::Duration;

    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Create an HTTP client that ignores SSL certificate errors
/// WARNING: Only use for testing environments, not production!
pub fn create_insecure_client() -> reqwest::Client {
    use std::time::Duration;

    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
