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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn is_finished(&self) -> bool {
        let (p1_wins, p2_wins) = self.get_score();
        p1_wins >= 2 || p2_wins >= 2
    }
}

/// Helper to extract RPS game state from Match
fn get_rps_state(game_match: &Match) -> Result<RPSGameState, Box<dyn std::error::Error>> {
    serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| format!("Failed to deserialize game state: {e}").into())
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    // Wait for match by polling current_match
    let game_match = loop {
        // Check for errors
        let messages = ws_client.get_messages().await;
        for msg in messages {
            if let ServerMessage::Error { message } = msg {
                return Err(format!("Error: {message}").into());
            }
        }

        // Check if a match was found
        if let Some(match_data) = ws_client.get_current_match().await {
            crate::ui::clear_screen()?;
            println!("{}", "Match found!".green().bold());
            break match_data;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    // Start game loop
    run_game_loop(session, game_match).await
}

pub async fn resume_game(session: &mut SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    run_game_loop(session, game_match).await
}

async fn run_game_loop(session: &mut SessionState, initial_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.unwrap();
    let my_number = if initial_match.player1_id == my_player_id { 1 } else { 2 };

    loop {
        // Always read the latest state from WebSocket - single source of truth
        let game_match = ws_client.get_current_match().await
            .ok_or("No active match")?;

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

        // Check if we've moved in the current (last) round
        let my_move_submitted = if let Some(last_round) = state.rounds.last() {
            match my_number {
                1 => last_round.0.is_some(),
                2 => last_round.1.is_some(),
                _ => false,
            }
        } else {
            false
        };

        if my_move_submitted {
            // We've already moved, wait for opponent
            println!("\n{}", "Waiting for opponent's move...".dimmed());

            wait_for_game_state_change(ws_client).await?;
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

            // Wait for state change (server will update state)
            wait_for_game_state_change(ws_client).await?;
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

    // DEBUG: Log the state we're displaying
    eprintln!("[DEBUG] Displaying state: {:?}", state);

    let (p1_wins, p2_wins) = state.get_score();

    // DEBUG: Log the scores
    eprintln!("[DEBUG] Scores: p1={}, p2={}", p1_wins, p2_wins);

    // Display score
    let my_score = if my_number == 1 { p1_wins } else { p2_wins };
    let opp_score = if my_number == 1 { p2_wins } else { p1_wins };

    println!("{}", format!("Score: You {my_score} - {opp_score} Opponent (First to 2 wins)").bold());
    println!();

    // Display round history
    if state.rounds.len() > 1 || state.rounds[0].0.is_some() || state.rounds[0].1.is_some() {
        println!("{}", "Round History:".dimmed());
        for (idx, round) in state.rounds.iter().enumerate() {
            let round_num = idx + 1;
            match round {
                (Some(p1_move), Some(p2_move)) => {
                    // Completed round - show result
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
                (Some(p1_move), None) if my_number == 1 => {
                    // Player 1 moved, waiting for player 2
                    println!(
                        "  Round {}: {} vs {} - {}",
                        round_num,
                        capitalize(p1_move),
                        "???".dimmed(),
                        "Waiting...".dimmed()
                    );
                }
                (None, Some(p2_move)) if my_number == 2 => {
                    // Player 2 moved, waiting for player 1
                    println!(
                        "  Round {}: {} vs {} - {}",
                        round_num,
                        capitalize(p2_move),
                        "???".dimmed(),
                        "Waiting...".dimmed()
                    );
                }
                _ => {
                    // Either both null or opponent moved but we haven't
                    // Don't display anything for this round
                }
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

async fn wait_for_game_state_change(ws_client: &WebSocketClient) -> Result<(), Box<dyn std::error::Error>> {
    // Capture current state
    let current_state_json = ws_client.get_current_match().await
        .map(|m| serde_json::to_string(&m.game_state).unwrap_or_default());

    eprintln!("[DEBUG] wait_for_game_state_change: waiting for change from: {:?}", current_state_json);

    loop {
        // Check for error messages
        let messages = ws_client.get_messages().await;
        for msg in messages {
            if let ServerMessage::Error { message } = msg {
                return Err(format!("Error: {message}").into());
            }
        }

        // Check if state has changed
        if let Some(new_match) = ws_client.get_current_match().await {
            let new_state_json = serde_json::to_string(&new_match.game_state)?;

            if Some(new_state_json.clone()) != current_state_json {
                eprintln!("[DEBUG] wait_for_game_state_change: state changed to: {}", new_state_json);
                return Ok(());
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
