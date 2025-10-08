use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write, BufRead};

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

            // Check if we need to enable user input
            match &msg {
                ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                    // Determine which player we are (1 or 2)
                    if my_number.is_none() {
                        my_number = Some(if match_data.player1_id == my_player_id { 1 } else { 2 });
                        println!("You are player {}", my_number.unwrap());
                    }

                    // Parse game state
                    if let Ok(game_state) = serde_json::from_value::<GameState>(match_data.game_state.clone()) {
                        // Check if it's our turn
                        if game_state.current_player == my_number.unwrap() && !game_state.is_finished {
                            println!("\nYour turn! Enter move as 'row col' (0-indexed, e.g., '1 2'): ");
                            io::stdout().flush()?;

                            // Read user input
                            let stdin = io::stdin();
                            let mut line = String::new();
                            stdin.lock().read_line(&mut line)?;

                            let parts: Vec<&str> = line.trim().split_whitespace().collect();
                            if parts.len() == 2 {
                                if let (Ok(row), Ok(col)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                    // Send move
                                    let move_data = serde_json::json!({
                                        "row": row,
                                        "col": col
                                    });
                                    ws_client.send(ClientMessage::MakeMove { move_data })?;
                                    println!("Move sent: {} {}", row, col);
                                } else {
                                    println!("Invalid input format. Use two numbers separated by space.");
                                }
                            } else {
                                println!("Invalid input format. Use 'row col' (e.g., '1 2')");
                            }
                            io::stdout().flush()?;
                        }
                    }
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

pub async fn resume_game(_session: &SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
