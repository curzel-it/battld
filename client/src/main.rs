pub mod api;
pub mod auth;
pub mod config;
pub mod leaderboard;
pub mod state;
pub mod stats;
pub mod tris;
pub mod ui;
pub mod utils;
pub mod websocket;

use colored::*;
use crossterm::{event::{self, Event}, terminal};
use rustyline::DefaultEditor;
use std::io::{self, Write};

use auth::try_auto_login;
use leaderboard::*;
use state::*;
use stats::*;
use ui::*;
use utils::VERSION;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());

    if let Err(e) = start_app(&config_path).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn start_app(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize session
    let mut session = SessionState::new_with_config(config_path)?;

    // Try automatic login
    match try_auto_login(&mut session).await {
        Ok(true) => {
            println!("{}", "✓ Logged in successfully".green());
        }
        Ok(false) | Err(_) => {
            println!("{}", "Please login or create an account:".dimmed());

            // If auto-login fails, try interactive login/registration
            if let Err(e) = auth::handle_login_command(&mut session).await {
                eprintln!("Login failed: {e}");
                return Err("Authentication required".into());
            }
        }
    }

    // Check for resumable match after login
    if let Err(e) = check_and_handle_resumable_match(&mut session).await {
        println!("{}", format!("Resume check error: {e}").yellow());
    }

    // Enter main menu loop
    loop {
        match read_menu_choice(&mut session).await? {
            MenuChoice::StartTicTacToe => {
                // Start TicTacToe game flow
                if let Err(e) = start_game_flow(&mut session, battld_common::GameType::TicTacToe).await {
                    println!("{}", format!("Game error: {e}").red());
                    println!("\nPress any key to return to menu...");
                    wait_for_keypress()?;
                }
            }
            MenuChoice::StartRPS => {
                // Start Rock-Paper-Scissors game flow
                if let Err(e) = start_game_flow(&mut session, battld_common::GameType::RockPaperScissors).await {
                    println!("{}", format!("Game error: {e}").red());
                    println!("\nPress any key to return to menu...");
                    wait_for_keypress()?;
                }
            }
            MenuChoice::Stats => {
                if let Err(e) = show_stats(&mut session).await {
                    println!("{}", format!("Error loading stats: {e}").red());
                }
                println!("\nPress any key to return to menu...");
                wait_for_keypress()?;
            }
            MenuChoice::Leaderboard => {
                if let Err(e) = show_leaderboard(&mut session).await {
                    println!("{}", format!("Error loading leaderboard: {e}").red());
                }
                println!("\nPress any key to return to menu...");
                wait_for_keypress()?;
            }
            MenuChoice::Exit => {
                println!("\n{}", "Goodbye!".cyan());
                break;
            }
        }
    }

    Ok(())
}

enum MenuChoice {
    StartTicTacToe,
    StartRPS,
    Stats,
    Leaderboard,
    Exit,
}

fn display_menu(title: &str, items: &[(String, String)]) {
    clear_screen().ok();

    // ASCII art logo
    println!();
    println!();
    println!("{}", "░█▀▄░█▀█░▀█▀░▀█▀░█░░░█▀▄".bright_cyan());
    println!("{}", "░█▀▄░█▀█░░█░░░█░░█░░░█░█".bright_cyan());
    println!("{}", "░▀▀░░▀░▀░░▀░░░▀░░▀▀▀░▀▀░".bright_cyan());
    println!();

    // Title
    println!("{}", title.dimmed());
    println!();

    // Menu items with numbers
    for (num, text) in items {
        println!("  {}. {}", num.bright_yellow(), text);
    }

    println!();
}

