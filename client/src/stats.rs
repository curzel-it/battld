use battld_common::HEADER_AUTH;
use colored::*;

use crate::state::*;
use crate::ui::*;

pub async fn show_stats(session: &mut SessionState) -> Result<(), Box<dyn std::error::Error>> {
    use battld_common::PlayerStats;

    clear_screen()?;
    println!("\n{}", "Loading your stats...".cyan());

    if !session.is_authenticated {
        return Err("Not authenticated".into());
    }

    let config = &session.config;
    let server_url = config.server_url.as_ref().ok_or("No server URL configured")?;
    let token = session.auth_token.as_ref().ok_or("No auth token")?;

    let client = reqwest::Client::new();
    let url = format!("{server_url}/stats");

    let response = client
        .get(&url)
        .header(HEADER_AUTH, format!("Bearer {token}"))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()).into());
    }

    let stats: PlayerStats = response.json().await?;

    clear_screen()?;
    println!();
    println!("{}", "═══════════════════════════════════════".bright_cyan());
    println!("{}", "            YOUR STATISTICS            ".bright_cyan().bold());
    println!("{}", "═══════════════════════════════════════".bright_cyan());
    println!();
    println!("  {} {}", "Total Matches:".bright_white(), stats.total.to_string().bright_yellow());
    println!("  {} {}", "Won:         ".bright_green(), stats.won.to_string().bright_green().bold());
    println!("  {} {}", "Lost:        ".bright_red(), stats.lost.to_string().bright_red());
    println!("  {} {}", "Draw:        ".bright_blue(), stats.draw.to_string().bright_blue());
    println!("  {} {}", "Dropped:     ".dimmed(), stats.dropped.to_string().dimmed());
    println!();
    println!("  {} {}", "Score:       ".bright_yellow().bold(), stats.score.to_string().bright_yellow().bold());
    println!();
    println!("{}", "═══════════════════════════════════════".bright_cyan());

    Ok(())
}