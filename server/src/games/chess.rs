use super::GameError;
use battld_common::games::chess::*;
use battld_common::games::players::PlayerSymbol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChessMoveData {
    pub from: ChessPosition,
    pub to: ChessPosition,
}

pub struct ChessEngine;

impl ChessEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn update(
        &self,
        state: &ChessGameState,
        player: PlayerSymbol,
        chess_move: &ChessMove,
    ) -> Result<ChessGameState, GameError> {
        if player != 1 && player != 2 {
            return Err(GameError::InvalidPlayer);
        }

        if state.is_finished() {
            return Err(GameError::GameNotInProgress);
        }

        let player_color = Player::from_symbol(player).ok_or(GameError::InvalidPlayer)?;

        if state.current_turn != player_color {
            return Err(GameError::WrongTurn);
        }

        match state.is_valid_move(chess_move, player_color) {
            Ok(true) => {},
            Ok(false) => return Err(GameError::IllegalMove("Invalid move".to_string())),
            Err(msg) => return Err(GameError::IllegalMove(msg)),
        }

        let mut new_state = state.clone();
        self.apply_move(&mut new_state, chess_move)?;

        new_state.move_history.push(chess_move.clone());
        new_state.current_turn = player_color.opponent();

        new_state.check_state = if new_state.is_in_check(new_state.current_turn) {
            Some(new_state.current_turn)
        } else {
            None
        };

        if self.is_checkmate(&new_state, new_state.current_turn) {
            new_state.game_over = Some(GameOverReason::Checkmate(player_color));
        } else if self.is_stalemate(&new_state, new_state.current_turn) {
            new_state.game_over = Some(GameOverReason::Stalemate);
        }

        Ok(new_state)
    }

    fn apply_move(&self, state: &mut ChessGameState, chess_move: &ChessMove) -> Result<(), GameError> {
        let piece = state.get_piece(chess_move.from).cloned()
            .ok_or_else(|| GameError::IllegalMove("No piece at source position".to_string()))?;

        *state.get_piece_mut(chess_move.from) = None;
        *state.get_piece_mut(chess_move.to) = Some(piece);

        Ok(())
    }

    fn is_checkmate(&self, state: &ChessGameState, player: Player) -> bool {
        if !state.is_in_check(player) {
            return false;
        }

        !self.has_legal_moves(state, player)
    }

    fn is_stalemate(&self, state: &ChessGameState, player: Player) -> bool {
        if state.is_in_check(player) {
            return false;
        }

        !self.has_legal_moves(state, player)
    }

    fn has_legal_moves(&self, state: &ChessGameState, player: Player) -> bool {
        for from_row in 0..8 {
            for from_col in 0..8 {
                let from = ChessPosition::new(from_row, from_col).unwrap();
                if let Some(piece) = state.get_piece(from) {
                    if piece.player == player {
                        for to_row in 0..8 {
                            for to_col in 0..8 {
                                let to = ChessPosition::new(to_row, to_col).unwrap();
                                let test_move = ChessMove { from, to };
                                if state.is_valid_move(&test_move, player).unwrap_or(false) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

impl Default for ChessEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let _engine = ChessEngine::new();
        let state = ChessGameState::new();
        assert_eq!(state.current_turn, Player::White);
    }

    #[test]
    fn test_valid_pawn_move() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(1, 4).unwrap(),
            to: ChessPosition::new(2, 4).unwrap(),
        };

        let new_state = engine.update(&state, 1, &chess_move).unwrap();
        assert_eq!(new_state.current_turn, Player::Black);
        assert!(new_state.get_piece(ChessPosition::new(2, 4).unwrap()).is_some());
    }

    #[test]
    fn test_invalid_move_empty_square() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(3, 3).unwrap(),
            to: ChessPosition::new(4, 4).unwrap(),
        };

        let result = engine.update(&state, 1, &chess_move);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_turn() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(6, 4).unwrap(),
            to: ChessPosition::new(5, 4).unwrap(),
        };

        let result = engine.update(&state, 2, &chess_move);
        assert!(matches!(result, Err(GameError::WrongTurn)));
    }

    #[test]
    fn test_cannot_capture_own_piece() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(0, 1).unwrap(),
            to: ChessPosition::new(1, 3).unwrap(),
        };

        let result = engine.update(&state, 1, &chess_move);
        assert!(result.is_err());
    }

    #[test]
    fn test_knight_movement() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(0, 1).unwrap(),
            to: ChessPosition::new(2, 2).unwrap(),
        };

        let new_state = engine.update(&state, 1, &chess_move).unwrap();
        assert!(new_state.get_piece(ChessPosition::new(2, 2).unwrap()).is_some());
    }

    #[test]
    fn test_pawn_double_move() {
        let engine = ChessEngine::new();
        let state = ChessGameState::new();

        let chess_move = ChessMove {
            from: ChessPosition::new(1, 4).unwrap(),
            to: ChessPosition::new(3, 4).unwrap(),
        };

        let new_state = engine.update(&state, 1, &chess_move).unwrap();
        assert!(new_state.get_piece(ChessPosition::new(3, 4).unwrap()).is_some());
    }
}
