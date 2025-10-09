use crate::config::*;
use crate::websocket::WebSocketClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct SessionState {
    pub config: Config,
    pub config_path: String,
    pub player_id: Option<i64>,
    pub auth_token: Option<String>,
    pub is_authenticated: bool,
    pub ws_client: Option<Arc<WebSocketClient>>,
}

impl SessionState {
    pub fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config("config.json")
    }

    pub fn new_with_config(config_path: &str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let config = Config::load_from(config_path)?;
        Ok(SessionState {
            player_id: config.player_id,
            config,
            config_path: config_path.to_string(),
            auth_token: None,
            is_authenticated: false,
            ws_client: None,
        })
    }

    pub fn set_authenticated(&mut self, player_id: i64, token: String) {
        self.player_id = Some(player_id);
        self.auth_token = Some(token);
        self.is_authenticated = true;
    }

    pub async fn connect_websocket(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(token) = &self.auth_token {
            let player_id = self.player_id.ok_or("No player ID")?;
            let server_url = self.config.server_url.as_ref().ok_or("No server URL configured")?;
            let ws_url = format!("{}/ws", server_url.replace("http", "ws"));
            let ws_token = format!("{player_id}:{token}");
            let client = WebSocketClient::connect(&ws_url, ws_token).await?;
            self.ws_client = Some(Arc::new(client));
            Ok(())
        } else {
            Err("Not authenticated".into())
        }
    }

    pub fn logout(&mut self) {
        self.auth_token = None;
        self.is_authenticated = false;
        self.ws_client = None;
    }

    pub fn save_config(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.config.save_to(&self.config_path)
    }
}