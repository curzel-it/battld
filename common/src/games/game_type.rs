use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the type of game being played
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    #[serde(rename = "tris")]
    TicTacToe,
    #[serde(rename = "rps")]
    RockPaperScissors,
}

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameType::TicTacToe => write!(f, "tris"),
            GameType::RockPaperScissors => write!(f, "rps"),
        }
    }
}

impl GameType {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "tris" => Some(GameType::TicTacToe),
            "rps" => Some(GameType::RockPaperScissors),
            _ => None,
        }
    }
}