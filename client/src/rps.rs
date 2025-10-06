use colored::*;
use crossterm::{event::{self, Event}, terminal};
use rustyline::DefaultEditor;
use std::io::{self, Write};
use battld_common::*;
use crate::state::SessionState;
use crate::websocket::WebSocketClient;

// Local representation of RPS game state for client
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
struct RPSGameState {
    rounds: Vec<(Option<String>, Option<String>)>,
}

impl RPSGameState {
    fn current_round(&self) -> usize {
        self.rounds.len()
    }

    fn get_score(&self) -> (u8, u8) {
        let mut p1_wins = 0;
        let mut p2_wins = 0;

        for round in &self.rounds {
            if let (Some(p1_move), Some(p2_move)) = round {
                if let Some(winner) = Self::compute_round_winner(p1_move, p2_move) {
                    if winner == 1 {
                        p1_wins += 1;
                    } else {
                        p2_wins += 1;
                    }
                }
            }
        }

        (p1_wins, p2_wins)
    }

    fn compute_round_winner(p1_move: &str, p2_move: &str) -> Option<i32> {
        match (p1_move, p2_move) {
            ("rock", "scissors") | ("paper", "rock") | ("scissors", "paper") => Some(1),
            ("scissors", "rock") | ("rock", "paper") | ("paper", "scissors") => Some(2),
            _ => None, // Draw
        }
    }

    fn is_finished(&self) -> bool {
        let (p1_wins, p2_wins) = self.get_score();
        p1_wins >= 2 || p2_wins >= 2
    }
}

/// Helper to extract RPS game state from Match
fn get_rps_state(game_match: &Match) -> Result<RPSGameState, Box<dyn std::error::Error>> {
    serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| format!("Failed to deserialize game state: {}", e).into())
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    // Wait for match
    let game_match = 'outer: loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            match msg {
                ServerMessage::WaitingForOpponent => {
                    // Continue waiting
                }
                ServerMessage::MatchFound { match_data } => {
                    crate::ui::clear_screen()?;
                    println!("{}", "Match found!".green().bold());
                    break 'outer match_data;
                }
                ServerMessage::GameStateUpdate { match_data } => {
                    crate::ui::clear_screen()?;
                    println!("{}", "Match found!".green().bold());
                    break 'outer match_data;
                }
                ServerMessage::Error { message } => {
                    return Err(format!("Error: {message}").into());
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    // Start game loop
    run_game_loop(session, game_match).await
}

pub async fn resume_game(session: &mut SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    run_game_loop(session, game_match).await
}

async fn run_game_loop(session: &mut SessionState, mut game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.unwrap();
    let my_number = if game_match.player1_id == my_player_id { 1 } else { 2 };

    loop {
        crate::ui::clear_screen()?;

        // Display current match state
        display_rps_match(&game_match, my_number)?;

        // Check if game is over
        if !game_match.in_progress {
            println!("\n{}", "Match ended!".yellow().bold());

            match &game_match.outcome {
                Some(MatchOutcome::Player1Win) => {
                    if my_number == 1 {
                        println!("{}", "You won the match! 🎉".green().bold());
                    } else {
                        println!("{}", "You lost the match.".red());
                    }
                }
                Some(MatchOutcome::Player2Win) => {
                    if my_number == 2 {
                        println!("{}", "You won the match! 🎉".green().bold());
                    } else {
                        println!("{}", "You lost the match.".red());
                    }
                }
                Some(MatchOutcome::Draw) => {
                    println!("{}", "Match ended in a draw.".yellow());
                }
                None => {}
            }

            println!("\nPress any key to return to menu...");
            wait_for_keypress()?;
            return Ok(());
        }

        // Determine if we need to make a move
        let state = get_rps_state(&game_match)?;
        let current_round_idx = state.rounds.len() - 1;
        let current_round = &state.rounds[current_round_idx];

        let my_move_submitted = match my_number {
            1 => current_round.0.is_some(),
            2 => current_round.1.is_some(),
            _ => false,
        };

        if my_move_submitted {
            // We've already moved, wait for opponent
            println!("\n{}", "Waiting for opponent's move...".dimmed());

            let (new_match, new_status) = wait_for_game_update(ws_client).await?;
            game_match = new_match;

            if let Some(status) = new_status {
                println!("\n{}", status.yellow());
            }
        } else {
            // We need to make a move
            println!("\n{}", "Your turn! Choose your move:".cyan().bold());
            println!("  1. Rock");
            println!("  2. Paper");
            println!("  3. Scissors");
            print!("\nEnter choice (1-3): ");
            io::stdout().flush()?;

            let choice = read_rps_input()?;

            // Send move
            let move_data = serde_json::json!({"choice": choice});
            ws_client.send(ClientMessage::MakeMove { move_data })?;

            println!("\n{}", "Move submitted! Waiting for opponent...".dimmed());

            // Wait for response
            let (new_match, new_status) = wait_for_game_update(ws_client).await?;
            game_match = new_match;

            if let Some(status) = new_status {
                println!("\n{}", status.yellow());
            }
        }
    }
}

