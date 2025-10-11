use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the type of game being played
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GameType {
    TicTacToe,
    RockPaperScissors,
    Briscola,
}

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameType::TicTacToe => write!(f, "Tic-Tac-Toe"),
            GameType::RockPaperScissors => write!(f, "Rock-Paper-Scissors"),
            GameType::Briscola => write!(f, "Briscola"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub disconnect_timeout_secs: u64,
}

pub fn get_game_config(game_type: &GameType) -> GameConfig {
    match game_type {
        GameType::TicTacToe | GameType::RockPaperScissors | GameType::Briscola => GameConfig {
            disconnect_timeout_secs: 30
        }
    }
}