use battld_common::games::{
    briscola::{BriscolaGameState, BriscolaMove, Card, Rank, RoundState, Suit},
    players::PlayerSymbol,
};
use rand::seq::SliceRandom;
use rand::thread_rng;

use super::GameError;

/// Stateless Briscola game engine
pub struct BriscolaGameEngine;

impl BriscolaGameEngine {
    /// Create a new game with shuffled deck
    pub fn new() -> BriscolaGameState {
        let mut deck = Self::create_and_shuffle_deck();

        // Deal 3 cards to each player
        let mut player1_hand = Vec::new();
        let mut player2_hand = Vec::new();
        for _ in 0..3 {
            player1_hand.push(deck.pop().unwrap());
        }
        for _ in 0..3 {
            player2_hand.push(deck.pop().unwrap());
        }

        // Set trump card
        let trump_card = deck.pop().unwrap();
        let briscola_suit = trump_card.suit;

        // Remaining deck has 33 cards
        let cards_remaining_in_deck = deck.len();

        BriscolaGameState {
            player1_hand,
            player2_hand,
            table: Vec::new(),
            deck,
            cards_remaining_in_deck,
            trump_card: Some(trump_card),
            briscola_suit,
            player1_pile: Vec::new(),
            player2_pile: Vec::new(),
            current_player: 1, // Will be randomized in initialize_game_state
            round_state: RoundState::AwaitingFirstCard,
            previous_round: None,
        }
    }

    /// Update game state with a player's move
    pub fn update(
        &self,
        state: &BriscolaGameState,
        player: PlayerSymbol,
        move_choice: BriscolaMove,
    ) -> Result<BriscolaGameState, GameError> {
        // 1. Validate game is in progress
        if state.is_finished() {
            return Err(GameError::GameNotInProgress);
        }

        // 2. Validate player number
        if player != 1 && player != 2 {
            return Err(GameError::InvalidPlayer);
        }

        // 3. Validate it's the player's turn
        if state.current_player != player {
            return Err(GameError::WrongTurn);
        }

        // 4. Extract card index from move
        let BriscolaMove::PlayCard { card_index } = move_choice;

        // 5. Validate player has card at that index
        let hand = if player == 1 {
            &state.player1_hand
        } else {
            &state.player2_hand
        };
        if card_index >= hand.len() {
            return Err(GameError::IllegalMove("Invalid card index".to_string()));
        }

        // 6. Get the card being played
        let card = hand[card_index];

        // 7. Create new state and remove card from hand
        let mut new_state = state.clone();
        if player == 1 {
            new_state.player1_hand.remove(card_index);
        } else {
            new_state.player2_hand.remove(card_index);
        }

        // 8. Add card to table
        new_state.table.push((card, player));

        // 9. Handle based on round state
        match state.round_state {
            RoundState::AwaitingFirstCard => {
                // First card played, switch to waiting for second
                new_state.round_state = RoundState::AwaitingSecondCard;
                new_state.current_player = if player == 1 { 2 } else { 1 };
            }
            RoundState::AwaitingSecondCard => {
                // Second card played, resolve the round
                new_state = Self::resolve_round(new_state)?;
            }
        }

        Ok(new_state)
    }

    /// Create and shuffle a 40-card deck
    fn create_and_shuffle_deck() -> Vec<Card> {
        let mut deck = Vec::new();

        // Create all 40 cards
        for suit in [Suit::Bastoni, Suit::Coppe, Suit::Denari, Suit::Spade] {
            for rank in [
                Rank::Ace,
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Jack,
                Rank::Knight,
                Rank::King,
            ] {
                deck.push(Card { suit, rank });
            }
        }

        // Shuffle using rand crate
        deck.shuffle(&mut thread_rng());

        deck
    }

