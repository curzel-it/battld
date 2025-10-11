use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChessPiece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub fn opponent(&self) -> Player {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }

    pub fn to_symbol(&self) -> i32 {
        match self {
            Player::White => 1,
            Player::Black => 2,
        }
    }

    pub fn from_symbol(symbol: i32) -> Option<Player> {
        match symbol {
            1 => Some(Player::White),
            2 => Some(Player::Black),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChessPosition {
    pub row: u8,
    pub col: u8,
}

impl ChessPosition {
    pub fn new(row: u8, col: u8) -> Option<Self> {
        if row < 8 && col < 8 {
            Some(Self { row, col })
        } else {
            None
        }
    }

    pub fn from_algebraic(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let col = bytes[0].to_ascii_lowercase();
        let row = bytes[1];

        if !col.is_ascii_lowercase() || !(b'a'..=b'h').contains(&col) {
            return None;
        }
        if !row.is_ascii_digit() || !(b'1'..=b'8').contains(&row) {
            return None;
        }

        Some(Self {
            row: row - b'1',
            col: col - b'a',
        })
    }

    pub fn to_algebraic(&self) -> String {
        let col = (b'a' + self.col) as char;
        let row = (b'1' + self.row) as char;
        format!("{col}{row}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChessPieceState {
    pub piece: ChessPiece,
    pub player: Player,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChessMove {
    pub from: ChessPosition,
    pub to: ChessPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameOverReason {
    Checkmate(Player),
    Stalemate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChessGameState {
    pub board: [[Option<ChessPieceState>; 8]; 8],
    pub current_turn: Player,
    pub check_state: Option<Player>,
    pub game_over: Option<GameOverReason>,
    pub move_history: Vec<ChessMove>,
}

impl ChessGameState {
    pub fn new() -> Self {
        let mut board = [[None; 8]; 8];

        let back_row = [
            ChessPiece::Rook,
            ChessPiece::Knight,
            ChessPiece::Bishop,
            ChessPiece::Queen,
            ChessPiece::King,
            ChessPiece::Bishop,
            ChessPiece::Knight,
            ChessPiece::Rook,
        ];

        for (col, &piece) in back_row.iter().enumerate() {
            board[0][col] = Some(ChessPieceState {
                piece,
                player: Player::White,
            });
            board[1][col] = Some(ChessPieceState {
                piece: ChessPiece::Pawn,
                player: Player::White,
            });
            board[6][col] = Some(ChessPieceState {
                piece: ChessPiece::Pawn,
                player: Player::Black,
            });
            board[7][col] = Some(ChessPieceState {
                piece,
                player: Player::Black,
            });
        }

        Self {
            board,
            current_turn: Player::White,
            check_state: None,
            game_over: None,
            move_history: Vec::new(),
        }
    }

    pub fn redact_for_player(&self, _player_symbol: i32) -> Self {
        self.clone()
    }

    pub fn get_piece(&self, pos: ChessPosition) -> Option<&ChessPieceState> {
        self.board[pos.row as usize][pos.col as usize].as_ref()
    }

    pub fn get_piece_mut(&mut self, pos: ChessPosition) -> &mut Option<ChessPieceState> {
        &mut self.board[pos.row as usize][pos.col as usize]
    }

    pub fn is_finished(&self) -> bool {
        self.game_over.is_some()
    }

    pub fn get_winner(&self) -> Option<i32> {
        match &self.game_over {
            Some(GameOverReason::Checkmate(player)) => Some(player.to_symbol()),
            Some(GameOverReason::Stalemate) => None,
            None => None,
        }
    }

    pub fn is_valid_move(&self, chess_move: &ChessMove, player: Player) -> Result<bool, String> {
        let piece = self.get_piece(chess_move.from)
            .ok_or_else(|| "No piece at source position".to_string())?;

        if piece.player != player {
            return Err("Cannot move opponent's piece".to_string());
        }

        if let Some(target_piece) = self.get_piece(chess_move.to) {
            if target_piece.player == player {
                return Err("Cannot capture own piece".to_string());
            }
        }

        if !self.is_valid_piece_move(chess_move, piece)? {
            return Ok(false);
        }

        if self.would_move_cause_check(chess_move, player) {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_valid_piece_move(&self, chess_move: &ChessMove, piece: &ChessPieceState) -> Result<bool, String> {
        let from = chess_move.from;
        let to = chess_move.to;

        if from == to {
            return Ok(false);
        }

        let row_diff = (to.row as i8 - from.row as i8).abs();
        let col_diff = (to.col as i8 - from.col as i8).abs();

        match piece.piece {
            ChessPiece::Pawn => self.is_valid_pawn_move(chess_move, piece.player),
            ChessPiece::Rook => {
                if row_diff == 0 || col_diff == 0 {
                    self.is_path_clear(from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::Knight => {
                Ok((row_diff == 2 && col_diff == 1) || (row_diff == 1 && col_diff == 2))
            }
            ChessPiece::Bishop => {
                if row_diff == col_diff && row_diff > 0 {
                    self.is_path_clear(from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::Queen => {
                if row_diff == col_diff || row_diff == 0 || col_diff == 0 {
                    self.is_path_clear(from, to)
                } else {
                    Ok(false)
                }
            }
            ChessPiece::King => {
                Ok(row_diff <= 1 && col_diff <= 1)
            }
        }
    }

    fn is_valid_pawn_move(&self, chess_move: &ChessMove, player: Player) -> Result<bool, String> {
        let from = chess_move.from;
        let to = chess_move.to;

        let direction: i8 = match player {
            Player::White => 1,
            Player::Black => -1,
        };

        let row_diff = to.row as i8 - from.row as i8;
        let col_diff = (to.col as i8 - from.col as i8).abs();

        if row_diff == direction && col_diff == 0 {
            return Ok(self.get_piece(to).is_none());
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
                return Ok(self.get_piece(middle_pos).is_none() && self.get_piece(to).is_none());
            }
        }

        if row_diff == direction && col_diff == 1 {
            return Ok(self.get_piece(to).is_some());
        }

        Ok(false)
    }

    fn is_path_clear(&self, from: ChessPosition, to: ChessPosition) -> Result<bool, String> {
        let row_dir = (to.row as i8 - from.row as i8).signum();
        let col_dir = (to.col as i8 - from.col as i8).signum();

        let mut current_row = from.row as i8 + row_dir;
        let mut current_col = from.col as i8 + col_dir;

        while current_row != to.row as i8 || current_col != to.col as i8 {
            let pos = ChessPosition::new(current_row as u8, current_col as u8)
                .ok_or_else(|| "Invalid position in path".to_string())?;

            if self.get_piece(pos).is_some() {
                return Ok(false);
            }

            current_row += row_dir;
            current_col += col_dir;
        }

        Ok(true)
    }

    fn would_move_cause_check(&self, chess_move: &ChessMove, player: Player) -> bool {
        let mut test_state = self.clone();
        let piece = test_state.get_piece(chess_move.from).cloned();
        if piece.is_none() {
            return true;
        }

        *test_state.get_piece_mut(chess_move.from) = None;
        *test_state.get_piece_mut(chess_move.to) = piece;

        test_state.is_in_check(player)
    }

    pub fn is_in_check(&self, player: Player) -> bool {
        let king_pos = self.find_king(player);
        if king_pos.is_none() {
            return false;
        }
        let king_pos = king_pos.unwrap();

        self.is_square_attacked(king_pos, player.opponent())
    }

    fn is_square_attacked(&self, pos: ChessPosition, by_player: Player) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                let from = ChessPosition::new(row, col).unwrap();
                if let Some(piece) = self.get_piece(from) {
                    if piece.player == by_player {
                        let test_move = ChessMove { from, to: pos };
                        if let Ok(true) = self.is_valid_piece_move(&test_move, piece) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn find_king(&self, player: Player) -> Option<ChessPosition> {
        for row in 0..8 {
            for col in 0..8 {
                let pos = ChessPosition::new(row, col).unwrap();
                if let Some(piece) = self.get_piece(pos) {
                    if piece.player == player && piece.piece == ChessPiece::King {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }
}

impl Default for ChessGameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_game() {
        let game = ChessGameState::new();
        assert_eq!(game.current_turn, Player::White);
        assert!(game.check_state.is_none());
        assert!(game.game_over.is_none());
        assert_eq!(game.move_history.len(), 0);
    }

    #[test]
    fn test_initial_board_setup() {
        let game = ChessGameState::new();

        assert_eq!(
            game.board[0][0],
            Some(ChessPieceState {
                piece: ChessPiece::Rook,
                player: Player::White
            })
        );
        assert_eq!(
            game.board[0][4],
            Some(ChessPieceState {
                piece: ChessPiece::King,
                player: Player::White
            })
        );
        assert_eq!(
            game.board[7][4],
            Some(ChessPieceState {
                piece: ChessPiece::King,
                player: Player::Black
            })
        );
    }

    #[test]
    fn test_position_algebraic() {
        let pos = ChessPosition::from_algebraic("e4").unwrap();
        assert_eq!(pos.row, 3);
        assert_eq!(pos.col, 4);
        assert_eq!(pos.to_algebraic(), "e4");
    }

    #[test]
    fn test_player_opponent() {
        assert_eq!(Player::White.opponent(), Player::Black);
        assert_eq!(Player::Black.opponent(), Player::White);
    }
}
