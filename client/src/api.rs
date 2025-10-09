use crate::state::SessionState;

/// Authentication API calls
pub mod auth {
    use std::path::Path;
    use std::fs;

    use battld_common::HEADER_AUTH;
    use battld_common::api::{ChallengeRequest, ChallengeResponse, VerifyRequest, AuthResponse};

    pub async fn create_player(server_url: &str, name: &str, public_key_path: &str) -> std::result::Result<battld_common::Player, Box<dyn std::error::Error>> {
        let public_key_pem = fs::read_to_string(public_key_path)?;

        let hint = Path::new(public_key_path)
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("unknown")
            .to_string();

        let request = battld_common::CreatePlayerRequest {
            public_key_hint: hint,
            public_key: public_key_pem,
            name: name.to_string(),
        };

        let client = reqwest::Client::new();
        let url = format!("{server_url}/player");

        let response = client.post(&url).json(&request).send().await?;
        let response_text = response.text().await?;

        let player: battld_common::Player = serde_json::from_str(&response_text)?;
        Ok(player)
    }

    pub async fn request_challenge(
        server_url: &str,
        player_id: i64,
        public_key_hint: &str,
    ) -> std::result::Result<ChallengeResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("{server_url}/auth/challenge");

        let request = ChallengeRequest {
            player_id,
            public_key_hint: public_key_hint.to_string(),
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Challenge request failed: {}", response.status()).into());
        }

        Ok(response.json().await?)
    }

    pub async fn verify_challenge(
        server_url: &str,
        player_id: i64,
        nonce: &str,
        signature: &str,
    ) -> std::result::Result<AuthResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("{server_url}/auth/verify");

        let request = VerifyRequest {
            player_id,
            nonce: nonce.to_string(),
            signature: signature.to_string(),
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Verification failed: {}", response.status()).into());
        }

        Ok(response.json().await?)
    }
}

/// Player data API calls
pub mod player {
    use battld_common::{games::matches::Match, HEADER_AUTH};

    use super::*;

    pub async fn fetch_player_data(session: &SessionState) -> std::result::Result<battld_common::Player, Box<dyn std::error::Error>> {
        if !session.is_authenticated {
            return Err("Not authenticated".into());
        }

        let token = session.auth_token.as_ref().unwrap();
        let server_url = session.config.server_url.as_ref().unwrap();

        let client = reqwest::Client::new();
        let url = format!("{server_url}/player");

        let response = client
            .get(&url)
            .header(HEADER_AUTH, format!("Bearer {token}"))
            .send()
            .await?;

        let response_text = response.text().await?;
        let player: battld_common::Player = serde_json::from_str(&response_text)?;
        Ok(player)
    }

    pub async fn fetch_active_matches(session: &SessionState) -> std::result::Result<Vec<Match>, Box<dyn std::error::Error>> {
        if !session.is_authenticated {
            return Err("Not authenticated".into());
        }

        let token = session.auth_token.as_ref().ok_or("No auth token")?;
        let server_url = session.config.server_url.as_ref().ok_or("No server URL")?;

        let client = reqwest::Client::new();
        let url = format!("{server_url}/matches/active");

        let response = client
            .get(&url)
            .header(HEADER_AUTH, format!("Bearer {token}"))
            .send()
            .await?;

        if response.status() == 401 {
            return Err("Authentication failed - please log in again".into());
        }

        let response_text = response.text().await?;
        let matches: Vec<Match> = serde_json::from_str(&response_text)?;
        Ok(matches)
    }

}
