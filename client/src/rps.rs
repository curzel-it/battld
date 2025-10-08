use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();

    println!("\nJoining matchmaking for {:?}...\n", game_type);

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    // Print all WebSocket messages as they come in
    loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            println!("Received: {:?}", msg);
            io::stdout().flush()?;

            // Exit on errors
            if let ServerMessage::Error { .. } = msg {
                return Ok(());
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

pub async fn resume_game(_session: &mut SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
