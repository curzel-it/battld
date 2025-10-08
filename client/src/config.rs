use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub player_id: Option<i64>,
    pub private_key_path: Option<String>,
    pub public_key_path: Option<String>,
    pub server_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            player_id: None,
            private_key_path: Some("private_key.pem".to_string()),
            public_key_path: Some("public_key.pem".to_string()),
            server_url: Some("http://127.0.0.1:8080".to_string()),
        }
    }
}

impl Config {
    pub fn load() -> std::result::Result<Config, Box<dyn std::error::Error>> {
        Self::load_from("config.json")
    }

    pub fn load_from(config_path: &str) -> std::result::Result<Config, Box<dyn std::error::Error>> {
        if Path::new(config_path).exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.save_to("config.json")
    }

    pub fn save_to(&self, config_path: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn has_keys(&self) -> bool {
        if let (Some(private_path), Some(public_path)) = (&self.private_key_path, &self.public_key_path) {
            Path::new(private_path).exists() && Path::new(public_path).exists()
        } else {
            false
        }
    }
}