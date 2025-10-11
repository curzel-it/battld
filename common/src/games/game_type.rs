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