    /// Resolve a round after both players have played
    fn resolve_round(mut state: BriscolaGameState) -> Result<BriscolaGameState, GameError> {
        // 1. Determine round winner
        let (first_card, first_player) = state.table[0];
        let (second_card, _second_player) = state.table[1];

        // Use briscola_suit instead of trump_card, so it works even after trump is drawn
        let trump_suit = state.briscola_suit;

        let round_winner = Self::determine_round_winner(first_card, second_card, trump_suit, first_player);

        // 2. Store previous round result before clearing table
        state.previous_round = Some((first_card, second_card, round_winner));

        // 3. Award both cards to winner's pile
        if round_winner == 1 {
            state.player1_pile.push(first_card);
            state.player1_pile.push(second_card);
        } else {
            state.player2_pile.push(first_card);
            state.player2_pile.push(second_card);
        }

        // 4. Clear table
        state.table.clear();

        // 5. Draw new cards (if deck not empty or trump available)
        if !state.deck.is_empty() || state.trump_card.is_some() {
            // Winner draws first
            Self::draw_card_to_player(&mut state, round_winner);

            // Loser draws second (if cards still available)
            if !state.deck.is_empty() || state.trump_card.is_some() {
                let other_player = if round_winner == 1 { 2 } else { 1 };
                Self::draw_card_to_player(&mut state, other_player);
            }
        }

        // 6. Winner of round starts next round
        state.current_player = round_winner;
        state.round_state = RoundState::AwaitingFirstCard;

        Ok(state)
    }

    /// Determine the winner of a round based on Briscola rules
    ///
    /// Rules:
    /// 1. If both cards are briscola (trump), higher rank wins
    /// 2. If only one card is briscola, it wins
    /// 3. If neither is briscola:
    ///    - If same suit as first card, higher rank wins
    ///    - If different suit, first card wins
    fn determine_round_winner(
        first_card: Card,
        second_card: Card,
        trump_suit: Suit,
        first_player: PlayerSymbol,
    ) -> PlayerSymbol {
        let first_is_trump = first_card.suit == trump_suit;
        let second_is_trump = second_card.suit == trump_suit;

        // Case 1: Both are trump - higher rank wins
        if first_is_trump && second_is_trump {
            if Self::rank_value(first_card.rank) > Self::rank_value(second_card.rank) {
                first_player
            } else {
                if first_player == 1 {
                    2
                } else {
                    1
                }
            }
        }
        // Case 2: Only first is trump - first wins
        else if first_is_trump {
            first_player
        }
        // Case 3: Only second is trump - second wins
        else if second_is_trump {
            if first_player == 1 {
                2
            } else {
                1
            }
        }
        // Case 4: Neither is trump
        else {
            // If same suit as first card, higher rank wins
            if first_card.suit == second_card.suit {
                if Self::rank_value(first_card.rank) > Self::rank_value(second_card.rank) {
                    first_player
                } else {
                    if first_player == 1 {
                        2
                    } else {
                        1
                    }
                }
            } else {
                // Different suits, neither trump - first card wins
                first_player
            }
        }
    }

    /// Rank ordering for comparison (higher value = stronger card)
    fn rank_value(rank: Rank) -> u8 {
        match rank {
            Rank::Ace => 11,
            Rank::Three => 10,
            Rank::King => 9,
            Rank::Knight => 8,
            Rank::Jack => 7,
            Rank::Seven => 6,
            Rank::Six => 5,
            Rank::Five => 4,
            Rank::Four => 3,
            Rank::Two => 2,
        }
    }

