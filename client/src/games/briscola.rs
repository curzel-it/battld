use battld_common::{
    games::{
        briscola::{BriscolaGameState, Card, Rank, Suit},
        game_type::GameType,
        matches::{Match, MatchEndReason, MatchOutcome},
    },
    *,
};
use crate::state::SessionState;
use colored::*;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Clone)]
enum BriscolaUiState {
    WaitingForOpponentToJoin,

    PlayingGame {
        match_data: Match,
        your_turn: bool,
        opponent_disconnected: bool,
    },

    WaitingForOpponentToReconnect {
        match_data: Match,
    },

    MatchEndedYouWon(Match),
    MatchEndedYouLost(Match),
    MatchEndedDraw(Match),
    MatchEndedOpponentDisconnected(Match),
}

impl BriscolaUiState {
    fn render(&self, my_player_number: i32) {
        crate::ui::clear_screen().ok();

        match self {
            BriscolaUiState::WaitingForOpponentToJoin => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                println!("{}", "  Waiting for opponent to join...".yellow());
                println!();
            }
            BriscolaUiState::PlayingGame {
                match_data,
                your_turn,
                opponent_disconnected,
            } => {
                let game_state = parse_game_state(match_data);

                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();

                // Scores
                let (p1_score, p2_score) = game_state.get_score();
                let (my_score, opp_score) = if my_player_number == 1 {
                    (p1_score, p2_score)
                } else {
                    (p2_score, p1_score)
                };
                println!(
                    "  Score: {} - {}",
                    format!("You {}", my_score).bright_green(),
                    format!("{} Opponent", opp_score).red()
                );
                println!();

                // Briscola suit
                let briscola_suit_str = match game_state.briscola_suit {
                    Suit::Bastoni => "Bastoni",
                    Suit::Coppe => "Coppe",
                    Suit::Denari => "Denari",
                    Suit::Spade => "Spade",
                };
                println!("  Briscola: {}", briscola_suit_str.yellow().bold());

                // Trump card and deck count
                if let Some(trump) = game_state.trump_card {
                    println!("  Trump card: {}", format_card(&trump).yellow());
                } else {
                    println!("  Trump card: {}", "(drawn)".dimmed());
                }
                println!(
                    "  Deck: {} cards remaining",
                    game_state.cards_remaining_in_deck
                );
                println!();

                // Table (cards played this round)
                if !game_state.table.is_empty() {
                    println!("{}", "  On table:".bold());
                    for (card, player) in &game_state.table {
                        let who = if *player == my_player_number {
                            "You".bright_green()
                        } else {
                            "Opponent".red()
                        };
                        println!("    {} played {}", who, format_card(card));
                    }
                    println!();
                }

                // Opponent's hand (just count)
                let opp_hand = if my_player_number == 1 {
                    &game_state.player2_hand
                } else {
                    &game_state.player1_hand
                };
                println!("  Opponent has {} cards", opp_hand.len());
                println!();

                // Your hand
                let my_hand = if my_player_number == 1 {
                    &game_state.player1_hand
                } else {
                    &game_state.player2_hand
                };
                println!("{}", "  Your hand:".bold());
                print!("{}", display_hand_ascii(my_hand));

                // Input prompt or waiting message
                if *opponent_disconnected {
                    println!("{}", "  Opponent disconnected. Waiting for reconnection...".yellow());
                } else if *your_turn {
                    println!(
                        "{}",
                        "  Your turn! Enter card index:".bright_green().bold()
                    );
                    print!("  > ");
                    io::stdout().flush().ok();
                } else {
                    println!("{}", "  Waiting for opponent...".dimmed());
                }
                println!();
            }
            BriscolaUiState::WaitingForOpponentToReconnect { match_data } => {
                let game_state = parse_game_state(match_data);

                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();

                // Show current game state
                let (p1_score, p2_score) = game_state.get_score();
                let (my_score, opp_score) = if my_player_number == 1 {
                    (p1_score, p2_score)
                } else {
                    (p2_score, p1_score)
                };
                println!("  Score: You {} - {} Opponent", my_score, opp_score);
                println!();

                println!("{}", "  Opponent disconnected. Waiting for reconnection...".yellow());
                println!();
            }
            BriscolaUiState::MatchEndedYouWon(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  YOU WON! 🎉".bright_green().bold());
                println!();
            }
            BriscolaUiState::MatchEndedYouLost(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  You lost.".red());
                println!();
            }
            BriscolaUiState::MatchEndedDraw(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  It's a draw!".yellow());
                println!();
            }
            BriscolaUiState::MatchEndedOpponentDisconnected(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Briscola".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  Match ended - Opponent disconnected.".yellow());
                println!();
            }
        }
    }
}

