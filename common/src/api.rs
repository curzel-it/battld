use serde::{Deserialize, Serialize};
use crate::games::{game_type::GameType, matches::{Match, MatchEndReason}};
use crate::player::Player;

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
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "authenticate")]
    Authenticate { token: String },
    #[serde(rename = "join_matchmaking")]
    JoinMatchmaking { game_type: GameType },
    #[serde(rename = "resume_match")]
    ResumeMatch,
    #[serde(rename = "make_move")]
    MakeMove { move_data: serde_json::Value },
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

// New auth flow types

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChallengeRequest {
    pub player_id: i64,
    pub public_key_hint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChallengeResponse {
    pub nonce: String,
    pub expires_in: u64, // seconds
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyRequest {
    pub player_id: i64,
    pub nonce: String,
    pub signature: String, // base64 encoded
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthResponse {
    pub session_token: String,
    pub expires_at: String, // ISO 8601 timestamp
    pub player: Player,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogoutRequest {
    pub session_token: String,
}
