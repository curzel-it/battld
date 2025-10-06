use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio::task::AbortHandle;
use tokio::time::{Duration, sleep};

use battld_common::{ClientMessage, ServerMessage};
use crate::{database::Database, AppState, game_logic};
use crate::game_logic::OutgoingMessage;

/// Connection info including sender and abort handle
struct ConnectionInfo {
    tx: mpsc::UnboundedSender<ServerMessage>,
    abort_handle: AbortHandle,
}

/// Tracks a player's disconnection from a match with a timer
struct DisconnectInfo {
    match_id: i64,
    timer_handle: AbortHandle,
}

/// Connection registry to track active WebSocket connections per player
pub struct ConnectionRegistry {
    connections: RwLock<HashMap<i64, ConnectionInfo>>,
    disconnects: RwLock<HashMap<i64, DisconnectInfo>>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            disconnects: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new connection for a player
    pub async fn register(&self, player_id: i64, tx: mpsc::UnboundedSender<ServerMessage>, abort_handle: AbortHandle) {
        let mut connections = self.connections.write().await;
        connections.insert(player_id, ConnectionInfo { tx, abort_handle });
        println!("Registered WebSocket connection for player {player_id}");
    }

    /// Unregister a connection and force-close the WebSocket
    pub async fn unregister(&self, player_id: i64) {
        let mut connections = self.connections.write().await;
        if let Some(info) = connections.remove(&player_id) {
            // Abort the send task to force-close the WebSocket
            info.abort_handle.abort();
            println!("Unregistered WebSocket connection for player {player_id}");
        }
    }

    /// Send a message to a specific player
    pub async fn send_to_player(&self, player_id: i64, message: ServerMessage) -> Result<(), String> {
        let connections = self.connections.read().await;
        if let Some(info) = connections.get(&player_id) {
            info.tx.send(message).map_err(|e| format!("Failed to send message: {e}"))
        } else {
            Err(format!("Player {player_id} not connected"))
        }
    }

    /// Send a message to multiple players
    pub async fn send_to_players(&self, player_ids: &[i64], message: ServerMessage) {
        for player_id in player_ids {
            let _ = self.send_to_player(*player_id, message.clone()).await;
        }
    }

    /// Send multiple messages (helper for game logic integration)
    pub async fn send_messages(&self, messages: Vec<OutgoingMessage>) {
        for msg in messages {
            let _ = self.send_to_player(msg.player_id, msg.message).await;
        }
    }

    /// Start a disconnect timer for a player in a match
    pub async fn start_disconnect_timer(
        &self,
        player_id: i64,
        match_id: i64,
        db: Arc<Database>,
        registry: SharedRegistry,
    ) {
        // Cancel any existing timer for this player
        self.cancel_disconnect_timer(player_id).await;

        let timer_task = tokio::spawn(async move {
            sleep(Duration::from_secs(10)).await;
            println!("Disconnect timer expired for player {player_id} in match {match_id}");
            handle_disconnect_timeout(player_id, match_id, &db, &registry).await;
        });

        let mut disconnects = self.disconnects.write().await;
        disconnects.insert(player_id, DisconnectInfo {
            match_id,
            timer_handle: timer_task.abort_handle(),
        });
        println!("Started 10s disconnect timer for player {player_id} in match {match_id}");
    }

    /// Cancel a disconnect timer for a player (they reconnected)
    pub async fn cancel_disconnect_timer(&self, player_id: i64) {
        let mut disconnects = self.disconnects.write().await;
        if let Some(info) = disconnects.remove(&player_id) {
            info.timer_handle.abort();
            println!("Cancelled disconnect timer for player {player_id}");
        }
    }

    /// Check if a player has a resumable match
    pub async fn get_resumable_match(&self, player_id: i64) -> Option<i64> {
        let disconnects = self.disconnects.read().await;
        disconnects.get(&player_id).map(|info| info.match_id)
    }
}

pub type SharedRegistry = Arc<ConnectionRegistry>;

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.db, state.registry))
}

