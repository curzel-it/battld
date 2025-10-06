use std::fs;
use battld_common::not_so_secret;
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs8::{EncodePrivateKey, DecodePrivateKey}, pkcs1::LineEnding};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::{Pkcs1v15Sign, sha2::Sha256};
use base64::{Engine as _, engine::general_purpose};
use colored::*;

use crate::api;
use crate::state::*;

pub async fn handle_login_command(session: &mut SessionState) -> Result<(), Box<dyn std::error::Error>> {
    if session.is_authenticated {
        println!("{}", format!("Already logged in as player {}, logging out first...", session.player_id.unwrap()).dimmed());
        session.logout();
    }

    login_interactive(session).await
}

pub async fn login_interactive(session: &mut SessionState) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let player_id = session.config.player_id;
    let has_keys = session.config.has_keys();

    match (player_id, has_keys) {
        // Case 1: No config.json or no player_id and no keys - new user, create everything
        (None, false) => {
            println!("{}", "New user setup - generating key pair and creating account...".dimmed());

            let private_key_path = session.config.private_key_path.as_ref().unwrap();
            let public_key_path = session.config.public_key_path.as_ref().unwrap();

            // Generate key pair
            generate_key_pair(private_key_path, public_key_path)?;

            // Get player name
            println!("Enter your player name:");
            let mut name = String::new();
            std::io::stdin().read_line(&mut name)?;
            let name = name.trim();

            // Create player on server
            let player = api::auth::create_player(session.config.server_url.as_ref().unwrap(), name, public_key_path).await?;

            // Update config with player ID
            session.config.player_id = Some(player.id);
            session.save_config()?;
            session.player_id = Some(player.id);

            println!("{}", format!("Account created successfully! Player ID: {}", player.id).dimmed());

            // Perform authentication after account creation
            let token = signed_token(private_key_path)?;

            match api::auth::test_authentication(session.config.server_url.as_ref().unwrap(), player.id, &token).await {
                Ok(_) => {
                    // Store just the signature, not the player_id:signature format
                    session.set_authenticated(player.id, token.clone());
                    println!("{}", "Authentication successful!".dimmed());

                    // Connect WebSocket
                    if let Err(e) = session.connect_websocket().await {
                        println!("{}", format!("WebSocket connection failed: {e}").yellow());
                    } else {
                        println!("{}", "WebSocket connected".dimmed());
                    }

                    println!("{}", format!("You are now logged in as player {}", player.id).dimmed());
                },
                Err(e) => {
                    println!("{}", format!("Authentication failed: {e}").dimmed());
                    return Err("Authentication test failed after account creation".into());
                }
            }
        },

        // Case 2: No player_id but has keys - new user with existing keys
        (None, true) => {
            println!("{}", "Found existing keys - creating account...".dimmed());

            // Get player name
            println!("Enter your player name:");
            let mut name = String::new();
            std::io::stdin().read_line(&mut name)?;
            let name = name.trim();

            // Create player on server
            let player = api::auth::create_player(session.config.server_url.as_ref().unwrap(), name, session.config.public_key_path.as_ref().unwrap()).await?;

            // Update config with player ID
            session.config.player_id = Some(player.id);
            session.save_config()?;
            session.player_id = Some(player.id);

            println!("{}", format!("Account created successfully! Player ID: {}", player.id).dimmed());

            // Perform authentication after account creation
            let token = signed_token(session.config.private_key_path.as_ref().unwrap())?;

            match api::auth::test_authentication(session.config.server_url.as_ref().unwrap(), player.id, &token).await {
                Ok(_) => {
                    // Store just the signature, not the player_id:signature format
                    session.set_authenticated(player.id, token.clone());
                    println!("{}", "Authentication successful!".dimmed());

                    // Connect WebSocket
                    if let Err(e) = session.connect_websocket().await {
                        println!("{}", format!("WebSocket connection failed: {e}").yellow());
                    } else {
                        println!("{}", "WebSocket connected".dimmed());
                    }

                    println!("{}", format!("You are now logged in as player {}", player.id).dimmed());
                },
                Err(e) => {
                    println!("{}", format!("Authentication failed: {e}").dimmed());
                    return Err("Authentication test failed after account creation".into());
                }
            }
        },

        // Case 3: Has player_id but no keys - error, need keys for existing account
        (Some(pid), false) => {
            println!("{}", format!("Error: Found player ID {pid} but no SSH keys.").dimmed());
            println!("{}", "You need the private/public key pair to login to an existing account.".dimmed());
            println!("{}", "Options:".dimmed());
            println!("{}", "1. Place your keys at the configured paths:".dimmed());
            println!("{}", format!("   - Private key: {}", session.config.private_key_path.as_ref().unwrap_or(&"private_key.pem".to_string())).dimmed());
            println!("{}", format!("   - Public key: {}", session.config.public_key_path.as_ref().unwrap_or(&"public_key.pem".to_string())).dimmed());
            println!("{}", "2. Or remove the player_id from config.json to create a new account".dimmed());
            return Err("Missing keys for existing account".into());
        },

        // Case 4: Has player_id and keys - regular login
        (Some(pid), true) => {
            println!("{}", format!("Logging in as player {pid}...").dimmed());
            let token = signed_token(session.config.private_key_path.as_ref().unwrap())?;

            // Test authentication by making an authenticated request to get player info
            match api::auth::test_authentication(session.config.server_url.as_ref().unwrap(), pid, &token).await {
                Ok(_) => {
                    // Store just the signature, not the player_id:signature format
                    session.set_authenticated(pid, token.clone());
                    println!("{}", "Authentication successful!".dimmed());

                    // Connect WebSocket
                    if let Err(e) = session.connect_websocket().await {
                        println!("{}", format!("WebSocket connection failed: {e}").yellow());
                    } else {
                        println!("{}", "WebSocket connected".dimmed());
                    }

                    println!("{}", format!("You are now logged in as player {pid}").dimmed());
                },
                Err(e) => {
                    println!("{}", format!("Authentication failed: {e}").dimmed());
                    return Err("Authentication test failed".into());
                }
            }
        }
    }

    Ok(())
}

