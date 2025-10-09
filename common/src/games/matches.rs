use serde::{Deserialize, Serialize};
use std::fmt;
use games::game_type::GameType;

/// Represents a tris match
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Match {
    pub id: i64,
    pub player1_id: i64,
    pub player2_id: i64,
    pub in_progress: bool,
    pub outcome: Option<MatchOutcome>,
    pub game_type: GameType,
    pub game_state: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MatchOutcome {
    #[serde(rename = "p1_win")]
    Player1Win,
    #[serde(rename = "p2_win")]
    Player2Win,
    #[serde(rename = "draw")]
    Draw,
}

impl fmt::Display for MatchOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchOutcome::Player1Win => write!(f, "p1_win"),
            MatchOutcome::Player2Win => write!(f, "p2_win"),
            MatchOutcome::Draw => write!(f, "draw"),
        }
    }
}

impl MatchOutcome {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "p1_win" => Some(MatchOutcome::Player1Win),
            "p2_win" => Some(MatchOutcome::Player2Win),
            "draw" => Some(MatchOutcome::Draw),
            _ => None,
        }
    }
}