//! HTTP client wrapper - executes requests and formats responses

use std::time::Instant;
use base64::Engine;
use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot};

use crate::models::{AuthType, Environment, HttpMethod, Request};
use crate::messages::NetworkResponse;

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
                    let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
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
            let msg = if e.is_timeout() {
                "Request timed out (30s)".to_string()
            } else if e.is_connect() {
                format!("Connection failed: {}", e)
            } else {
                format!("Request failed: {}", e)
            };
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
                                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                                    body.push_str(&text);
                                    let _ = response_tx.send(NetworkResponse::StreamChunk {
                                        id: request_id,
                                        chunk: text,
                                        bytes_received: total_bytes,
                                    });
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
                                // Stream complete - try to format as JSON
                                let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                                    serde_json::to_string_pretty(&json).unwrap_or(body)
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
            let msg = if e.is_timeout() {
                "Request timed out (30s)".to_string()
            } else if e.is_connect() {
                format!("Connection failed: {}", e)
            } else {
                format!("Request failed: {}", e)
            };
            let _ = response_tx.send(NetworkResponse::Error {
                id: request_id,
                message: msg,
                time_ms: start.elapsed().as_millis() as u64,
            });
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