/// Format a card for display
fn format_card(card: &Card) -> String {
    let suit_str = match card.suit {
        Suit::Bastoni => "Bastoni",
        Suit::Coppe => "Coppe",
        Suit::Denari => "Denari",
        Suit::Spade => "Spade",
    };
    let rank_str = match card.rank {
        Rank::Ace => "A",
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Jack => "J",
        Rank::Knight => "C", // Cavallo
        Rank::King => "K",
    };
    format!("{} {}", rank_str, suit_str)
}

fn render_final_results(match_data: &Match, my_player_number: i32) {
    if let Ok(game_state) = serde_json::from_value::<BriscolaGameState>(match_data.game_state.clone()) {
        let (p1_score, p2_score) = game_state.get_score();
        let (my_score, opp_score) = if my_player_number == 1 {
            (p1_score, p2_score)
        } else {
            (p2_score, p1_score)
        };

        println!("{}", "  Final Score:".bold());
        println!(
            "    You: {} points",
            my_score.to_string().bright_green()
        );
        println!("    Opponent: {} points", opp_score.to_string().red());
    }
}

fn parse_game_state(match_data: &Match) -> BriscolaGameState {
    serde_json::from_value::<BriscolaGameState>(match_data.game_state.clone())
        .unwrap_or_else(|_| BriscolaGameState::new())
}

fn handle_player_disconnected(
    player_id: i64,
    my_player_id: i64,
    ui_state: &BriscolaUiState,
    opponent_disconnected: &mut bool,
    _my_number: i32,
) -> Option<BriscolaUiState> {
    if player_id == my_player_id {
        return None;
    }

    *opponent_disconnected = true;

    if let BriscolaUiState::PlayingGame {
        match_data,
        your_turn: false,
        ..
    } = ui_state
    {
        Some(BriscolaUiState::WaitingForOpponentToReconnect {
            match_data: match_data.clone(),
        })
    } else {
        None
    }
}

fn handle_match_ended(
    reason: &MatchEndReason,
    ui_state: &BriscolaUiState,
    my_number: Option<i32>,
) -> BriscolaUiState {
    let final_match = match ui_state {
        BriscolaUiState::PlayingGame { match_data, .. }
        | BriscolaUiState::WaitingForOpponentToReconnect { match_data } => match_data.clone(),
        _ => return ui_state.clone(),
    };

    match reason {
        MatchEndReason::Disconnection => {
            BriscolaUiState::MatchEndedOpponentDisconnected(final_match)
        }
        MatchEndReason::Ended => determine_match_end_state(&final_match, my_number),
    }
}

fn determine_match_end_state(
    match_data: &Match,
    my_number: Option<i32>,
) -> BriscolaUiState {
    if let Some(outcome) = &match_data.outcome {
        match outcome {
            MatchOutcome::Player1Win => {
                if my_number == Some(1) {
                    BriscolaUiState::MatchEndedYouWon(match_data.clone())
                } else {
                    BriscolaUiState::MatchEndedYouLost(match_data.clone())
                }
            }
            MatchOutcome::Player2Win => {
                if my_number == Some(2) {
                    BriscolaUiState::MatchEndedYouWon(match_data.clone())
                } else {
                    BriscolaUiState::MatchEndedYouLost(match_data.clone())
                }
            }
            MatchOutcome::Draw => BriscolaUiState::MatchEndedDraw(match_data.clone()),
        }
    } else {
        BriscolaUiState::MatchEndedDraw(match_data.clone())
    }
}

fn handle_match_found_or_update(
    match_data: &Match,
    my_player_id: i64,
    my_number: &mut Option<i32>,
    opponent_disconnected: &mut bool,
    ui_state: &BriscolaUiState,
) -> Result<Option<BriscolaUiState>, Box<dyn std::error::Error>> {
    // Determine player number
    if my_number.is_none() {
        *my_number = Some(if match_data.player1_id == my_player_id {
            1
        } else {
            2
        });
    }

    // Check if match has ended
    if !match_data.in_progress {
        return Ok(Some(determine_match_end_state(match_data, *my_number)));
    }

    // Parse game state
    let game_state = serde_json::from_value::<BriscolaGameState>(match_data.game_state.clone())?;

    // Determine if it's your turn
    let your_turn = game_state.current_player == my_number.unwrap();

    // Check if we're transitioning to a state where we can play
    let was_waiting = matches!(
        ui_state,
        BriscolaUiState::PlayingGame { your_turn: false, .. } |
        BriscolaUiState::WaitingForOpponentToReconnect { .. } |
        BriscolaUiState::WaitingForOpponentToJoin
    );

    // If we're now able to play but weren't before, drain buffered input
    if your_turn && was_waiting {
        crate::ui::drain_stdin_buffer();
    }

    // If opponent reconnected, clear the flag
    if *opponent_disconnected {
        *opponent_disconnected = false;
    }

    Ok(Some(BriscolaUiState::PlayingGame {
        match_data: match_data.clone(),
        your_turn,
        opponent_disconnected: *opponent_disconnected,
    }))
}

