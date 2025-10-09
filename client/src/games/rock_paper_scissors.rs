use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use colored::*;

#[derive(Debug, Clone)]
struct RoundResult {
    player1_move: Option<RPSMove>,
    player2_move: Option<RPSMove>,
}

#[derive(Debug, Clone)]
enum RockPaperScissorsUiState {
    WaitingForOpponentToJoin,
    SelectMove {
        match_data: Match,
        previous_rounds: Vec<RoundResult>,
        opponent_selected: bool,
        you_selected: bool,
    },
    WaitingForOpponentToReconnect {
        match_data: Match,
        previous_rounds: Vec<RoundResult>,
    },
    MatchEndedYouWon(Match),
    MatchEndedYouLost(Match),
    MatchEndedDraw(Match),
    MatchEndedOpponentDisconnected(Match),
}

impl RockPaperScissorsUiState {
    fn render(&self, my_player_number: i32) {
        crate::ui::clear_screen().ok();

        match self {
            RockPaperScissorsUiState::WaitingForOpponentToJoin => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                println!("{}", "  Waiting for opponent to join...".yellow());
                println!();
            }
            RockPaperScissorsUiState::SelectMove {
                match_data: _,
                previous_rounds,
                opponent_selected,
                you_selected,
            } => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();

                // Display previous rounds
                if !previous_rounds.is_empty() {
                    println!("{}", "  Previous Rounds:".bold());
                    println!();
                    for (i, round) in previous_rounds.iter().enumerate() {
                        let (my_move, opponent_move) = if my_player_number == 1 {
                            (&round.player1_move, &round.player2_move)
                        } else {
                            (&round.player2_move, &round.player1_move)
                        };

                        let result = determine_round_winner(my_move, opponent_move, my_player_number);
                        let result_str = match result {
                            RoundWinner::You => "WIN".bright_green().bold(),
                            RoundWinner::Opponent => "LOSS".red(),
                            RoundWinner::Draw => "DRAW".yellow(),
                        };

                        println!(
                            "    Round {}: {} vs {} - {}",
                            i + 1,
                            format_move(my_move).bright_blue(),
                            format_move(opponent_move).bright_magenta(),
                            result_str
                        );
                    }
                    println!();
                }

                // Display current round status
                println!("{}", "  Current Round:".bold());
                println!();

                if *opponent_selected {
                    println!("{}", "    Opponent has selected their move".dimmed());
                } else {
                    println!("{}", "    Opponent is choosing...".dimmed());
                }

                if *you_selected {
                    println!("{}", "    You have selected your move".dimmed());
                    println!();
                    println!("{}", "  Waiting for results...".yellow());
                } else {
                    println!("{}", "    You haven't selected yet".dimmed());
                    println!();
                    println!("{}", "  SELECT YOUR MOVE".bright_green().bold());
                    println!();
                    println!("{}", "  Enter your choice (rock/paper/scissors):".dimmed());
                    print!("  > ");
                    io::stdout().flush().ok();
                }
                println!();
            }
            RockPaperScissorsUiState::WaitingForOpponentToReconnect {
                match_data: _,
                previous_rounds,
            } => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();

                if !previous_rounds.is_empty() {
                    println!("{}", "  Previous Rounds:".bold());
                    println!();
                    for (i, round) in previous_rounds.iter().enumerate() {
                        let (my_move, opponent_move) = if my_player_number == 1 {
                            (&round.player1_move, &round.player2_move)
                        } else {
                            (&round.player2_move, &round.player1_move)
                        };

                        println!(
                            "    Round {}: {} vs {}",
                            i + 1,
                            format_move(my_move).bright_blue(),
                            format_move(opponent_move).bright_magenta()
                        );
                    }
                    println!();
                }

