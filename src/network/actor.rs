//! Network actor - runs HTTP requests and WebSockets in Tokio async runtime

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

use crate::messages::{NetworkCommand, NetworkResponse};
use crate::network::client::{create_client, execute_request, execute_streaming_request};
use crate::network::websocket::connect_websocket;

/// Tracks an active request for cancellation
#[allow(dead_code)]
struct ActiveRequest {
    cancel_tx: oneshot::Sender<()>,
}

/// Tracks an active WebSocket connection
#[allow(dead_code)]
struct ActiveWebSocket {
    message_tx: mpsc::UnboundedSender<String>,
    cancel_tx: oneshot::Sender<()>,
}

/// Network actor that processes HTTP request and WebSocket commands
pub struct NetworkActor {
    client: reqwest::Client,
    response_tx: mpsc::UnboundedSender<NetworkResponse>,
    active_requests: JoinSet<()>,
    cancel_handles: HashMap<u64, ActiveRequest>,
    websockets: HashMap<u64, ActiveWebSocket>,
}

impl NetworkActor {
    pub fn new(response_tx: mpsc::UnboundedSender<NetworkResponse>) -> Self {
        NetworkActor {
            client: create_client(),
            response_tx,
            active_requests: JoinSet::new(),
            cancel_handles: HashMap::new(),
            websockets: HashMap::new(),
        }
    }

    /// Run the network actor message loop
    pub async fn run(mut self, mut cmd_rx: mpsc::UnboundedReceiver<NetworkCommand>) {
        loop {
            tokio::select! {
                biased;

                // Handle incoming commands
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(NetworkCommand::ExecuteRequest { id, request, environment }) => {
                            let response_tx = self.response_tx.clone();
                            let client = self.client.clone();

                            // Simple buffered request - no cancellation tracking
                            self.active_requests.spawn(async move {
                                tracing::info!(id, url = %request.url, method = ?request.method, "Executing request");
                                let result = execute_request(&client, request, environment, id).await;
                                tracing::info!(id, status = ?result.id(), "Request completed");
                                let _ = response_tx.send(result);
                            });
                        }

                        Some(NetworkCommand::ExecuteStreamingRequest { id, request, environment }) => {
                            let (cancel_tx, cancel_rx) = oneshot::channel();
                            self.cancel_handles.insert(id, ActiveRequest { cancel_tx });

                            let response_tx = self.response_tx.clone();
                            let client = self.client.clone();

                            self.active_requests.spawn(async move {
                                execute_streaming_request(
                                    &client,
                                    request,
                                    environment,
                                    id,
                                    response_tx,
                                    cancel_rx,
                                ).await;
                            });
                        }

                        Some(NetworkCommand::CancelRequest(id)) => {
                            if let Some(active) = self.cancel_handles.remove(&id) {
                                tracing::info!(id, "Cancelling request");
                                let _ = active.cancel_tx.send(());
                                let _ = self.response_tx.send(NetworkResponse::Cancelled { id });
                            }
                        }

                        Some(NetworkCommand::ConnectWebSocket { id, url }) => {
                            let (cancel_tx, cancel_rx) = oneshot::channel();
                            let (message_tx, message_rx) = mpsc::unbounded_channel();

                            self.websockets.insert(id, ActiveWebSocket {
                                message_tx,
                                cancel_tx,
                            });

                            let response_tx = self.response_tx.clone();

                            self.active_requests.spawn(async move {
                                connect_websocket(id, &url, response_tx, message_rx, cancel_rx).await;
                            });
                        }

                        Some(NetworkCommand::SendWebSocketMessage { id, message }) => {
                            if let Some(ws) = self.websockets.get(&id) {
                                let _ = ws.message_tx.send(message);
                            }
                        }

                        Some(NetworkCommand::CloseWebSocket(id)) => {
                            if let Some(ws) = self.websockets.remove(&id) {
                                let _ = ws.cancel_tx.send(());
                            }
                        }

                        Some(NetworkCommand::ExecuteGraphQL { id, endpoint, query, variables, headers, auth }) => {
                            let response_tx = self.response_tx.clone();
                            let client = self.client.clone();

                            self.active_requests.spawn(async move {
                                tracing::info!(id, endpoint = %endpoint, "Executing GraphQL query");
                                let result = crate::network::client::execute_graphql(
                                    &client,
                                    endpoint,
                                    query,
                                    variables,
                                    headers,
                                    auth,
                                    id,
                                ).await;
                                tracing::info!(id, "GraphQL query completed");
                                let _ = response_tx.send(result);
                            });
                        }

                        Some(NetworkCommand::Shutdown) => {
                            // Cancel all active requests
                            for (_, active) in self.cancel_handles.drain() {
                                let _ = active.cancel_tx.send(());
                            }
                            // Close all WebSockets
                            for (_, ws) in self.websockets.drain() {
                                let _ = ws.cancel_tx.send(());
                            }
                            break;
                        }

                        None => break,
                    }
                }

                // Clean up completed tasks
                Some(_result) = self.active_requests.join_next() => {
                    // Task completed - cleanup is handled by the tasks themselves
                }
            }
        }
    }
}
