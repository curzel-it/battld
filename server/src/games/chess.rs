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

        if !self.is_valid_move(state, chess_move, player_color)? {
            return Err(GameError::IllegalMove("Invalid move".to_string()));
        }

        let mut new_state = state.clone();
        self.apply_move(&mut new_state, chess_move)?;

        new_state.move_history.push(chess_move.clone());
        new_state.current_turn = player_color.opponent();

        new_state.check_state = if self.is_in_check(&new_state, new_state.current_turn) {
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

    fn is_valid_move(
        &self,
        state: &ChessGameState,
        chess_move: &ChessMove,
        player: Player,
    ) -> Result<bool, GameError> {
        let piece = state.get_piece(chess_move.from)
            .ok_or_else(|| GameError::IllegalMove("No piece at source position".to_string()))?;

        if piece.player != player {
            return Err(GameError::IllegalMove("Cannot move opponent's piece".to_string()));
        }

        if let Some(target_piece) = state.get_piece(chess_move.to) {
            if target_piece.player == player {
                return Err(GameError::IllegalMove("Cannot capture own piece".to_string()));
            }
        }

        if !self.is_valid_piece_move(state, chess_move, piece)? {
            return Ok(false);
        }

        if self.would_move_cause_check(state, chess_move, player) {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_valid_piece_move(
        &self,
        state: &ChessGameState,
        chess_move: &ChessMove,
        piece: &ChessPieceState,
    ) -> Result<bool, GameError> {
        let from = chess_move.from;
        let to = chess_move.to;

        if from == to {
            return Ok(false);
        }

        let row_diff = (to.row as i8 - from.row as i8).abs();
        let col_diff = (to.col as i8 - from.col as i8).abs();

        match piece.piece {
            ChessPiece::Pawn => self.is_valid_pawn_move(state, chess_move, piece.player),
            ChessPiece::Rook => {
                if row_diff == 0 || col_diff == 0 {
                    self.is_path_clear(state, from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::Knight => {
                Ok((row_diff == 2 && col_diff == 1) || (row_diff == 1 && col_diff == 2))
            }
            ChessPiece::Bishop => {
                if row_diff == col_diff && row_diff > 0 {
                    self.is_path_clear(state, from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::Queen => {
                if row_diff == col_diff || row_diff == 0 || col_diff == 0 {
                    self.is_path_clear(state, from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::King => {
                Ok(row_diff <= 1 && col_diff <= 1)
            }
        }
    }

    fn is_valid_pawn_move(
        &self,
        state: &ChessGameState,
        chess_move: &ChessMove,
        player: Player,
    ) -> Result<bool, GameError> {
        let from = chess_move.from;
        let to = chess_move.to;

        let direction: i8 = match player {
            Player::White => 1,
            Player::Black => -1,
        };

        let row_diff = to.row as i8 - from.row as i8;
        let col_diff = (to.col as i8 - from.col as i8).abs();

        if row_diff == direction && col_diff == 0 {
            return Ok(state.get_piece(to).is_none());
        }

        if row_diff == direction * 2 && col_diff == 0 {
            let start_row = match player {
                Player::White => 1,
                Player::Black => 6,
            };
            if from.row == start_row {
                let middle_pos = ChessPosition::new(
                    (from.row as i8 + direction) as u8,
                    from.col,
                ).unwrap();
                return Ok(state.get_piece(middle_pos).is_none() && state.get_piece(to).is_none());
            }
        }

        if row_diff == direction && col_diff == 1 {
            return Ok(state.get_piece(to).is_some());
        }

        Ok(false)
    }

    fn is_path_clear(&self, state: &ChessGameState, from: ChessPosition, to: ChessPosition) -> Result<bool, GameError> {
        let row_dir = (to.row as i8 - from.row as i8).signum();
        let col_dir = (to.col as i8 - from.col as i8).signum();

        let mut current_row = from.row as i8 + row_dir;
        let mut current_col = from.col as i8 + col_dir;

        while current_row != to.row as i8 || current_col != to.col as i8 {
            let pos = ChessPosition::new(current_row as u8, current_col as u8)
                .ok_or_else(|| GameError::IllegalMove("Invalid position in path".to_string()))?;

            if state.get_piece(pos).is_some() {
                return Ok(false);
            }

            current_row += row_dir;
            current_col += col_dir;
        }

        Ok(true)
    }

    fn would_move_cause_check(&self, state: &ChessGameState, chess_move: &ChessMove, player: Player) -> bool {
        let mut test_state = state.clone();
        if self.apply_move(&mut test_state, chess_move).is_err() {
            return true;
        }
        self.is_in_check(&test_state, player)
    }

    fn is_in_check(&self, state: &ChessGameState, player: Player) -> bool {
        let king_pos = self.find_king(state, player);
        if king_pos.is_none() {
            return false;
        }
        let king_pos = king_pos.unwrap();

        self.is_square_attacked(state, king_pos, player.opponent())
    }

    fn is_square_attacked(&self, state: &ChessGameState, pos: ChessPosition, by_player: Player) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                let from = ChessPosition::new(row, col).unwrap();
                if let Some(piece) = state.get_piece(from) {
                    if piece.player == by_player {
                        let test_move = ChessMove { from, to: pos };
                        if let Ok(true) = self.is_valid_piece_move(state, &test_move, piece) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn find_king(&self, state: &ChessGameState, player: Player) -> Option<ChessPosition> {
        for row in 0..8 {
            for col in 0..8 {
                let pos = ChessPosition::new(row, col).unwrap();
                if let Some(piece) = state.get_piece(pos) {
                    if piece.player == player && piece.piece == ChessPiece::King {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }

    fn is_checkmate(&self, state: &ChessGameState, player: Player) -> bool {
        if !self.is_in_check(state, player) {
            return false;
        }

        !self.has_legal_moves(state, player)
    }

    fn is_stalemate(&self, state: &ChessGameState, player: Player) -> bool {
        if self.is_in_check(state, player) {
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
                                if self.is_valid_move(state, &test_move, player).unwrap_or(false) {
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
