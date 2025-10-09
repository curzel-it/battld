use serde::{Deserialize, Serialize};

/// Represents the state of a single cell in the tris board
/// 0 = empty, 1 = player1, 2 = player2
pub type CellState = i32;

/// Represents the 3x3 tris game board as a flat array of 9 cells
/// Index mapping: [0,1,2,3,4,5,6,7,8]
/// Visual layout:
/// ```text
/// 0 | 1 | 2
/// ---------
/// 3 | 4 | 5
/// ---------
/// 6 | 7 | 8
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    pub board: [CellState; 9],
    pub current_player: i32,
    pub winner: Option<i32>,
    pub is_finished: bool,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            board: [0; 9],
            current_player: 1,
            winner: None,
            is_finished: false,
        }
    }

    /// Convert row and column (0-indexed) to board index
    pub fn coords_to_index(row: usize, col: usize) -> Option<usize> {
        if row < 3 && col < 3 {
            Some(row * 3 + col)
        } else {
            None
        }
    }

    /// Place a move on the board
    pub fn place_move(&mut self, index: usize, player: i32) -> Result<(), String> {
        if index >= 9 {
            return Err("Invalid cell index".to_string());
        }
        if self.board[index] != 0 {
            return Err("Cell already occupied".to_string());
        }
        if player != 1 && player != 2 {
            return Err("Invalid player number".to_string());
        }
        self.board[index] = player;
        Ok(())
    }

    /// Check if there's a winner. Returns Some(player_num) if there's a winner, None otherwise
    pub fn check_winner(&self) -> Option<i32> {
        // Winning combinations
        let wins = [
            [0, 1, 2], [3, 4, 5], [6, 7, 8], // rows
            [0, 3, 6], [1, 4, 7], [2, 5, 8], // columns
            [0, 4, 8], [2, 4, 6],            // diagonals
        ];

        for win in &wins {
            let [a, b, c] = *win;
            if self.board[a] != 0
                && self.board[a] == self.board[b]
                && self.board[b] == self.board[c] {
                return Some(self.board[a]);
            }
        }
        None
    }

    /// Check if the board is full (draw condition if no winner)
    pub fn is_full(&self) -> bool {
        self.board.iter().all(|&cell| cell != 0)
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "[]".to_string())
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coords_to_index() {
        assert_eq!(GameState::coords_to_index(0, 0), Some(0));
        assert_eq!(GameState::coords_to_index(0, 1), Some(1));
        assert_eq!(GameState::coords_to_index(0, 2), Some(2));
        assert_eq!(GameState::coords_to_index(1, 0), Some(3));
        assert_eq!(GameState::coords_to_index(1, 1), Some(4));
        assert_eq!(GameState::coords_to_index(2, 2), Some(8));
        assert_eq!(GameState::coords_to_index(3, 0), None);
        assert_eq!(GameState::coords_to_index(0, 3), None);
    }

    #[test]
    fn test_place_move() {
        let mut state = GameState::new();
        assert!(state.place_move(0, 1).is_ok());
        assert_eq!(state.board[0], 1);

        // Can't place on occupied cell
        assert!(state.place_move(0, 2).is_err());
    }

    #[test]
    fn test_check_winner_row() {
        let mut state = GameState::new();
        state.board = [1, 1, 1, 0, 0, 0, 0, 0, 0];
        assert_eq!(state.check_winner(), Some(1));
    }

    #[test]
    fn test_check_winner_column() {
        let mut state = GameState::new();
        state.board = [2, 0, 0, 2, 0, 0, 2, 0, 0];
        assert_eq!(state.check_winner(), Some(2));
    }

    #[test]
    fn test_check_winner_diagonal() {
        let mut state = GameState::new();
        state.board = [1, 0, 0, 0, 1, 0, 0, 0, 1];
        assert_eq!(state.check_winner(), Some(1));
    }

    #[test]
    fn test_is_full() {
        let mut state = GameState::new();
        assert!(!state.is_full());

        state.board = [1, 2, 1, 2, 1, 2, 2, 1, 2];
        assert!(state.is_full());
    }
}
