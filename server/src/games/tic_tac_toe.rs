use super::GameError;
use battld_common::games::players::PlayerSymbol;
use serde::{Deserialize, Serialize};

/// Represents a move in tic-tac-toe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicTacToeMove {
    pub row: usize,
    pub col: usize,
}

impl TicTacToeMove {
    /// Convert row and column to board index
    fn to_index(&self) -> Option<usize> {
        if self.row < 3 && self.col < 3 {
            Some(self.row * 3 + self.col)
        } else {
            None
        }
    }
}

/// Represents the complete state of a tic-tac-toe game
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TicTacToeGameState {
    /// The 3x3 board as a flat array of 9 cells (0 = empty, 1 = player1, 2 = player2)
    pub board: [i32; 9],
    /// Which player's turn it is (1 or 2)
    pub current_player: PlayerSymbol,
    /// Winner if game is finished (None if no winner or game in progress)
    pub winner: Option<PlayerSymbol>,
    /// Whether the game has finished
    pub is_finished: bool,
}

impl TicTacToeGameState {
    /// Create a new game with empty board, player 1 to start
    pub fn new() -> Self {
        Self {
            board: [0; 9],
            current_player: 1,
            winner: None,
            is_finished: false,
        }
    }

    /// Redact game state for a specific player
    /// TicTacToe doesn't need redaction (all info is public), so returns clone
    pub fn redact_for_player(&self, _player: PlayerSymbol) -> Self {
        self.clone()
    }

    /// Check if the board is full (draw condition if no winner)
    fn is_full(&self) -> bool {
        self.board.iter().all(|&cell| cell != 0)
    }

    /// Check if there's a winner. Returns Some(player_num) if there's a winner, None otherwise
    fn check_winner(&self) -> Option<PlayerSymbol> {
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
                && self.board[b] == self.board[c]
            {
                return Some(self.board[a]);
            }
        }
        None
    }
}

impl Default for TicTacToeGameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateless tic-tac-toe game engine
/// This engine doesn't hold any state; it purely transforms game states
pub struct TicTacToeEngine;

impl TicTacToeEngine {
    /// Create a new engine instance
    pub fn new() -> Self {
        Self
    }

    /// Update the game state with a new move
    ///
    /// This is a pure function that takes the current state and returns a new state.
    /// The old state is not modified.
    ///
    /// # Arguments
    /// * `state` - The current game state
    /// * `player` - The player making the move (1 or 2)
    /// * `game_move` - The move to make
    ///
    /// # Returns
    /// * `Ok(new_state)` - The new game state after applying the move
    /// * `Err(GameError)` - If the move is invalid
    pub fn update(
        &self,
        state: &TicTacToeGameState,
        player: PlayerSymbol,
        game_move: &TicTacToeMove,
    ) -> Result<TicTacToeGameState, GameError> {
        // Validate player number first
        if player != 1 && player != 2 {
            return Err(GameError::InvalidPlayer);
        }

        // Check if game is still in progress
        if state.is_finished {
            return Err(GameError::GameNotInProgress);
        }

        // Validate it's the correct player's turn
        if state.current_player != player {
            return Err(GameError::WrongTurn);
        }

        // Convert move to index
        let index = game_move
            .to_index()
            .ok_or_else(|| GameError::IllegalMove("Invalid coordinates".to_string()))?;

        // Check if cell is empty
        if state.board[index] != 0 {
            return Err(GameError::IllegalMove("Cell already occupied".to_string()));
        }

        // Create new state with the move applied
        let mut new_state = state.clone();
        new_state.board[index] = player;

        // Check for winner
        if let Some(winner) = new_state.check_winner() {
            new_state.winner = Some(winner);
            new_state.is_finished = true;
        } else if new_state.is_full() {
            // Draw - no winner but board is full
            new_state.winner = None;
            new_state.is_finished = true;
        } else {
            // Game continues - switch to next player
            new_state.current_player = if player == 1 { 2 } else { 1 };
        }

        Ok(new_state)
    }
}

