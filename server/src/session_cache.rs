use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionCache {
    sessions: Arc<RwLock<HashMap<String, SessionToken>>>,
}

#[derive(Clone)]
pub struct SessionToken {
    pub token_id: String,
    pub player_id: i64,
    pub issued_at: SystemTime,
    pub expires_at: SystemTime,
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCache {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, player_id: i64) -> String {
        let token_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        // Get session duration from env var, default to 24 hours
        let session_duration_secs = std::env::var("SESSION_DURATION_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86400);

        let expires_at = now + Duration::from_secs(session_duration_secs);

        let mut sessions = self.sessions.write().await;
        sessions.insert(token_id.clone(), SessionToken {
            token_id: token_id.clone(),
            player_id,
            issued_at: now,
            expires_at,
        });

        token_id
    }

    pub async fn verify_session(&self, token_id: &str) -> Result<i64, String> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(token_id).ok_or("Invalid session".to_string())?;

        if SystemTime::now() > session.expires_at {
            return Err("Session expired".to_string());
        }

        Ok(session.player_id)
    }

    pub async fn refresh_session(&self, token_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(token_id).ok_or("Invalid session".to_string())?;

        // Check if session is still valid before refreshing
        if SystemTime::now() > session.expires_at {
            return Err("Session expired".to_string());
        }

        // Get session duration from env var, default to 24 hours
        let session_duration_secs = std::env::var("SESSION_DURATION_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86400);

        // Extend the session
        session.expires_at = SystemTime::now() + Duration::from_secs(session_duration_secs);
        println!("Refreshed session for player {}", session.player_id);

        Ok(())
    }

    pub async fn revoke_session(&self, token_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token_id);
    }

    pub async fn revoke_all_for_player(&self, player_id: i64) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| session.player_id != player_id);
    }

    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().await;
        let now = SystemTime::now();

        sessions.retain(|_, session| session.expires_at > now);
    }

    pub async fn get_active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    pub async fn get_player_sessions(&self, player_id: i64) -> Vec<SessionToken> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| s.player_id == player_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[tokio::test]
    async fn test_create_and_verify_session() {
        let cache = SessionCache::new();
        let token = cache.create_session(123).await;

        let player_id = cache.verify_session(&token).await;
        assert_eq!(player_id.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_invalid_session() {
        let cache = SessionCache::new();
        let result = cache.verify_session("invalid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_session() {
        let cache = SessionCache::new();
        let token = cache.create_session(123).await;

        // Verify it works
        assert!(cache.verify_session(&token).await.is_ok());

        // Revoke it
        cache.revoke_session(&token).await;

        // Should now fail
        assert!(cache.verify_session(&token).await.is_err());
    }

    #[tokio::test]
    async fn test_revoke_all_for_player() {
        let cache = SessionCache::new();
        let token1 = cache.create_session(123).await;
        let token2 = cache.create_session(123).await;
        let token3 = cache.create_session(456).await;

        // Revoke all for player 123
        cache.revoke_all_for_player(123).await;

        // Player 123's tokens should be revoked
        assert!(cache.verify_session(&token1).await.is_err());
        assert!(cache.verify_session(&token2).await.is_err());

        // Player 456's token should still work
        assert!(cache.verify_session(&token3).await.is_ok());
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let cache = SessionCache::new();

        // Manually create an expired session
        let token_id = Uuid::new_v4().to_string();
        {
            let mut sessions = cache.sessions.write().await;
            sessions.insert(token_id.clone(), SessionToken {
                token_id: token_id.clone(),
                player_id: 123,
                issued_at: SystemTime::now() - Duration::from_secs(86401),
                expires_at: SystemTime::now() - Duration::from_secs(1),
            });
        }

        // Should fail due to expiration
        let result = cache.verify_session(&token_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let cache = SessionCache::new();

        // Create a valid session
        let valid_token = cache.create_session(123).await;

        // Manually add an expired session
        let expired_token = Uuid::new_v4().to_string();
        {
            let mut sessions = cache.sessions.write().await;
            sessions.insert(expired_token.clone(), SessionToken {
                token_id: expired_token.clone(),
                player_id: 456,
                issued_at: SystemTime::now() - Duration::from_secs(86401),
                expires_at: SystemTime::now() - Duration::from_secs(1),
            });
        }

        // Before cleanup, we should have 2 sessions
        assert_eq!(cache.get_active_session_count().await, 2);

        // Run cleanup
        cache.cleanup_expired().await;

        // After cleanup, should only have 1
        assert_eq!(cache.get_active_session_count().await, 1);

        // Valid token should still work
        assert!(cache.verify_session(&valid_token).await.is_ok());

        // Expired token should be gone
        assert!(cache.verify_session(&expired_token).await.is_err());
    }

    #[tokio::test]
    async fn test_get_player_sessions() {
        let cache = SessionCache::new();
        let token1 = cache.create_session(123).await;
        let token2 = cache.create_session(123).await;
        let _token3 = cache.create_session(456).await;

        let sessions = cache.get_player_sessions(123).await;
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s.token_id == token1));
        assert!(sessions.iter().any(|s| s.token_id == token2));
    }

    #[tokio::test]
    async fn test_refresh_session() {
        let cache = SessionCache::new();
        let token = cache.create_session(123).await;

        // Get initial expiry time
        let initial_expiry = {
            let sessions = cache.sessions.read().await;
            sessions.get(&token).unwrap().expires_at
        };

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Refresh session
        cache.refresh_session(&token).await.unwrap();

        // Get new expiry time
        let new_expiry = {
            let sessions = cache.sessions.read().await;
            sessions.get(&token).unwrap().expires_at
        };

        // New expiry should be later than initial
        assert!(new_expiry > initial_expiry);
    }

    #[tokio::test]
    async fn test_refresh_expired_session_fails() {
        let cache = SessionCache::new();

        // Manually create an expired session
        let token_id = Uuid::new_v4().to_string();
        {
            let mut sessions = cache.sessions.write().await;
            sessions.insert(token_id.clone(), SessionToken {
                token_id: token_id.clone(),
                player_id: 123,
                issued_at: SystemTime::now() - Duration::from_secs(86401),
                expires_at: SystemTime::now() - Duration::from_secs(1),
            });
        }

        // Should fail to refresh
        let result = cache.refresh_session(&token_id).await;
        assert!(result.is_err());
    }
}
