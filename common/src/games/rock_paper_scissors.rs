use serde::{Deserialize, Serialize};

/// Type alias for player symbols (1 or 2)
pub type PlayerSymbol = i32;

/// Represents a move in Rock-Paper-Scissors
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RPSMove {
    Rock,
    Paper,
    Scissors,
    Redacted,
}

impl RPSMove {
    /// Determine winner: returns Some(winning_move) or None for draw
    pub fn beats(&self, other: &RPSMove) -> Option<RPSMove> {
        match (self, other) {
            (RPSMove::Rock, RPSMove::Scissors) => Some(*self),
            (RPSMove::Paper, RPSMove::Rock) => Some(*self),
            (RPSMove::Scissors, RPSMove::Paper) => Some(*self),
            (RPSMove::Scissors, RPSMove::Rock) => Some(*other),
            (RPSMove::Rock, RPSMove::Paper) => Some(*other),
            (RPSMove::Paper, RPSMove::Scissors) => Some(*other),
            _ => None, // Draw
        }
    }
}

/// Represents the complete state of a Rock-Paper-Scissors game
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RPSGameState {
    /// List of rounds: each round is (player1_move, player2_move)
    /// None means the player hasn't submitted their move yet
    pub rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>,
}

impl Default for RPSGameState {
    fn default() -> Self {
        Self::new()
    }
}

impl RPSGameState {
    /// Create a new RPS game with initial round
    pub fn new() -> Self {
        Self {
            rounds: vec![(None, None)],
        }
    }

    /// Get current round number (1-indexed for display)
    #[allow(dead_code)]
    pub fn current_round(&self) -> usize {
        self.rounds.len()
    }

    /// Get score as (player1_wins, player2_wins)
    pub fn get_score(&self) -> (u8, u8) {
        let mut p1_wins = 0;
        let mut p2_wins = 0;

        for round in &self.rounds {
            if let (Some(p1_move), Some(p2_move)) = round {
                if let Some(winner) = p1_move.beats(p2_move) {
                    if winner == *p1_move {
                        p1_wins += 1;
                    } else {
                        p2_wins += 1;
                    }
                }
                // If None (draw), neither player gets a point
            }
        }

        (p1_wins, p2_wins)
    }

    /// Check if the game is finished (either player has 2 wins)
    pub fn is_finished(&self) -> bool {
        let (p1_wins, p2_wins) = self.get_score();
        p1_wins >= 2 || p2_wins >= 2
    }

    /// Get the winner (if game is finished)
    pub fn get_winner(&self) -> Option<PlayerSymbol> {
        if !self.is_finished() {
            return None;
        }

        let (p1_wins, p2_wins) = self.get_score();
        if p1_wins >= 2 {
            Some(1)
        } else if p2_wins >= 2 {
            Some(2)
        } else {
            None
        }
    }

    /// Redact opponent's moves for a specific player
    /// Player 1 sees their own moves but player 2's moves are redacted (and vice versa)
    pub fn redact_for_player(&self, player: PlayerSymbol) -> Self {
        let redacted_rounds = self.rounds.iter().map(|(p1_move, p2_move)| {
            match player {
                1 => {
                    // Player 1 sees their own moves, but player 2's moves are redacted if incomplete
                    let redacted_p2 = if p1_move.is_some() && p2_move.is_some() {
                        *p2_move // Both moves in, show actual move
                    } else if p2_move.is_some() {
                        Some(RPSMove::Redacted) // Only p2 moved, hide it
                    } else {
                        None // p2 hasn't moved yet
                    };
                    (*p1_move, redacted_p2)
                }
                2 => {
                    // Player 2 sees their own moves, but player 1's moves are redacted if incomplete
                    let redacted_p1 = if p1_move.is_some() && p2_move.is_some() {
                        *p1_move // Both moves in, show actual move
                    } else if p1_move.is_some() {
                        Some(RPSMove::Redacted) // Only p1 moved, hide it
                    } else {
                        None // p1 hasn't moved yet
                    };
                    (redacted_p1, *p2_move)
                }
                _ => (*p1_move, *p2_move), // Invalid player, return as-is
            }
        }).collect();

        Self {
            rounds: redacted_rounds,
        }
    }

    /// Compute the winner of a specific round
    #[allow(dead_code)]
    pub fn compute_round_winner(p1_move: RPSMove, p2_move: RPSMove) -> Option<PlayerSymbol> {
        match p1_move.beats(&p2_move) {
            Some(winner) => {
                if winner == p1_move {
                    Some(1)
                } else {
                    Some(2)
                }
            }
            None => None, // Draw
        }
    }
}