                println!("{}", "  Opponent disconnected. Waiting for reconnection...".yellow());
                println!();
            }
            RockPaperScissorsUiState::MatchEndedYouWon(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  YOU WON! 🎉".bright_green().bold());
                println!();
            }
            RockPaperScissorsUiState::MatchEndedYouLost(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  You lost.".red());
                println!();
            }
            RockPaperScissorsUiState::MatchEndedDraw(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_final_results(match_data, my_player_number);
                println!();
                println!("{}", "  It's a draw!".yellow());
                println!();
            }
            RockPaperScissorsUiState::MatchEndedOpponentDisconnected(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Rock-Paper-Scissors".bright_cyan().bold());
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

enum RoundWinner {
    You,
    Opponent,
    Draw,
}

fn determine_round_winner(my_move: &Option<RPSMove>, opponent_move: &Option<RPSMove>, _my_player_number: i32) -> RoundWinner {
    match (my_move, opponent_move) {
        (Some(RPSMove::Rock), Some(RPSMove::Scissors)) => RoundWinner::You,
        (Some(RPSMove::Paper), Some(RPSMove::Rock)) => RoundWinner::You,
        (Some(RPSMove::Scissors), Some(RPSMove::Paper)) => RoundWinner::You,
        (Some(RPSMove::Rock), Some(RPSMove::Paper)) => RoundWinner::Opponent,
        (Some(RPSMove::Paper), Some(RPSMove::Scissors)) => RoundWinner::Opponent,
        (Some(RPSMove::Scissors), Some(RPSMove::Rock)) => RoundWinner::Opponent,
        (Some(a), Some(b)) if matches!((a, b), (RPSMove::Rock, RPSMove::Rock) | (RPSMove::Paper, RPSMove::Paper) | (RPSMove::Scissors, RPSMove::Scissors)) => RoundWinner::Draw,
        _ => RoundWinner::Draw,
    }
}

fn format_move(m: &Option<RPSMove>) -> String {
    match m {
        Some(RPSMove::Rock) => "Rock".to_string(),
        Some(RPSMove::Paper) => "Paper".to_string(),
        Some(RPSMove::Scissors) => "Scissors".to_string(),
        Some(RPSMove::Redacted) => "???".to_string(),
        None => "---".to_string(),
    }
}

fn render_final_results(match_data: &Match, my_player_number: i32) {
    if let Ok(game_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
        println!("{}", "  Final Results:".bold());
        println!();

        let mut my_wins = 0;
        let mut opponent_wins = 0;
        let mut draws = 0;

        for (i, (p1_move, p2_move)) in game_state.rounds.iter().enumerate() {
            let (my_move, opponent_move) = if my_player_number == 1 {
                (p1_move, p2_move)
            } else {
                (p2_move, p1_move)
            };

            let result = determine_round_winner(my_move, opponent_move, my_player_number);
            let result_str = match result {
                RoundWinner::You => {
                    my_wins += 1;
                    "WIN".bright_green().bold()
                }
                RoundWinner::Opponent => {
                    opponent_wins += 1;
                    "LOSS".red()
                }
                RoundWinner::Draw => {
                    draws += 1;
                    "DRAW".yellow()
                }
            };

            println!(
                "    Round {}: {} vs {} - {}",
                i + 1,
                format_move(my_move).bright_blue(),
                format_move(opponent_move).bright_magenta(),
                result_str
            );
        }

        println!();
        println!("  Score: {} - {} (Draws: {})",
            my_wins.to_string().bright_green(),
            opponent_wins.to_string().red(),
            draws.to_string().yellow()
        );
    }
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    let mut my_number: Option<i32> = None;
    let mut ui_state = RockPaperScissorsUiState::WaitingForOpponentToJoin;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    // Initial render
    ui_state.render(my_number.unwrap_or(1));

    // Main game loop
    loop {
        let waiting_for_input = matches!(
            ui_state,
            RockPaperScissorsUiState::SelectMove { you_selected: false, .. }
        );

        tokio::select! {
            // Poll for incoming WebSocket messages
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    // Handle errors
                    if let ServerMessage::Error { message } = &msg {
                        println!("\n{}", format!("Error: {}", message).red());
                        io::stdout().flush()?;
                        continue;
                    }

                    // Update state based on message
                    match &msg {
                        ServerMessage::PlayerDisconnected { player_id } => {
                            // Check if it's the opponent
                            if *player_id != my_player_id {
                                opponent_disconnected = true;

                                // Transition to WaitingForOpponentToReconnect if not already selected
                                if let RockPaperScissorsUiState::SelectMove {
                                    match_data,
                                    previous_rounds,
                                    you_selected: false,
                                    ..
                                } = &ui_state {
                                    ui_state = RockPaperScissorsUiState::WaitingForOpponentToReconnect {
                                        match_data: match_data.clone(),
                                        previous_rounds: previous_rounds.clone(),
                                    };
                                    ui_state.render(my_number.unwrap_or(1));
                                }
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            // Get the last match data from current state
                            let final_match = match &ui_state {
                                RockPaperScissorsUiState::SelectMove { match_data, .. } |
                                RockPaperScissorsUiState::WaitingForOpponentToReconnect { match_data, .. } => match_data.clone(),
                                _ => {
                                    println!("\n{}", "Match ended".yellow());
                                    println!("\nPress any key to return to main menu...");
                                    io::stdout().flush()?;
                                    crate::wait_for_keypress()?;
                                    return Ok(());
                                }
                            };

                            ui_state = match reason {
                                MatchEndReason::Disconnection => {
                                    RockPaperScissorsUiState::MatchEndedOpponentDisconnected(final_match)
                                }
                                MatchEndReason::Ended => {
                                    // Check outcome
                                    if let Some(outcome) = &final_match.outcome {
                                        match outcome {
                                            MatchOutcome::Player1Win => {
                                                if my_number == Some(1) {
                                                    RockPaperScissorsUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    RockPaperScissorsUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Player2Win => {
                                                if my_number == Some(2) {
                                                    RockPaperScissorsUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    RockPaperScissorsUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Draw => {
                                                RockPaperScissorsUiState::MatchEndedDraw(final_match)
                                            }
                                        }
                                    } else {
                                        RockPaperScissorsUiState::MatchEndedDraw(final_match)
                                    }
                                }
                            };

                            ui_state.render(my_number.unwrap_or(1));
                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::ui::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                            // Determine which player we are (1 or 2)
                            if my_number.is_none() {
                                my_number = Some(if match_data.player1_id == my_player_id { 1 } else { 2 });
                            }

                            // Check if match has ended (in the match data itself)
                            if !match_data.in_progress {
                                ui_state = if let Some(outcome) = &match_data.outcome {
                                    match outcome {
                                        MatchOutcome::Player1Win => {
                                            if my_number == Some(1) {
                                                RockPaperScissorsUiState::MatchEndedYouWon(match_data.clone())
                                            } else {
                                                RockPaperScissorsUiState::MatchEndedYouLost(match_data.clone())
                                            }
                                        }
                                        MatchOutcome::Player2Win => {
                                            if my_number == Some(2) {
                                                RockPaperScissorsUiState::MatchEndedYouWon(match_data.clone())
                                            } else {
                                                RockPaperScissorsUiState::MatchEndedYouLost(match_data.clone())
                                            }
                                        }
                                        MatchOutcome::Draw => {
                                            RockPaperScissorsUiState::MatchEndedDraw(match_data.clone())
                                        }
                                    }
                                } else {
                                    RockPaperScissorsUiState::MatchEndedDraw(match_data.clone())
                                };

                                ui_state.render(my_number.unwrap());
                                println!("\nPress any key to return to main menu...");
                                io::stdout().flush()?;
                                crate::ui::wait_for_keypress()?;
                                return Ok(());
                            }

                            // Parse game state
                            if let Ok(game_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                                // Extract previous rounds (all completed rounds)
                                let previous_rounds: Vec<RoundResult> = game_state.rounds.iter()
                                    .filter(|(p1, p2)| p1.is_some() && p2.is_some())
                                    .map(|(p1, p2)| RoundResult {
                                        player1_move: p1.clone(),
                                        player2_move: p2.clone(),
                                    })
                                    .collect();

                                // Check current round status
                                if let Some(current_round) = game_state.rounds.last() {
                                    let (you_selected, opponent_selected) = match my_number.unwrap() {
                                        1 => (current_round.0.is_some(), current_round.1.is_some()),
                                        2 => (current_round.1.is_some(), current_round.0.is_some()),
                                        _ => (false, false),
                                    };

                                    // Check if we're transitioning to a state where we can select
                                    let was_waiting = matches!(
                                        ui_state,
                                        RockPaperScissorsUiState::SelectMove { you_selected: true, .. } |
                                        RockPaperScissorsUiState::WaitingForOpponentToReconnect { .. } |
                                        RockPaperScissorsUiState::WaitingForOpponentToJoin
                                    );

                                    // If we're now able to select but weren't before, drain buffered input
                                    if !you_selected && was_waiting {
                                        crate::ui::drain_stdin_buffer();
                                        input_line.clear();
                                    }

                                    // If opponent reconnected, clear the flag
                                    if opponent_disconnected {
                                        opponent_disconnected = false;
                                    }

                                    ui_state = RockPaperScissorsUiState::SelectMove {
                                        match_data: match_data.clone(),
                                        previous_rounds,
                                        opponent_selected,
                                        you_selected,
                                    };

                                    ui_state.render(my_number.unwrap());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },

            // Poll for user input (only when in SelectMove state and not yet selected)
            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                if let Ok(_) = result {
                    let move_str = input_line.trim().to_lowercase();
                    input_line.clear();

                    // Skip empty input
                    if move_str.is_empty() {
                        continue;
                    }

                    // Parse the move
                    let move_choice = match move_str.as_str() {
                        "rock" => Some("rock"),
                        "paper" => Some("paper"),
                        "scissors" => Some("scissors"),
                        _ => None,
                    };

                    if let Some(move_name) = move_choice {
                        // Send move
                        let move_data = serde_json::json!({
                            "choice": move_name
                        });
                        ws_client.send(ClientMessage::MakeMove { move_data })?;

                        // Transition state to you_selected = true
                        if let RockPaperScissorsUiState::SelectMove {
                            match_data,
                            previous_rounds,
                            opponent_selected,
                            ..
                        } = &ui_state {
                            ui_state = if opponent_disconnected {
                                RockPaperScissorsUiState::WaitingForOpponentToReconnect {
                                    match_data: match_data.clone(),
                                    previous_rounds: previous_rounds.clone(),
                                }
                            } else {
                                RockPaperScissorsUiState::SelectMove {
                                    match_data: match_data.clone(),
                                    previous_rounds: previous_rounds.clone(),
                                    opponent_selected: *opponent_selected,
                                    you_selected: true,
                                }
                            };
                            ui_state.render(my_number.unwrap());
                        }
                    } else {
                        println!("{}", "Invalid move. Please enter 'rock', 'paper', or 'scissors'.".red());
                        print!("  > ");
                        io::stdout().flush()?;
                    }
                }
            }
        }
    }
}

/// Game state for Rock-Paper-Scissors
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RPSGameState {
    pub rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RPSMove {
    Rock,
    Paper,
    Scissors,
    Redacted,
}

pub async fn resume_game(session: &mut SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    // Determine my player number
    let my_number = if game_match.player1_id == my_player_id {
        Some(1)
    } else {
        Some(2)
    };

    // Reconstruct previous rounds from match state
    let mut previous_rounds = Vec::new();
    if let Ok(game_state) = serde_json::from_value::<RPSGameState>(game_match.game_state.clone()) {
        for (p1, p2) in &game_state.rounds {
            if p1.is_some() && p2.is_some() {
                previous_rounds.push(RoundResult {
                    player1_move: p1.clone(),
                    player2_move: p2.clone(),
                });
            }
        }
    }

    // Start in SelectMove state
    let mut ui_state = RockPaperScissorsUiState::SelectMove {
        match_data: game_match,
        previous_rounds,
        opponent_selected: false,
        you_selected: false,
    };

    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    // Initial render
    ui_state.render(my_number.unwrap_or(1));

    // Main game loop (same as start_game)
    loop {
        let waiting_for_input = matches!(
            ui_state,
            RockPaperScissorsUiState::SelectMove { you_selected: false, .. }
        );

        tokio::select! {
            // Poll for incoming WebSocket messages
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    // Handle errors
                    if let ServerMessage::Error { message } = &msg {
                        println!("\n{}", format!("Error: {}", message).red());
                        io::stdout().flush()?;
                        continue;
                    }

                    // Update state based on message
                    match &msg {
                        ServerMessage::PlayerDisconnected { player_id } => {
                            // Check if it's the opponent
                            if *player_id != my_player_id {
                                opponent_disconnected = true;

                                // Transition to WaitingForOpponentToReconnect if not already selected
                                if let RockPaperScissorsUiState::SelectMove {
                                    match_data,
                                    previous_rounds,
                                    you_selected: false,
                                    ..
                                } = &ui_state {
                                    ui_state = RockPaperScissorsUiState::WaitingForOpponentToReconnect {
                                        match_data: match_data.clone(),
                                        previous_rounds: previous_rounds.clone(),
                                    };
                                    ui_state.render(my_number.unwrap_or(1));
                                }
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            // Get the last match data from current state
                            let final_match = match &ui_state {
                                RockPaperScissorsUiState::SelectMove { match_data, .. } |
                                RockPaperScissorsUiState::WaitingForOpponentToReconnect { match_data, .. } => match_data.clone(),
                                _ => {
                                    println!("\n{}", "Match ended".yellow());
                                    println!("\nPress any key to return to main menu...");
                                    io::stdout().flush()?;
                                    crate::wait_for_keypress()?;
                                    return Ok(());
                                }
                            };

                            ui_state = match reason {
                                MatchEndReason::Disconnection => {
                                    RockPaperScissorsUiState::MatchEndedOpponentDisconnected(final_match)
                                }
                                MatchEndReason::Ended => {
                                    // Check outcome
                                    if let Some(outcome) = &final_match.outcome {
                                        match outcome {
                                            MatchOutcome::Player1Win => {
                                                if my_number == Some(1) {
                                                    RockPaperScissorsUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    RockPaperScissorsUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Player2Win => {
                                                if my_number == Some(2) {
                                                    RockPaperScissorsUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    RockPaperScissorsUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Draw => {
                                                RockPaperScissorsUiState::MatchEndedDraw(final_match)
                                            }
                                        }
                                    } else {
                                        RockPaperScissorsUiState::MatchEndedDraw(final_match)
                                    }
                                }
                            };

                            ui_state.render(my_number.unwrap_or(1));

                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::GameStateUpdate { match_data } => {
                            // Update state based on new match data
                            match &ui_state {
                                RockPaperScissorsUiState::WaitingForOpponentToJoin => {
                                    // Reconstruct round results from server game state
                                    let mut rounds = Vec::new();
                                    if let Ok(rps_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                                        for (p1, p2) in &rps_state.rounds {
                                            if p1.is_some() && p2.is_some() {
                                                rounds.push(RoundResult {
                                                    player1_move: p1.clone(),
                                                    player2_move: p2.clone(),
                                                });
                                            }
                                        }
                                    }

                                    ui_state = RockPaperScissorsUiState::SelectMove {
                                        match_data: match_data.clone(),
                                        previous_rounds: rounds,
                                        opponent_selected: false,
                                        you_selected: false,
                                    };
                                    ui_state.render(my_number.unwrap_or(1));
                                }
                                RockPaperScissorsUiState::SelectMove {
                                    previous_rounds: old_rounds,
                                    you_selected,
                                    ..
                                } => {
                                    // Reconstruct round results from server game state
                                    let mut rounds = Vec::new();
                                    if let Ok(rps_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                                        for (p1, p2) in &rps_state.rounds {
                                            if p1.is_some() && p2.is_some() {
                                                rounds.push(RoundResult {
                                                    player1_move: p1.clone(),
                                                    player2_move: p2.clone(),
                                                });
                                            }
                                        }
                                    }

                                    // If we got a new round result, reset selection state
                                    let (new_you_selected, new_opponent_selected) = if rounds.len() > old_rounds.len() {
                                        (false, false)
                                    } else {
                                        (*you_selected, false)
                                    };

                                    ui_state = RockPaperScissorsUiState::SelectMove {
                                        match_data: match_data.clone(),
                                        previous_rounds: rounds,
                                        opponent_selected: new_opponent_selected,
                                        you_selected: new_you_selected,
                                    };

                                    // If opponent reconnected, clear the flag
                                    if opponent_disconnected {
                                        opponent_disconnected = false;
                                    }

                                    ui_state.render(my_number.unwrap_or(1));
                                }
                                RockPaperScissorsUiState::WaitingForOpponentToReconnect { .. } => {
                                    // Opponent reconnected, transition back to SelectMove
                                    opponent_disconnected = false;

                                    // Reconstruct round results
                                    let mut rounds = Vec::new();
                                    if let Ok(rps_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                                        for (p1, p2) in &rps_state.rounds {
                                            if p1.is_some() && p2.is_some() {
                                                rounds.push(RoundResult {
                                                    player1_move: p1.clone(),
                                                    player2_move: p2.clone(),
                                                });
                                            }
                                        }
                                    }

                                    ui_state = RockPaperScissorsUiState::SelectMove {
                                        match_data: match_data.clone(),
                                        previous_rounds: rounds,
                                        opponent_selected: false,
                                        you_selected: false,
                                    };
                                    ui_state.render(my_number.unwrap_or(1));
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Read user input only when waiting for input
            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                match result {
                    Ok(_) => {
                        let move_str = input_line.trim().to_lowercase();
                        input_line.clear();

                        // Skip empty input
                        if move_str.is_empty() {
                            continue;
                        }

                        // Parse move
                        let move_choice = match move_str.as_str() {
                            "rock" => Some("rock"),
                            "paper" => Some("paper"),
                            "scissors" => Some("scissors"),
                            _ => None,
                        };

                        if let Some(move_name) = move_choice {
                            // Send move to server
                            let move_data = serde_json::json!({
                                "choice": move_name
                            });
                            ws_client.send(ClientMessage::MakeMove {
                                move_data,
                            })?;

                            // Update UI state
                            if let RockPaperScissorsUiState::SelectMove {
                                match_data,
                                previous_rounds,
                                opponent_selected,
                                ..
                            } = &ui_state
                            {
                                ui_state = if opponent_disconnected {
                                    RockPaperScissorsUiState::WaitingForOpponentToReconnect {
                                        match_data: match_data.clone(),
                                        previous_rounds: previous_rounds.clone(),
                                    }
                                } else {
                                    RockPaperScissorsUiState::SelectMove {
                                        match_data: match_data.clone(),
                                        previous_rounds: previous_rounds.clone(),
                                        opponent_selected: *opponent_selected,
                                        you_selected: true,
                                    }
                                };
                                ui_state.render(my_number.unwrap_or(1));
                            }
                        } else {
                            println!("{}", "Invalid choice. Please enter rock, paper, or scissors.".red());
                            print!("  > ");
                            io::stdout().flush()?;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {e}");
                        return Err(Box::new(e));
                    }
                }
            }
        }
    }
}