fn handle_game_state_update(
    match_data: &Match,
    ui_state: &BriscolaUiState,
    my_player_id: i64,
    my_number: &mut Option<i32>,
    opponent_disconnected: &mut bool,
) -> Option<BriscolaUiState> {
    // Use the same logic as match found/update
    match handle_match_found_or_update(match_data, my_player_id, my_number, opponent_disconnected, ui_state)
    {
        Ok(Some(new_state)) => Some(new_state),
        _ => None,
    }
}

fn handle_user_input(
    input_str: &str,
    ui_state: &BriscolaUiState,
    opponent_disconnected: bool,
    ws_client: &crate::websocket::WebSocketClient,
    my_number: i32,
) -> Result<Option<BriscolaUiState>, Box<dyn std::error::Error>> {
    // Parse card index
    let card_index = match input_str.parse::<usize>() {
        Ok(idx) => idx,
        Err(_) => {
            println!("{}", "Invalid input. Please enter a number.".red());
            print!("  > ");
            io::stdout().flush()?;
            return Ok(None);
        }
    };

    // Validate against hand size
    if let BriscolaUiState::PlayingGame { match_data, .. } = ui_state {
        let game_state = parse_game_state(match_data);
        let my_hand = if my_number == 1 {
            &game_state.player1_hand
        } else {
            &game_state.player2_hand
        };

        if card_index >= my_hand.len() {
            println!(
                "{}",
                format!(
                    "Invalid card index. Please enter 0-{}.",
                    my_hand.len() - 1
                )
                .red()
            );
            print!("  > ");
            io::stdout().flush()?;
            return Ok(None);
        }
    }

    // Send move to server
    let move_data = serde_json::json!({
        "card_index": card_index
    });
    ws_client.send(ClientMessage::MakeMove { move_data })?;

    // Update UI state
    if let BriscolaUiState::PlayingGame { match_data, .. } = ui_state {
        let new_state = if opponent_disconnected {
            BriscolaUiState::WaitingForOpponentToReconnect {
                match_data: match_data.clone(),
            }
        } else {
            BriscolaUiState::PlayingGame {
                match_data: match_data.clone(),
                your_turn: false,
                opponent_disconnected: false,
            }
        };
        Ok(Some(new_state))
    } else {
        Ok(None)
    }
}

