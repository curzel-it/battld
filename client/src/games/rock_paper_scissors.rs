use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;
    let mut my_number: Option<i32> = None;

    println!("\nJoining matchmaking for {:?}...\n", game_type);

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    // Print all WebSocket messages as they come in
    loop {
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

            // Check for match end
            match &msg {
                ServerMessage::MatchEnded { reason } => {
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
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

pub async fn resume_game(_session: &mut SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
