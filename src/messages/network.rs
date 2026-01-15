//! Network messages - communication between App and Network layers

use crate::models::{Environment, Request};

/// Commands sent from App layer to Network layer
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    /// Execute an HTTP request (buffered, for small responses)
    ExecuteRequest {
        id: u64,
        request: Request,
        environment: Option<Environment>,
    },
    /// Execute an HTTP request with streaming (for large responses)
    ExecuteStreamingRequest {
        id: u64,
        request: Request,
        environment: Option<Environment>,
    },
    /// Cancel a pending request
    CancelRequest(u64),
    
    // WebSocket commands
    /// Connect to a WebSocket server
    ConnectWebSocket {
        id: u64,
        url: String,
    },
    /// Send a message through an active WebSocket connection
    SendWebSocketMessage {
        id: u64,
        message: String,
    },
    /// Close a WebSocket connection
    CloseWebSocket(u64),
    
    /// Shutdown the network actor
    Shutdown,
}

/// Responses sent from Network layer to App layer
#[derive(Debug, Clone)]
pub enum NetworkResponse {
    /// Successful HTTP response (complete)
    Success {
        id: u64,
        status: u16,
        body: String,
        time_ms: u64,
    },
    /// Streaming chunk received
    StreamChunk {
        id: u64,
        chunk: String,
        bytes_received: usize,
    },
    /// Streaming complete
    StreamComplete {
        id: u64,
        status: u16,
        total_bytes: usize,
        time_ms: u64,
    },
    /// Error response
    Error {
        id: u64,
        message: String,
        time_ms: u64,
    },
    /// Request was cancelled
    Cancelled {
        id: u64,
    },
    
    // WebSocket responses
    /// WebSocket connection established
    WebSocketConnected {
        id: u64,
    },
    /// WebSocket message received
    WebSocketMessage {
        id: u64,
        message: String,
    },
    /// WebSocket connection closed
    WebSocketClosed {
        id: u64,
    },
    /// WebSocket error
    WebSocketError {
        id: u64,
        error: String,
    },
}

impl NetworkResponse {
    /// Get the request ID from the response
    pub fn id(&self) -> u64 {
        match self {
            NetworkResponse::Success { id, .. } => *id,
            NetworkResponse::StreamChunk { id, .. } => *id,
            NetworkResponse::StreamComplete { id, .. } => *id,
            NetworkResponse::Error { id, .. } => *id,
            NetworkResponse::Cancelled { id } => *id,
            NetworkResponse::WebSocketConnected { id } => *id,
            NetworkResponse::WebSocketMessage { id, .. } => *id,
            NetworkResponse::WebSocketClosed { id } => *id,
            NetworkResponse::WebSocketError { id, .. } => *id,
        }
    }
    
    /// Check if this is a terminal response (no more messages expected for this id)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            NetworkResponse::Success { .. }
                | NetworkResponse::StreamComplete { .. }
                | NetworkResponse::Error { .. }
                | NetworkResponse::Cancelled { .. }
                | NetworkResponse::WebSocketClosed { .. }
                | NetworkResponse::WebSocketError { .. }
        )
    }
}