async fn run_game_loop(
    ws_client: &crate::websocket::WebSocketClient,
    my_player_id: i64,
    initial_state: BriscolaUiState,
    initial_my_number: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut my_number = initial_my_number;
    let mut ui_state = initial_state;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    // Initial render
    ui_state.render(my_number.unwrap_or(1));

    loop {
        let waiting_for_input = matches!(
            ui_state,
            BriscolaUiState::PlayingGame {
                your_turn: true,
                opponent_disconnected: false,
                ..
            }
        );

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    if let ServerMessage::Error { message } = &msg {
                        println!("\n{}", format!("Error: {message}").red());
                        io::stdout().flush()?;
                        continue;
                    }

                    match &msg {
                        ServerMessage::PlayerDisconnected { player_id } => {
                            if let Some(new_state) = handle_player_disconnected(
                                *player_id,
                                my_player_id,
                                &ui_state,
                                &mut opponent_disconnected,
                                my_number.unwrap_or(1),
                            ) {
                                ui_state = new_state;
                                ui_state.render(my_number.unwrap_or(1));
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            ui_state = handle_match_ended(reason, &ui_state, my_number);
                            ui_state.render(my_number.unwrap_or(1));
                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::ui::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } => {
                            if let Ok(Some(new_state)) = handle_match_found_or_update(
                                match_data,
                                my_player_id,
                                &mut my_number,
                                &mut opponent_disconnected,
                                &ui_state,
                            ) {
                                let should_exit = matches!(
                                    new_state,
                                    BriscolaUiState::MatchEndedYouWon(_)
                                        | BriscolaUiState::MatchEndedYouLost(_)
                                        | BriscolaUiState::MatchEndedDraw(_)
                                        | BriscolaUiState::MatchEndedOpponentDisconnected(_)
                                );

                                ui_state = new_state;
                                ui_state.render(my_number.unwrap());

                                if should_exit {
                                    println!("\nPress any key to return to main menu...");
                                    io::stdout().flush()?;
                                    crate::ui::wait_for_keypress()?;
                                    return Ok(());
                                }

                                input_line.clear();
                            }
                        }
                        ServerMessage::GameStateUpdate { match_data } => {
                            if let Some(new_state) = handle_game_state_update(
                                match_data,
                                &ui_state,
                                my_player_id,
                                &mut my_number,
                                &mut opponent_disconnected,
                            ) {
                                ui_state = new_state;
                                ui_state.render(my_number.unwrap());
                                input_line.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }

            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                if result.is_ok() {
                    let input_str = input_line.trim().to_lowercase();
                    input_line.clear();

                    if input_str.is_empty() {
                        continue;
                    }

                    if let Ok(Some(new_state)) = handle_user_input(
                        &input_str,
                        &ui_state,
                        opponent_disconnected,
                        ws_client,
                        my_number.unwrap_or(1),
                    ) {
                        ui_state = new_state;
                        ui_state.render(my_number.unwrap());
                    }
                }
            }
        }
    }
}

pub async fn start_game(
    session: &mut SessionState,
    game_type: GameType,
) -> Result<(), Box<dyn std::error::Error>> {
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    run_game_loop(
        ws_client,
        my_player_id,
        BriscolaUiState::WaitingForOpponentToJoin,
        None,
    )
    .await
}

pub async fn resume_game(
    session: &mut SessionState,
    game_match: Match,
) -> Result<(), Box<dyn std::error::Error>> {
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    let my_number = if game_match.player1_id == my_player_id {
        Some(1)
    } else {
        Some(2)
    };

    let game_state = parse_game_state(&game_match);
    let your_turn = game_state.current_player == my_number.unwrap();

    let initial_state = BriscolaUiState::PlayingGame {
        match_data: game_match,
        your_turn,
        opponent_disconnected: false,
    };

    run_game_loop(ws_client, my_player_id, initial_state, my_number).await
}

/// Returns ASCII art representation of a card as a vector of lines
pub fn card_view(suit: Suit, rank: Rank) -> Vec<String> {
    let rank_str = match rank {
        Rank::Ace => "A",
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Jack => "J",
        Rank::Knight => "C",
        Rank::King => "K",
    };

    let suit_char = match suit {
        Suit::Bastoni => "B",
        Suit::Coppe => "C",
        Suit::Denari => "D",
        Suit::Spade => "S",
    };

    // Generate middle rows based on rank
    let middle_rows = match rank {
        Rank::Ace => vec![
            format!("│       │"),
            format!("│   {}   │", suit_char),
            format!("│       │"),
        ],
        Rank::Two => vec![
            format!("│       │"),
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│       │"),
        ],
        Rank::Three => vec![
            format!("│   {}   │", suit_char),
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│       │"),
        ],
        Rank::Four => vec![
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│       │"),
            format!("│  {} {}  │", suit_char, suit_char),
        ],
        Rank::Five => vec![
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│   {}   │", suit_char),
            format!("│  {} {}  │", suit_char, suit_char),
        ],
        Rank::Six => vec![
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│  {} {}  │", suit_char, suit_char),
        ],
        Rank::Seven => vec![
            format!("│  {} {}  │", suit_char, suit_char),
            format!("│ {} {} {} │", suit_char, suit_char, suit_char),
            format!("│  {} {}  │", suit_char, suit_char),
        ],
        Rank::Jack => vec![
            format!("│     {} │", suit_char),
            format!("│ ╭┼╮╱  │"),
            format!("│ ╭┴╮   │"),
        ],
        Rank::Knight => vec![
            format!("│ ╰┼╯╭{} │", suit_char),
            format!("│╭─┼─┴╮ │"),
            format!("││ ╵  │ │"),
        ],
        Rank::King => vec![
            format!("│╰─┼─╮{} │", suit_char),
            format!("│ ╭┴╮   │"),
            format!("│ │ │   │"),
        ],
    };

    // Build the complete card
    let mut lines = vec![
        String::from("╭───────╮"),
        format!("│     {} │", rank_str),
    ];
    lines.extend(middle_rows);
    lines.push(String::from("╰───────╯"));

    lines
}

/// Display multiple cards side-by-side with indices
fn display_hand_ascii(hand: &[Card]) -> String {
    if hand.is_empty() {
        return String::new();
    }

    // Get ASCII art for each card
    let card_arts: Vec<Vec<String>> = hand
        .iter()
        .map(|card| card_view(card.suit, card.rank))
        .collect();

    let mut output = String::new();

    // Display cards side by side
    for line_idx in 0..6 {
        // 6 lines per card
        output.push_str("  ");
        for card_art in &card_arts {
            output.push_str(&card_art[line_idx]);
            output.push_str("  ");
        }
        output.push('\n');
    }

    // Display indices below cards
    output.push_str("  ");
    for i in 0..hand.len() {
        output.push_str(&format!("   [{}]    ", i));
        output.push_str(" ");
    }
    output.push('\n');

    output
}