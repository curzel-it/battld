use battld_common::{ClientMessage, ServerMessage, Match};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, interval};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::fs::OpenOptions;
use std::io::Write as _;

/// WebSocket client for real-time game updates
pub struct WebSocketClient {
    tx: mpsc::UnboundedSender<ClientMessage>,
    server_messages: Arc<RwLock<Vec<ServerMessage>>>,
    current_match: Arc<RwLock<Option<Match>>>,
    connected: Arc<RwLock<bool>>,
    close_tx: Arc<RwLock<Option<mpsc::UnboundedSender<()>>>>,
    #[allow(dead_code)]
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketClient {
    /// Connect to the WebSocket server and authenticate
    pub async fn connect(ws_url: &str, auth_token: String) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Create channel for sending messages to server
        let (tx, mut rx) = mpsc::unbounded_channel::<ClientMessage>();

        // Send authentication as first message
        let auth_msg = ClientMessage::Authenticate { token: auth_token.clone() };
        let auth_json = serde_json::to_string(&auth_msg)?;
        write.send(Message::Text(auth_json)).await?;

        // Shared storage for server messages
        let server_messages = Arc::new(RwLock::new(Vec::new()));
        let server_messages_clone = server_messages.clone();

        // Shared storage for current match state
        let current_match = Arc::new(RwLock::new(None));
        let current_match_clone = current_match.clone();

        // Connection status
        let connected = Arc::new(RwLock::new(true));
        let connected_read = connected.clone();
        let connected_write = connected.clone();

        // Channel for triggering close
        let (close_tx, mut close_rx) = mpsc::unbounded_channel::<()>();
        let close_tx_shared = Arc::new(RwLock::new(Some(close_tx)));

        // Spawn task to send messages to server
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        // Log outgoing message
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                            let _ = writeln!(file, "[SEND] {msg:?}");
                        }

                        if let Ok(json) = serde_json::to_string(&msg) {
                            if write.send(Message::Text(json)).await.is_err() {
                                eprintln!("WebSocket send failed - connection may be lost");
                                *connected_write.write().await = false;
                                break;
                            }
                        }
                    }
                    Some(_) = close_rx.recv() => {
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                            let _ = writeln!(file, "[EVENT] Closing WebSocket connection");
                        }
                        let _ = write.send(Message::Close(None)).await;
                        let _ = write.close().await;
                        *connected_write.write().await = false;
                        break;
                    }
                }
            }
        });

        // Spawn task to receive messages from server
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                            // Log incoming message
                            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                                let _ = writeln!(file, "[RECV] {server_msg:?}");
                            }

                            // Update current match state immediately for game state updates
                            match &server_msg {
                                ServerMessage::MatchFound { match_data } => {
                                    *current_match_clone.write().await = Some(match_data.clone());
                                }
                                ServerMessage::GameStateUpdate { match_data } => {
                                    *current_match_clone.write().await = Some(match_data.clone());
                                }
                                _ => {}
                            }

                            // Always queue ALL messages so they can be printed/processed
                            let mut messages = server_messages_clone.write().await;
                            messages.push(server_msg);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                            let _ = writeln!(file, "[EVENT] WebSocket connection closed by server");
                        }
                        eprintln!("WebSocket connection closed by server");
                        *connected_read.write().await = false;
                        break;
                    }
                    Err(e) => {
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                            let _ = writeln!(file, "[EVENT] WebSocket error: {e}");
                        }
                        eprintln!("WebSocket error: {e}");
                        *connected_read.write().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn keepalive task
        let tx_keepalive = tx.clone();
        let keepalive_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if tx_keepalive.send(ClientMessage::Ping).is_err() {
                    break;
                }
            }
        });

        Ok(WebSocketClient {
            tx,
            server_messages,
            current_match,
            connected,
            close_tx: close_tx_shared,
            keepalive_handle: Some(keepalive_handle),
        })
    }

    /// Send a message to the server
    pub fn send(&self, msg: ClientMessage) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(msg)?;
        Ok(())
    }

    /// Get and clear all pending server messages
    pub async fn get_messages(&self) -> Vec<ServerMessage> {
        let mut messages = self.server_messages.write().await;
        let result = messages.clone();
        messages.clear();
        result
    }

    /// Get the current match state (updated in real-time)
    pub async fn get_current_match(&self) -> Option<Match> {
        self.current_match.read().await.clone()
    }

    /// Check if the WebSocket is currently connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Close the WebSocket connection
    pub async fn close(&self) {
        if let Some(tx) = self.close_tx.write().await.take() {
            let _ = tx.send(());
        }
    }
}