impl Default for TicTacToeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_game_state() {
        let state = TicTacToeGameState::new();
        assert_eq!(state.board, [0; 9]);
        assert_eq!(state.current_player, 1);
        assert_eq!(state.winner, None);
        assert!(!state.is_finished);
    }

    #[test]
    fn test_valid_move() {
        let engine = TicTacToeEngine::new();
        let state = TicTacToeGameState::new();
        let game_move = TicTacToeMove { row: 0, col: 0 };

        let new_state = engine.update(&state, 1, &game_move).unwrap();

        assert_eq!(new_state.board[0], 1);
        assert_eq!(new_state.current_player, 2);
        assert!(!new_state.is_finished);
        assert_eq!(new_state.winner, None);
    }

    #[test]
    fn test_illegal_move_occupied_cell() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();
        state.board[0] = 1; // Cell already occupied

        let game_move = TicTacToeMove { row: 0, col: 0 };
        let result = engine.update(&state, 1, &game_move);

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }

    #[test]
    fn test_illegal_move_out_of_bounds() {
        let engine = TicTacToeEngine::new();
        let state = TicTacToeGameState::new();

        let game_move = TicTacToeMove { row: 3, col: 0 };
        let result = engine.update(&state, 1, &game_move);

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }

    #[test]
    fn test_wrong_turn() {
        let engine = TicTacToeEngine::new();
        let state = TicTacToeGameState::new(); // Player 1's turn

        let game_move = TicTacToeMove { row: 0, col: 0 };
        let result = engine.update(&state, 2, &game_move); // Player 2 tries to move

        assert!(matches!(result, Err(GameError::WrongTurn)));
    }

    #[test]
    fn test_win_condition_row() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();

        // Set up a board where player 1 is about to win with top row
        // X X _
        // O O _
        // _ _ _
        state.board = [1, 1, 0, 2, 2, 0, 0, 0, 0];
        state.current_player = 1;

        let game_move = TicTacToeMove { row: 0, col: 2 };
        let new_state = engine.update(&state, 1, &game_move).unwrap();

        assert_eq!(new_state.winner, Some(1));
        assert!(new_state.is_finished);
    }

    #[test]
    fn test_win_condition_column() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();

        // Set up a board where player 2 is about to win with first column
        // O X X
        // O X _
        // _ _ _
        state.board = [2, 1, 1, 2, 1, 0, 0, 0, 0];
        state.current_player = 2;

        let game_move = TicTacToeMove { row: 2, col: 0 };
        let new_state = engine.update(&state, 2, &game_move).unwrap();

        assert_eq!(new_state.winner, Some(2));
        assert!(new_state.is_finished);
    }

    #[test]
    fn test_win_condition_diagonal() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();

        // Set up a board where player 1 is about to win with diagonal
        // X O _
        // O X _
        // _ _ _
        state.board = [1, 2, 0, 2, 1, 0, 0, 0, 0];
        state.current_player = 1;

        let game_move = TicTacToeMove { row: 2, col: 2 };
        let new_state = engine.update(&state, 1, &game_move).unwrap();

        assert_eq!(new_state.winner, Some(1));
        assert!(new_state.is_finished);
    }

    #[test]
    fn test_draw_condition() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();

        // Set up a board that will be a draw after one more move
        // X O X
        // X O O
        // O X _
        state.board = [1, 2, 1, 1, 2, 2, 2, 1, 0];
        state.current_player = 1;

        let game_move = TicTacToeMove { row: 2, col: 2 };
        let new_state = engine.update(&state, 1, &game_move).unwrap();

        assert_eq!(new_state.winner, None);
        assert!(new_state.is_finished);
        assert!(new_state.is_full());
    }

    #[test]
    fn test_game_already_finished() {
        let engine = TicTacToeEngine::new();
        let mut state = TicTacToeGameState::new();
        state.is_finished = true;

        let game_move = TicTacToeMove { row: 0, col: 0 };
        let result = engine.update(&state, 1, &game_move);

        assert!(matches!(result, Err(GameError::GameNotInProgress)));
    }

    #[test]
    fn test_state_immutability() {
        let engine = TicTacToeEngine::new();
        let state = TicTacToeGameState::new();
        let game_move = TicTacToeMove { row: 1, col: 1 };

        let _new_state = engine.update(&state, 1, &game_move).unwrap();

        // Original state should be unchanged
        assert_eq!(state.board, [0; 9]);
        assert_eq!(state.current_player, 1);
        assert!(!state.is_finished);
    }

    #[test]
    fn test_invalid_player() {
        let engine = TicTacToeEngine::new();
        let state = TicTacToeGameState::new();
        let game_move = TicTacToeMove { row: 0, col: 0 };

        let result = engine.update(&state, 3, &game_move);
        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }
}
