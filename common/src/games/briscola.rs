use serde::{Deserialize, Serialize};

use crate::games::players::PlayerSymbol;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Suit {
    Bastoni,
    Coppe,
    Denari,
    Spade,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Rank {
    Two,    //  0 Points
    Four,   //  0 Points
    Five,   //  0 Points
    Six,    //  0 Points
    Seven,  //  0 Points
    Jack,   //  2 Points
    Knight, //  3 Points
    King,   //  4 Points
    Three,  // 10 Points
    Ace,    // 11 Points
}

/// A single card
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

/// Card or redacted (for hiding opponent's cards)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CardView {
    Visible(Card),
    Redacted,
}

/// A move in Briscola
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BriscolaMove {
    PlayCard { card_index: usize }, // Index in player's hand (0-2, or fewer near end of game)
}

/// Current state of a round
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoundState {
    AwaitingFirstCard,  // Waiting for first player to play
    AwaitingSecondCard, // Waiting for second player to play
}

/// Complete game state for Briscola
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BriscolaGameState {
    // Player hands - will be Vec<Card> for own hand, empty Vec for opponent
    pub player1_hand: Vec<Card>,
    pub player2_hand: Vec<Card>,

    // Cards currently on the table: (card, player_who_played_it)
    pub table: Vec<(Card, PlayerSymbol)>,

    // The deck of remaining cards (shuffled once at game start)
    // This will be empty Vec when sent to clients
    pub deck: Vec<Card>,

    // Number of cards left in deck (visible to both players)
    // Note: This counts deck.len() only, NOT including the trump card
    pub cards_remaining_in_deck: usize,

    // The trump card (visible to both players, None after it's drawn)
    pub trump_card: Option<Card>,

    // Collected cards (for scoring)
    pub player1_pile: Vec<Card>,
    pub player2_pile: Vec<Card>,

    // Whose turn is it
    pub current_player: PlayerSymbol,

    // Current round state
    pub round_state: RoundState,
}

impl BriscolaGameState {
    /// Create a new empty game state (used for testing)
    pub fn new() -> Self {
        Self {
            player1_hand: Vec::new(),
            player2_hand: Vec::new(),
            table: Vec::new(),
            deck: Vec::new(),
            cards_remaining_in_deck: 0,
            trump_card: None,
            player1_pile: Vec::new(),
            player2_pile: Vec::new(),
            current_player: 1,
            round_state: RoundState::AwaitingFirstCard,
        }
    }

    /// Redact opponent's hand and deck for a specific player
    pub fn redact_for_player(&self, player: PlayerSymbol) -> Self {
        let mut redacted = self.clone();

        // Hide opponent's hand (replace with empty Vec)
        if player == 1 {
            redacted.player2_hand = Vec::new();
        } else {
            redacted.player1_hand = Vec::new();
        }

        // Hide deck (replace with empty Vec, but keep cards_remaining_in_deck)
        redacted.deck = Vec::new();

        // Keep everything else visible (table, trump, piles, own hand, cards_remaining_in_deck)
        redacted
    }

    /// Calculate score from collected piles
    pub fn get_score(&self) -> (u8, u8) {
        let p1_score = self.player1_pile.iter().map(Self::card_points).sum();
        let p2_score = self.player2_pile.iter().map(Self::card_points).sum();
        (p1_score, p2_score)
    }

    /// Check if game is finished
    pub fn is_finished(&self) -> bool {
        // All 40 cards have been played
        // (both hands empty and deck is empty and no trump card left)
        self.player1_hand.is_empty()
            && self.player2_hand.is_empty()
            && self.deck.is_empty()
            && self.trump_card.is_none()
    }

    /// Get the winner (if finished)
    pub fn get_winner(&self) -> Option<PlayerSymbol> {
        if !self.is_finished() {
            return None;
        }

        let (p1_score, p2_score) = self.get_score();
        if p1_score > p2_score {
            Some(1)
        } else if p2_score > p1_score {
            Some(2)
        } else {
            None // Tie
        }
    }

    /// Helper: Get point value of a card
    pub fn card_points(card: &Card) -> u8 {
        match card.rank {
            Rank::Ace => 11,    // Asso
            Rank::Three => 10,  // Tre
            Rank::King => 4,    // Re
            Rank::Knight => 3,  // Cavallo
            Rank::Jack => 2,    // Fante
            _ => 0,             // 2, 4, 5, 6, 7 have no points
        }
    }
}

impl Default for BriscolaGameState {
    fn default() -> Self {
        Self::new()
    }
}
