use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

#[derive(Clone)]
pub struct NonceCache {
    nonces: Arc<RwLock<HashMap<String, NonceInfo>>>,
}

struct NonceInfo {
    player_id: i64,
    created_at: SystemTime,
    used: bool,
}

impl NonceCache {
    pub fn new() -> Self {
        Self {
            nonces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_nonce(&self, player_id: i64) -> String {
        let nonce = generate_secure_random_string(32);
        let mut nonces = self.nonces.write().await;
        nonces.insert(nonce.clone(), NonceInfo {
            player_id,
            created_at: SystemTime::now(),
            used: false,
        });
        nonce
    }

    pub async fn verify_and_consume(&self, nonce: &str, player_id: i64) -> Result<(), String> {
        let mut nonces = self.nonces.write().await;

        match nonces.entry(nonce.to_string()) {
            Entry::Occupied(mut e) => {
                let info = e.get_mut();

                if info.used {
                    return Err("Nonce already used".to_string());
                }

                if info.player_id != player_id {
                    return Err("Wrong player".to_string());
                }

                if info.created_at.elapsed().unwrap_or(Duration::from_secs(61)) > Duration::from_secs(60) {
                    return Err("Nonce expired".to_string());
                }

                info.used = true;
                Ok(())
            }
            Entry::Vacant(_) => Err("Invalid nonce".to_string())
        }
    }

    pub async fn cleanup_expired(&self) {
        let mut nonces = self.nonces.write().await;
        nonces.retain(|_, info| {
            info.created_at.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_secs(300)
        });
    }
}

fn generate_secure_random_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_nonce_single_use() {
        let cache = NonceCache::new();
        let nonce = cache.create_nonce(123).await;

        // First use should succeed
        assert!(cache.verify_and_consume(&nonce, 123).await.is_ok());

        // Second use should fail
        assert!(cache.verify_and_consume(&nonce, 123).await.is_err());
    }

    #[tokio::test]
    async fn test_nonce_wrong_player() {
        let cache = NonceCache::new();
        let nonce = cache.create_nonce(123).await;

        // Wrong player should fail
        assert!(cache.verify_and_consume(&nonce, 456).await.is_err());
    }

    #[tokio::test]
    async fn test_nonce_expiration() {
        let cache = NonceCache::new();

        // Manually insert an expired nonce
        let nonce = generate_secure_random_string(32);
        {
            let mut nonces = cache.nonces.write().await;
            nonces.insert(nonce.clone(), NonceInfo {
                player_id: 123,
                created_at: SystemTime::now() - Duration::from_secs(61),
                used: false,
            });
        }

        // Should fail due to expiration
        assert!(cache.verify_and_consume(&nonce, 123).await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let cache = NonceCache::new();

        // Add an old nonce
        let old_nonce = generate_secure_random_string(32);
        {
            let mut nonces = cache.nonces.write().await;
            nonces.insert(old_nonce.clone(), NonceInfo {
                player_id: 123,
                created_at: SystemTime::now() - Duration::from_secs(301),
                used: false,
            });
        }

        // Add a recent nonce
        let recent_nonce = cache.create_nonce(456).await;

        // Run cleanup
        cache.cleanup_expired().await;

        // Old nonce should be gone
        {
            let nonces = cache.nonces.read().await;
            assert!(!nonces.contains_key(&old_nonce));
            assert!(nonces.contains_key(&recent_nonce));
        }
    }
}