async fn read_menu_choice(_session: &mut SessionState) -> io::Result<MenuChoice> {
    let menu_items = vec![
        ("1".to_string(), "Start Tic-Tac-Toe Game".to_string()),
        ("2".to_string(), "Start Rock-Paper-Scissors Game".to_string()),
        ("3".to_string(), "Your Stats".to_string()),
        ("4".to_string(), "Leaderboard".to_string()),
        ("5".to_string(), "Exit".to_string()),
    ];

    let title = format!("v{VERSION}");
    display_menu(&title, &menu_items);

    let mut rl = DefaultEditor::new().map_err(io::Error::other)?;

    loop {
        let readline = rl.readline("Select option: ");
        match readline {
            Ok(line) => {
                let choice = line.trim();
                match choice {
                    "1" => return Ok(MenuChoice::StartTicTacToe),
                    "2" => return Ok(MenuChoice::StartRPS),
                    "3" => return Ok(MenuChoice::Stats),
                    "4" => return Ok(MenuChoice::Leaderboard),
                    "5" => return Ok(MenuChoice::Exit),
                    _ => {
                        println!("{}", "Invalid choice. Please enter 1-5.".red());
                        continue;
                    }
                }
            }
            Err(_) => {
                return Ok(MenuChoice::Exit);
            }
        }
    }
}

async fn check_and_handle_resumable_match(session: &mut SessionState) -> Result<(), Box<dyn std::error::Error>> {
    use battld_common::*;

    let ws_client = session.ws_client.as_ref().ok_or("Not connected to WebSocket")?;

    // Wait a bit for server to send ResumableMatch message after auth
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let messages = ws_client.get_messages().await;

    for msg in messages {
        if let ServerMessage::ResumableMatch { match_data } = msg {
            clear_screen()?;
            println!("\n{}", "You have an active match!".yellow().bold());
            println!("{}", format!("Match ID: {}", match_data.id).dimmed());
            println!("{}", format!("Opponent: Player {}", if match_data.player1_id == session.player_id.unwrap() { match_data.player2_id } else { match_data.player1_id }).dimmed());
            println!();
            println!("{}", "Do you want to resume? (y/n)".cyan());
            print!("> ");
            io::stdout().flush()?;

            let mut rl = DefaultEditor::new().map_err(io::Error::other)?;

            loop {
                let readline = rl.readline("> ");
                match readline {
                    Ok(line) => {
                        let choice = line.trim().to_lowercase();
                        match choice.as_str() {
                            "y" | "yes" => {
                                // Send ResumeMatch
                                ws_client.send(ClientMessage::ResumeMatch)?;

                                // Wait for GameStateUpdate and resume game
                                println!("\n{}", "Resuming match...".cyan());
                                let game_match = wait_for_game_state(ws_client).await?;
                                tris::resume_game(session, game_match).await?;
                                return Ok(());
                            }
                            "n" | "no" => {
                                println!("\n{}", "Match declined. You will be disconnected from it.".yellow());
                                println!("{}", "Press any key to continue...".dimmed());
                                wait_for_keypress()?;
                                return Ok(());
                            }
                            _ => {
                                println!("{}", "Invalid choice. Please enter 'y' or 'n'.".red());
                                continue;
                            }
                        }
                    }
                    Err(_) => {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

async fn wait_for_game_state(ws_client: &crate::websocket::WebSocketClient) -> Result<battld_common::Match, Box<dyn std::error::Error>> {
    use battld_common::*;

    loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            if let ServerMessage::GameStateUpdate { match_data } = msg {
                return Ok(match_data);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

async fn start_game_flow(session: &mut SessionState, game_type: battld_common::GameType) -> Result<(), Box<dyn std::error::Error>> {
    clear_screen()?;

    let game_name = match game_type {
        battld_common::GameType::TicTacToe => "Tic-Tac-Toe",
        battld_common::GameType::RockPaperScissors => "Rock-Paper-Scissors",
    };

    println!("\n{}", format!("Starting {} matchmaking...", game_name).cyan());
    println!("{}", "Waiting for opponent...".dimmed());

    // Connect to WebSocket and join matchmaking
    tris::start_game(session, game_type).await?;

    Ok(())
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