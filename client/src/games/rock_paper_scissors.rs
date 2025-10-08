use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    println!("\nJoining matchmaking for {:?}...\n", game_type);

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    let mut my_number: Option<i32> = None;
    let mut waiting_for_input = false;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();

    // Print all WebSocket messages as they come in
    loop {
        tokio::select! {
            // Poll for incoming WebSocket messages
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    // Handle GameError by just printing it
                    if let ServerMessage::Error { message } = &msg {
                        println!("Error: {}", message);
                        io::stdout().flush()?;
                        continue;
                    }

                    println!("Received: {:?}", msg);
                    io::stdout().flush()?;

                    // Check for match end or state updates
                    match &msg {
                        ServerMessage::MatchEnded { reason } => {
                            // Cancel any pending input
                            waiting_for_input = false;

                            println!("\n{}", "=".repeat(40));
                            println!("Match ended: {:?}", reason);
                            println!("{}", "=".repeat(40));

                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                            // Determine which player we are (1 or 2)
                            if my_number.is_none() {
                                my_number = Some(if match_data.player1_id == my_player_id { 1 } else { 2 });
                                println!("You are player {}", my_number.unwrap());
                            }

                            // Check if match has ended
                            if !match_data.in_progress {
                                // Cancel any pending input
                                waiting_for_input = false;

                                // Display outcome
                                if let Some(outcome) = &match_data.outcome {
                                    println!("\n{}", "=".repeat(40));
                                    match outcome {
                                        MatchOutcome::Player1Win => {
                                            if my_number == Some(1) {
                                                println!("🎉 You won!");
                                            } else {
                                                println!("You lost.");
                                            }
                                        }
                                        MatchOutcome::Player2Win => {
                                            if my_number == Some(2) {
                                                println!("🎉 You won!");
                                            } else {
                                                println!("You lost.");
                                            }
                                        }
                                        MatchOutcome::Draw => {
                                            println!("It's a draw!");
                                        }
                                    }
                                    println!("{}", "=".repeat(40));
                                }

                                println!("\nPress any key to return to main menu...");
                                io::stdout().flush()?;
                                crate::wait_for_keypress()?;
                                return Ok(());
                            }

                            // Parse game state to check if we can make a move
                            if let Ok(game_state) = serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                                // Check if we haven't submitted a move for the current round yet
                                if let Some(current_round) = game_state.rounds.last() {
                                    let have_submitted = match my_number.unwrap() {
                                        1 => current_round.0.is_some(),
                                        2 => current_round.1.is_some(),
                                        _ => true,
                                    };

                                    // If we haven't submitted and not already waiting for input, prompt for move
                                    if !have_submitted && !waiting_for_input {
                                        println!("\nYour turn! Enter your move (rock/paper/scissors): ");
                                        io::stdout().flush()?;
                                        waiting_for_input = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },

            // Poll for user input (only when waiting_for_input is true)
            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                if let Ok(_) = result {
                    let move_str = input_line.trim().to_lowercase();

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
                        println!("Move sent: {}", move_name);
                        io::stdout().flush()?;
                        waiting_for_input = false;
                    } else {
                        println!("Invalid move. Please enter 'rock', 'paper', or 'scissors'.");
                        println!("\nYour turn! Enter your move (rock/paper/scissors): ");
                        io::stdout().flush()?;
                    }
                    input_line.clear();
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

pub async fn resume_game(_session: &mut SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