    /// Draw a card to a player's hand
    fn draw_card_to_player(state: &mut BriscolaGameState, player: PlayerSymbol) {
        let card_to_draw = if !state.deck.is_empty() {
            // Draw from deck
            Some(state.deck.pop().unwrap())
        } else if let Some(trump) = state.trump_card.take() {
            // Deck is empty, draw the trump card
            Some(trump)
        } else {
            None
        };

        if let Some(card) = card_to_draw {
            // Add to appropriate player's hand
            if player == 1 {
                state.player1_hand.push(card);
            } else {
                state.player2_hand.push(card);
            }
            // Update counter: only counts deck cards, not trump
            state.cards_remaining_in_deck = state.deck.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_points() {
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Ace }), 11);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Three }), 10);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::King }), 4);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Knight }), 3);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Jack }), 2);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Two }), 0);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Seven }), 0);
    }

    #[test]
    fn test_new_game_initialization() {
        let state = BriscolaGameEngine::new();

        // Each player should have 3 cards
        assert_eq!(state.player1_hand.len(), 3);
        assert_eq!(state.player2_hand.len(), 3);

        // Trump card should exist
        assert!(state.trump_card.is_some());

        // Deck should have 33 cards (40 - 6 dealt - 1 trump)
        assert_eq!(state.deck.len(), 33);
        assert_eq!(state.cards_remaining_in_deck, 33);

        // Table should be empty
        assert!(state.table.is_empty());

        // Piles should be empty
        assert!(state.player1_pile.is_empty());
        assert!(state.player2_pile.is_empty());

        // Game should not be finished
        assert!(!state.is_finished());
    }

    #[test]
    fn test_round_winner_both_trump_higher_wins() {
        let trump_suit = Suit::Bastoni;
        let first_card = Card { suit: Suit::Bastoni, rank: Rank::Ace };  // Trump, value 11
        let second_card = Card { suit: Suit::Bastoni, rank: Rank::Jack }; // Trump, value 7

        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 1); // First player has higher trump

        let winner = BriscolaGameEngine::determine_round_winner(second_card, first_card, trump_suit, 2);
        assert_eq!(winner, 1); // Second player (1) has higher trump
    }

    #[test]
    fn test_round_winner_one_trump_wins() {
        let trump_suit = Suit::Bastoni;

        // First card is trump
        let first_card = Card { suit: Suit::Bastoni, rank: Rank::Two };
        let second_card = Card { suit: Suit::Coppe, rank: Rank::Ace };
        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 1);

        // Second card is trump
        let first_card = Card { suit: Suit::Coppe, rank: Rank::Ace };
        let second_card = Card { suit: Suit::Bastoni, rank: Rank::Two };
        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 2);
    }

    #[test]
    fn test_round_winner_same_suit_not_trump() {
        let trump_suit = Suit::Bastoni;

        // Same suit, first has higher rank
        let first_card = Card { suit: Suit::Coppe, rank: Rank::Ace };
        let second_card = Card { suit: Suit::Coppe, rank: Rank::Jack };
        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 1);

        // Same suit, second has higher rank
        let first_card = Card { suit: Suit::Coppe, rank: Rank::Jack };
        let second_card = Card { suit: Suit::Coppe, rank: Rank::Ace };
        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 2);
    }

    #[test]
    fn test_round_winner_different_suits_no_trump() {
        let trump_suit = Suit::Bastoni;

        // Different suits, neither trump - first card wins
        let first_card = Card { suit: Suit::Coppe, rank: Rank::Two };
        let second_card = Card { suit: Suit::Denari, rank: Rank::Ace };
        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 1);
        assert_eq!(winner, 1);

        let winner = BriscolaGameEngine::determine_round_winner(first_card, second_card, trump_suit, 2);
        assert_eq!(winner, 2);
    }

    #[test]
    fn test_play_card_valid_first_card() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![
            Card { suit: Suit::Bastoni, rank: Rank::Ace },
            Card { suit: Suit::Coppe, rank: Rank::King },
        ];
        state.player2_hand = vec![
            Card { suit: Suit::Denari, rank: Rank::Jack },
        ];
        state.current_player = 1;
        state.round_state = RoundState::AwaitingFirstCard;
        state.trump_card = Some(Card { suit: Suit::Spade, rank: Rank::Three });

        let engine = BriscolaGameEngine;
        let new_state = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 0 }).unwrap();

        // Hand should have one less card
        assert_eq!(new_state.player1_hand.len(), 1);

        // Card should be on table
        assert_eq!(new_state.table.len(), 1);
        assert_eq!(new_state.table[0].0, Card { suit: Suit::Bastoni, rank: Rank::Ace });
        assert_eq!(new_state.table[0].1, 1);

        // State should change to awaiting second card
        assert_eq!(new_state.round_state, RoundState::AwaitingSecondCard);

        // Current player should switch
        assert_eq!(new_state.current_player, 2);
    }

    #[test]
    fn test_play_card_invalid_index() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.current_player = 1;

        let engine = BriscolaGameEngine;
        let result = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 5 });

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }

    #[test]
    fn test_play_card_wrong_turn() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.player2_hand = vec![Card { suit: Suit::Coppe, rank: Rank::King }];
        state.current_player = 1;

        let engine = BriscolaGameEngine;
        let result = engine.update(&state, 2, BriscolaMove::PlayCard { card_index: 0 });

        assert!(matches!(result, Err(GameError::WrongTurn)));
    }

    #[test]
    fn test_round_resolution() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.player2_hand = vec![Card { suit: Suit::Coppe, rank: Rank::Two }];
        state.deck = vec![Card { suit: Suit::Denari, rank: Rank::King }, Card { suit: Suit::Spade, rank: Rank::Jack }];
        state.cards_remaining_in_deck = 2;
        state.trump_card = Some(Card { suit: Suit::Bastoni, rank: Rank::Three });
        state.current_player = 1;
        state.round_state = RoundState::AwaitingFirstCard;

        let engine = BriscolaGameEngine;

        // Player 1 plays ace of trump
        let state = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 0 }).unwrap();

        // Player 2 plays two of coppe
        let state = engine.update(&state, 2, BriscolaMove::PlayCard { card_index: 0 }).unwrap();

        // Player 1 should win (trump beats non-trump)
        assert_eq!(state.player1_pile.len(), 2);
        assert_eq!(state.player2_pile.len(), 0);

        // Both players should have drawn new cards
        assert_eq!(state.player1_hand.len(), 1);
        assert_eq!(state.player2_hand.len(), 1);

        // Deck should have 0 cards left
        assert_eq!(state.deck.len(), 0);
        assert_eq!(state.cards_remaining_in_deck, 0);

        // Table should be clear
        assert!(state.table.is_empty());

        // Winner should start next round
        assert_eq!(state.current_player, 1);
        assert_eq!(state.round_state, RoundState::AwaitingFirstCard);
    }

    #[test]
    fn test_trump_card_drawing() {
        let mut state = BriscolaGameState::new();
        let trump = Card { suit: Suit::Bastoni, rank: Rank::Three };
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.player2_hand = vec![Card { suit: Suit::Coppe, rank: Rank::Two }];
        state.deck = vec![Card { suit: Suit::Denari, rank: Rank::King }]; // Only 1 card in deck
        state.cards_remaining_in_deck = 1;
        state.trump_card = Some(trump);
        state.current_player = 1;
        state.round_state = RoundState::AwaitingFirstCard;

        let engine = BriscolaGameEngine;

        // Play a round
        let state = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 0 }).unwrap();
        let state = engine.update(&state, 2, BriscolaMove::PlayCard { card_index: 0 }).unwrap();

        // Winner (player 1) draws last deck card, loser draws trump
        assert_eq!(state.deck.len(), 0);
        assert_eq!(state.cards_remaining_in_deck, 0);
        assert_eq!(state.trump_card, None);

        // Both players should have 1 card
        assert_eq!(state.player1_hand.len(), 1);
        assert_eq!(state.player2_hand.len(), 1);

        // One of them should have the trump card
        let has_trump = state.player1_hand.contains(&trump) || state.player2_hand.contains(&trump);
        assert!(has_trump);
    }

    #[test]
    fn test_game_finish() {
        let mut state = BriscolaGameState::new();
        // Set up end-game scenario: no cards anywhere
        state.player1_hand = vec![];
        state.player2_hand = vec![];
        state.deck = vec![];
        state.cards_remaining_in_deck = 0;
        state.trump_card = None;

        // Add some points to piles
        state.player1_pile = vec![
            Card { suit: Suit::Bastoni, rank: Rank::Ace },  // 11 points
            Card { suit: Suit::Coppe, rank: Rank::Three },  // 10 points
        ]; // Total: 21
        state.player2_pile = vec![
            Card { suit: Suit::Denari, rank: Rank::King },  // 4 points
            Card { suit: Suit::Spade, rank: Rank::Jack },   // 2 points
        ]; // Total: 6

        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(1));
        assert_eq!(state.get_score(), (21, 6));
    }

    #[test]
    fn test_game_finish_with_tie() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![];
        state.player2_hand = vec![];
        state.deck = vec![];
        state.cards_remaining_in_deck = 0;
        state.trump_card = None;

        // Equal points
        state.player1_pile = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }]; // 11 points
        state.player2_pile = vec![Card { suit: Suit::Coppe, rank: Rank::Ace }];   // 11 points

        assert!(state.is_finished());
        assert_eq!(state.get_winner(), None);
        assert_eq!(state.get_score(), (11, 11));
    }

    #[test]
    fn test_redaction() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![
            Card { suit: Suit::Bastoni, rank: Rank::Ace },
            Card { suit: Suit::Coppe, rank: Rank::King },
        ];
        state.player2_hand = vec![
            Card { suit: Suit::Denari, rank: Rank::Jack },
            Card { suit: Suit::Spade, rank: Rank::Two },
        ];
        state.deck = vec![Card { suit: Suit::Bastoni, rank: Rank::Three }];
        state.cards_remaining_in_deck = 1;
        state.trump_card = Some(Card { suit: Suit::Spade, rank: Rank::Ace });
        state.table = vec![(Card { suit: Suit::Coppe, rank: Rank::Two }, 1)];

        // Redact for player 1
        let redacted = state.redact_for_player(1);
        assert_eq!(redacted.player1_hand.len(), 2); // Own hand visible
        assert_eq!(redacted.player2_hand.len(), 0); // Opponent hand hidden
        assert_eq!(redacted.deck.len(), 0);         // Deck hidden
        assert_eq!(redacted.cards_remaining_in_deck, 1); // Count still visible
        assert_eq!(redacted.trump_card, state.trump_card); // Trump visible
        assert_eq!(redacted.table, state.table);     // Table visible

        // Redact for player 2
        let redacted = state.redact_for_player(2);
        assert_eq!(redacted.player1_hand.len(), 0); // Opponent hand hidden
        assert_eq!(redacted.player2_hand.len(), 2); // Own hand visible
        assert_eq!(redacted.deck.len(), 0);         // Deck hidden
        assert_eq!(redacted.cards_remaining_in_deck, 1); // Count still visible
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.player2_hand = vec![Card { suit: Suit::Coppe, rank: Rank::King }];
        state.trump_card = Some(Card { suit: Suit::Denari, rank: Rank::Three });

        // Serialize to JSON
        let json = serde_json::to_value(&state).unwrap();

        // Deserialize back
        let deserialized: BriscolaGameState = serde_json::from_value(json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_state_immutability() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.player2_hand = vec![Card { suit: Suit::Coppe, rank: Rank::King }];
        state.current_player = 1;
        state.trump_card = Some(Card { suit: Suit::Spade, rank: Rank::Three });

        let original_hand = state.player1_hand.clone();
        let engine = BriscolaGameEngine;

        // Make a move
        let _new_state = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 0 }).unwrap();

        // Original state should be unchanged
        assert_eq!(state.player1_hand, original_hand);
        assert!(state.table.is_empty());
    }

    #[test]
    fn test_game_already_finished() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![];
        state.player2_hand = vec![];
        state.deck = vec![];
        state.cards_remaining_in_deck = 0;
        state.trump_card = None;

        let engine = BriscolaGameEngine;
        let result = engine.update(&state, 1, BriscolaMove::PlayCard { card_index: 0 });

        assert!(matches!(result, Err(GameError::GameNotInProgress)));
    }

    #[test]
    fn test_invalid_player() {
        let mut state = BriscolaGameState::new();
        state.player1_hand = vec![Card { suit: Suit::Bastoni, rank: Rank::Ace }];
        state.current_player = 1;
        state.trump_card = Some(Card { suit: Suit::Spade, rank: Rank::Three });

        let engine = BriscolaGameEngine;

        let result = engine.update(&state, 3, BriscolaMove::PlayCard { card_index: 0 });
        assert!(matches!(result, Err(GameError::InvalidPlayer)));

        let result = engine.update(&state, 0, BriscolaMove::PlayCard { card_index: 0 });
        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }
}
