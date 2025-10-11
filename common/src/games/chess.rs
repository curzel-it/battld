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