/// Handle a single WebSocket connection
async fn handle_socket(socket: WebSocket, db: Arc<Database>, registry: SharedRegistry) {
    let (mut sender, mut receiver) = socket.split();

    // Channel to send messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            println!("[WS SEND] {msg:?}");
            if let Ok(_json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(_json)).await.is_err() {
                    break;
                }
            }
        }
        // Explicitly close the websocket when channel closes
        // This sends a Close frame to the client
        println!("[WS EVENT] Closing WebSocket connection");
        let _ = sender.close().await;
    });

    // Handle incoming messages
    let mut player_id: Option<i64> = None;

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    println!("[WS RECV] {client_msg:?}");
                    match client_msg {
                        ClientMessage::Authenticate { token } => {
                            // Authenticate the connection
                            match authenticate_token(&db, &token).await {
                                Ok(pid) => {
                                    player_id = Some(pid);
                                    registry.register(pid, tx.clone(), send_task.abort_handle()).await;

                                    let response = ServerMessage::AuthSuccess { player_id: pid };
                                    let _ = tx.send(response);
                                    println!("Player {pid} authenticated via WebSocket");

                                    // Check if player has a resumable match
                                    if let Some(match_id) = registry.get_resumable_match(pid).await {
                                        if let Some(match_record) = db.get_match_by_id(match_id).await {
                                            if let Some(match_info) = match_record.to_match() {
                                                println!("Player {pid} has resumable match {match_id}");
                                                let _ = tx.send(ServerMessage::ResumableMatch {
                                                    match_data: match_info,
                                                });
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let response = ServerMessage::AuthFailed { reason: e };
                                    let _ = tx.send(response);
                                    break; // Close connection on auth failure
                                }
                            }
                        }
                        ClientMessage::Ping => {
                            let _ = tx.send(ServerMessage::Pong);
                        }
                        ClientMessage::JoinMatchmaking => {
                            if let Some(pid) = player_id {
                                // For now, default to TicTacToe (Phase 3 will add menu selection)
                                handle_join_matchmaking(pid, battld_common::GameType::TicTacToe, &db, &registry).await;
                            } else {
                                let _ = tx.send(ServerMessage::Error {
                                    message: "Not authenticated".to_string(),
                                });
                            }
                        }
                        ClientMessage::ResumeMatch => {
                            if let Some(pid) = player_id {
                                handle_resume_match(pid, &db, &registry).await;
                            } else {
                                let _ = tx.send(ServerMessage::Error {
                                    message: "Not authenticated".to_string(),
                                });
                            }
                        }
                        ClientMessage::MakeMove { move_data } => {
                            if let Some(pid) = player_id {
                                handle_make_move(pid, move_data, &db, &registry).await;
                            } else {
                                let _ = tx.send(ServerMessage::Error {
                                    message: "Not authenticated".to_string(),
                                });
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            _ => {}
        }
    }

    // Cleanup on disconnect
    if let Some(pid) = player_id {
        handle_disconnect(pid, &db, &registry).await;
        registry.unregister(pid).await;
    }

    send_task.abort();
}

/// Authenticate a token and return player_id
async fn authenticate_token(db: &Database, token: &str) -> Result<i64, String> {
    // Token format: "player_id:signature"
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid token format".to_string());
    }

    let player_id: i64 = parts[0]
        .parse()
        .map_err(|_| "Invalid player ID".to_string())?;

    let player = db
        .get_player_by_id(player_id)
        .await
        .ok_or_else(|| "Player not found".to_string())?;

    // Verify signature (reuse existing auth logic)
    crate::auth::verify_signature(&player, parts[1])
        .map_err(|_| "Invalid signature".to_string())?;

    Ok(player_id)
}

/// Handle resume match request
async fn handle_resume_match(player_id: i64, db: &Arc<Database>, registry: &SharedRegistry) {
    let resumable_match_id = registry.get_resumable_match(player_id).await;

    // Cancel the disconnect timer if exists
    if resumable_match_id.is_some() {
        registry.cancel_disconnect_timer(player_id).await;
    }

    let messages = game_logic::handle_resume_match_logic(player_id, resumable_match_id, db).await;
    registry.send_messages(messages).await;
}

/// Handle matchmaking request
async fn handle_join_matchmaking(player_id: i64, game_type: battld_common::GameType, db: &Arc<Database>, registry: &SharedRegistry) {
    let messages = game_logic::handle_join_matchmaking_logic(player_id, game_type, db).await;
    registry.send_messages(messages).await;
}

/// Handle a move request
async fn handle_make_move(
    player_id: i64,
    move_data: serde_json::Value,
    db: &Arc<Database>,
    registry: &SharedRegistry,
) {
    let messages = game_logic::handle_make_move_logic(player_id, move_data, db).await;
    registry.send_messages(messages).await;
}

/// Handle disconnect - start grace period timer instead of immediately ending match
async fn handle_disconnect(
    player_id: i64,
    db: &Arc<Database>,
    registry: &SharedRegistry,
) {
    let (messages, match_id_opt) = game_logic::handle_disconnect_logic(player_id, db).await;
    registry.send_messages(messages).await;

    // Start disconnect timer if player was in an active match
    if let Some(match_id) = match_id_opt {
        registry.start_disconnect_timer(player_id, match_id, db.clone(), registry.clone()).await;
    }
}

/// Handle disconnect timeout - called when 10s grace period expires
async fn handle_disconnect_timeout(
    player_id: i64,
    match_id: i64,
    db: &Arc<Database>,
    registry: &SharedRegistry,
) {
    // Remove player from disconnects map (timer expired)
    {
        let mut disconnects = registry.disconnects.write().await;
        disconnects.remove(&player_id);
        println!("Removed player {player_id} from disconnects map (timer expired)");
    }

    let messages = game_logic::handle_disconnect_timeout_logic(player_id, match_id, db).await;
    registry.send_messages(messages).await;
}