pub async fn try_auto_login(session: &mut SessionState) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    if let Some(player_id) = session.config.player_id {
        if session.config.has_keys() {
            println!("{}", "Attempting automatic login...".dimmed());
            let token = signed_token(session.config.private_key_path.as_ref().unwrap())?;

            // Test authentication
            match api::auth::test_authentication(session.config.server_url.as_ref().unwrap(), player_id, &token).await {
                Ok(_) => {
                    // Store just the signature, not the player_id:signature format
                    session.set_authenticated(player_id, token.clone());
                    println!("{}", "Automatic login successful!".green());

                    // Connect WebSocket
                    if let Err(e) = session.connect_websocket().await {
                        println!("{}", format!("WebSocket connection failed: {e}").yellow());
                    } else {
                        println!("{}", "WebSocket connected".dimmed());
                    }

                    println!("{}", format!("Logged in as player {player_id}").dimmed());
                    return Ok(true);
                },
                Err(e) => {
                    println!("{}", format!("Automatic login failed: {e}").red());
                    return Ok(false);
                }
            }
        }
    }
    Ok(false)
}


fn generate_key_pair(private_key_path: &str, public_key_path: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use rand::rngs::OsRng;

    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;
    let public_key = RsaPublicKey::from(&private_key);

    // Save private key in PKCS#8 PEM format
    let private_pem = private_key.to_pkcs8_pem(LineEnding::LF)?;
    fs::write(private_key_path, private_pem.as_bytes())?;

    // Save public key in PKCS#1 PEM format (same as server expects)
    let public_pem = public_key.to_pkcs1_pem(LineEnding::LF)?;
    fs::write(public_key_path, public_pem)?;

    println!("{}", "Generated new RSA key pair:".dimmed());
    println!("{}", format!("  Private key: {private_key_path}").dimmed());
    println!("{}", format!("  Public key: {public_key_path}").dimmed());

    Ok(())
}

pub fn signed_token(private_key_path: &str,) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let (random_string, _) = not_so_secret();
    let private_key_pem = fs::read_to_string(private_key_path)?;
    let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_pem)?;

    // Hash the random string first, then sign
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(random_string.as_bytes());
    let hashed = hasher.finalize();

    // Sign the hash using PKCS1v15 with SHA256
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let signature = private_key.sign(padding, &hashed)?;

    // Encode signature to base64
    Ok(general_purpose::STANDARD.encode(signature))
}