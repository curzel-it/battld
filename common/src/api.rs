use serde::{Deserialize, Serialize};
use crate::Match;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatePlayerRequest {
    pub public_key_hint: String,
    pub public_key: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrisMoveRequest {
    pub row: usize,
    pub col: usize,
}

// WebSocket message types

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MatchEndReason {
    #[serde(rename = "ended")]
    Ended,
    #[serde(rename = "disconnection")]
    Disconnection,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "authenticate")]
    Authenticate { token: String },
    #[serde(rename = "join_matchmaking")]
    JoinMatchmaking,
    #[serde(rename = "resume_match")]
    ResumeMatch,
    #[serde(rename = "make_move")]
    MakeMove { row: usize, col: usize },
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess { player_id: i64 },

    #[serde(rename = "auth_failed")]
    AuthFailed { reason: String },

    #[serde(rename = "waiting_for_opponent")]
    WaitingForOpponent,

    #[serde(rename = "match_found")]
    MatchFound { match_data: Match },

    #[serde(rename = "game_state_update")]
    GameStateUpdate { match_data: Match },

    #[serde(rename = "player_disconnected")]
    PlayerDisconnected { player_id: i64 },

    #[serde(rename = "resumable_match")]
    ResumableMatch { match_data: Match },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "match_ended")]
    MatchEnded { reason: MatchEndReason },

    #[serde(rename = "pong")]
    Pong,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerStats {
    pub player_id: i64,
    pub won: i64,
    pub lost: i64,
    pub draw: i64,
    pub dropped: i64,
    pub total: i64,
    pub score: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeaderboardEntry {
    pub player_id: i64,
    pub player_name: String,
    pub rank: i64,
    pub score: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeaderboardResponse {
    pub entries: Vec<LeaderboardEntry>,
    pub total_count: i64,
}
