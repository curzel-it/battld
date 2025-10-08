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

                    // Check if we need to enable user input
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

                            // Parse game state
                            if let Ok(game_state) = serde_json::from_value::<GameState>(match_data.game_state.clone()) {
                                // Check if it's our turn
                                if game_state.current_player == my_number.unwrap() && !game_state.is_finished && !waiting_for_input {
                                    println!("\nYour turn! Enter move as 'row col' (0-indexed, e.g., '1 2'): ");
                                    io::stdout().flush()?;
                                    waiting_for_input = true;
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
                    let parts: Vec<&str> = input_line.trim().split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(row), Ok(col)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            // Send move
                            let move_data = serde_json::json!({
                                "row": row,
                                "col": col
                            });
                            ws_client.send(ClientMessage::MakeMove { move_data })?;
                            println!("Move sent: {} {}", row, col);
                            io::stdout().flush()?;
                            waiting_for_input = false;
                        } else {
                            println!("Invalid input format. Use two numbers separated by space.");
                            println!("\nYour turn! Enter move as 'row col' (0-indexed, e.g., '1 2'): ");
                            io::stdout().flush()?;
                        }
                    } else {
                        println!("Invalid input format. Use 'row col' (e.g., '1 2')");
                        println!("\nYour turn! Enter move as 'row col' (0-indexed, e.g., '1 2'): ");
                        io::stdout().flush()?;
                    }
                    input_line.clear();
                }
            }
        }
    }
}

pub async fn resume_game(_session: &SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
