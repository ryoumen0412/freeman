//! WebSocket client - connects to WebSocket servers

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::messages::NetworkResponse;

/// Connect to a WebSocket server and handle bidirectional communication
pub async fn connect_websocket(
    id: u64,
    url: &str,
    response_tx: mpsc::UnboundedSender<NetworkResponse>,
    mut message_rx: mpsc::UnboundedReceiver<String>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    // Attempt to connect
    let ws_stream = match connect_async(url).await {
        Ok((stream, _response)) => stream,
        Err(e) => {
            let _ = response_tx.send(NetworkResponse::WebSocketError {
                id,
                error: format!("Connection failed: {}", e),
            });
            return;
        }
    };

    // Notify successful connection
    let _ = response_tx.send(NetworkResponse::WebSocketConnected { id });

    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            biased;
            
            // Check for cancellation/close request
            _ = &mut cancel_rx => {
                let _ = write.close().await;
                let _ = response_tx.send(NetworkResponse::WebSocketClosed { id });
                return;
            }
            
            // Handle outgoing messages
            Some(msg) = message_rx.recv() => {
                if let Err(e) = write.send(Message::Text(msg)).await {
                    let _ = response_tx.send(NetworkResponse::WebSocketError {
                        id,
                        error: format!("Send failed: {}", e),
                    });
                    return;
                }
            }
            
            // Handle incoming messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let _ = response_tx.send(NetworkResponse::WebSocketMessage {
                            id,
                            message: text,
                        });
                    }
                    Some(Ok(Message::Binary(data))) => {
                        // Convert binary to hex representation
                        let hex = data.iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let _ = response_tx.send(NetworkResponse::WebSocketMessage {
                            id,
                            message: format!("[Binary: {} bytes]\n{}", data.len(), hex),
                        });
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond with pong
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Ignore pong responses
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| format!("{}: {}", f.code, f.reason))
                            .unwrap_or_else(|| "Connection closed".to_string());
                        let _ = response_tx.send(NetworkResponse::WebSocketMessage {
                            id,
                            message: format!("[Closed: {}]", reason),
                        });
                        let _ = response_tx.send(NetworkResponse::WebSocketClosed { id });
                        return;
                    }
                    Some(Ok(Message::Frame(_))) => {
                        // Raw frame, ignore
                    }
                    Some(Err(e)) => {
                        let _ = response_tx.send(NetworkResponse::WebSocketError {
                            id,
                            error: format!("Receive error: {}", e),
                        });
                        return;
                    }
                    None => {
                        // Stream ended
                        let _ = response_tx.send(NetworkResponse::WebSocketClosed { id });
                        return;
                    }
                }
            }
        }
    }
}
