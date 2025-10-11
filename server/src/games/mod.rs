pub mod tic_tac_toe;
pub mod rock_paper_scissors;
pub mod briscola;

use std::fmt;

/// Errors that can occur during game operations
#[derive(Debug, Clone, PartialEq)]
pub enum GameError {
    /// Move is illegal (e.g., cell already occupied, out of bounds)
    IllegalMove(String),
    /// Game is not in progress (already finished)
    GameNotInProgress,
    /// Wrong player's turn
    WrongTurn,
    /// Invalid player
    InvalidPlayer,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::IllegalMove(msg) => write!(f, "Illegal move: {msg}"),
            GameError::GameNotInProgress => write!(f, "Game is not in progress"),
            GameError::WrongTurn => write!(f, "Not your turn"),
            GameError::InvalidPlayer => write!(f, "Invalid player"),
        }
    }
}

impl std::error::Error for GameError {}