fn display_rps_match(game_match: &Match, my_number: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "═══════════════════════════════════════".bright_cyan());
    println!("{}", "    ROCK  ·  PAPER  ·  SCISSORS".bright_cyan().bold());
    println!("{}", "═══════════════════════════════════════".bright_cyan());
    println!();

    let state = get_rps_state(game_match)?;
    let (p1_wins, p2_wins) = state.get_score();

    // Display score
    let my_score = if my_number == 1 { p1_wins } else { p2_wins };
    let opp_score = if my_number == 1 { p2_wins } else { p1_wins };

    println!("{}", format!("Score: You {} - {} Opponent (First to 2 wins)", my_score, opp_score).bold());
    println!();

    // Display round history
    if state.rounds.len() > 1 || state.rounds[0].0.is_some() {
        println!("{}", "Round History:".dimmed());
        for (idx, round) in state.rounds.iter().enumerate() {
            if let (Some(p1_move), Some(p2_move)) = round {
                let round_num = idx + 1;
                let my_move = if my_number == 1 { p1_move } else { p2_move };
                let opp_move = if my_number == 1 { p2_move } else { p1_move };

                let result = match RPSGameState::compute_round_winner(p1_move, p2_move) {
                    Some(winner) => {
                        if (winner == 1 && my_number == 1) || (winner == 2 && my_number == 2) {
                            "You won".green()
                        } else {
                            "You lost".red()
                        }
                    }
                    None => "Draw".yellow(),
                };

                println!(
                    "  Round {}: {} vs {} - {}",
                    round_num,
                    capitalize(my_move),
                    capitalize(opp_move),
                    result
                );
            }
        }
        println!();
    }

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn read_rps_input() -> Result<String, Box<dyn std::error::Error>> {
    let mut rl = DefaultEditor::new().map_err(io::Error::other)?;

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let choice = line.trim();
                match choice {
                    "1" => return Ok("rock".to_string()),
                    "2" => return Ok("paper".to_string()),
                    "3" => return Ok("scissors".to_string()),
                    _ => {
                        println!("{}", "Invalid choice. Please enter 1, 2, or 3.".red());
                        print!("Enter choice (1-3): ");
                        io::stdout().flush()?;
                        continue;
                    }
                }
            }
            Err(_) => {
                return Err("Input error".into());
            }
        }
    }
}

async fn wait_for_game_update(ws_client: &WebSocketClient) -> Result<(Match, Option<String>), Box<dyn std::error::Error>> {
    loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            match msg {
                ServerMessage::GameStateUpdate { match_data } => {
                    return Ok((match_data, None));
                }
                ServerMessage::MatchEnded { reason } => {
                    let status = match reason {
                        MatchEndReason::Ended => None,
                        MatchEndReason::Disconnection => Some("Match ended due to disconnection".to_string()),
                    };
                    // Wait a bit for final GameStateUpdate
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                ServerMessage::Error { message } => {
                    return Err(format!("Error: {message}").into());
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

fn wait_for_keypress() -> io::Result<()> {
    terminal::enable_raw_mode()?;

    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(_) = event::read()? {
                break Ok(());
            }
        }
    };

    terminal::disable_raw_mode()?;
    result
}
