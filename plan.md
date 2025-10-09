# Authentication Improvement Plan

## Current Problems

### The Issue
Our current authentication system (`common/src/auth.rs`) uses **time-based deterministic challenges**:

```rust
pub fn global_seed() -> u64 {
    unix_time / 3600  // Changes every hour
}

pub fn not_so_secret() -> (String, u64) {
    let seed = global_seed();
    let mut rng = StdRng::seed_from_u64(seed);
    // Generate 8-char "random" string
}
```

### Why This Is Weak

1. **Predictable**: Only 8,760 possible challenge strings per year (one per hour)
2. **Deterministic**: Same seed = same "random" string for both client and server
3. **Replay attacks**: A captured signature is valid for up to 1 hour
4. **No revocation**: Can't invalidate compromised credentials
5. **No logout**: No way to end a session early

### Attack Scenario
```
1. Attacker intercepts your auth packet at 2:05 PM
2. Challenge string is valid until 3:00 PM
3. Attacker replays your packet anytime before 3:00 PM
4. Server accepts it - they're authenticated as you
```

---

## Proposed Solution: Server-Issued Nonces

Keep the SSH key-based authentication (it's perfect for terminal usage!) but fix the challenge mechanism.

### Architecture

```
┌────────────┐                                    ┌────────────┐
│   Client   │                                    │   Server   │
└────────────┘                                    └────────────┘
       │                                                 │
       │  1. Request challenge (with player_id)         │
       ├────────────────────────────────────────────────>│
       │                                                 │
       │                                                 │ Generate random nonce
       │                                                 │ Store in Redis/Memory
       │                                                 │ Set 60s expiration
       │  2. Challenge response (nonce + timestamp)     │
       │<────────────────────────────────────────────────┤
       │                                                 │
       │  Sign nonce with private key                   │
       │                                                 │
       │  3. Send signed nonce                          │
       ├────────────────────────────────────────────────>│
       │                                                 │
       │                                                 │ Verify signature
       │                                                 │ Check nonce exists & not used
       │                                                 │ Mark nonce as used
       │                                                 │ Issue session token
       │  4. Session token (valid 24h)                  │
       │<────────────────────────────────────────────────┤
       │                                                 │
       │  5. All requests use session token             │
       ├────────────────────────────────────────────────>│
       │                                                 │
```

### Key Changes

#### 1. Server-Side Nonce Storage
```rust
// In-memory or Redis cache
pub struct NonceCache {
    nonces: RwLock<HashMap<String, NonceInfo>>,
}

struct NonceInfo {
    player_id: i64,
    created_at: SystemTime,
    used: bool,
}

impl NonceCache {
    pub fn create_nonce(&self, player_id: i64) -> String {
        let nonce = generate_secure_random_string(32); // crypto random
        self.nonces.write().insert(nonce.clone(), NonceInfo {
            player_id,
            created_at: SystemTime::now(),
            used: false,
        });
        nonce
    }

    pub fn verify_and_consume(&self, nonce: &str, player_id: i64) -> Result<(), AuthError> {
        let mut nonces = self.nonces.write();
        let info = nonces.get_mut(nonce).ok_or(AuthError::InvalidNonce)?;

        if info.used {
            return Err(AuthError::NonceAlreadyUsed);
        }
        if info.player_id != player_id {
            return Err(AuthError::WrongPlayer);
        }
        if info.created_at.elapsed()? > Duration::from_secs(60) {
            return Err(AuthError::NonceExpired);
        }

        info.used = true; // Prevent replay
        Ok(())
    }
}
```

#### 2. Session Token System
```rust
// After successful nonce verification, issue a session token
pub struct SessionToken {
    token_id: String,        // Random UUID
    player_id: i64,
    issued_at: SystemTime,
    expires_at: SystemTime,  // 24 hours from issue
}

pub struct SessionCache {
    sessions: RwLock<HashMap<String, SessionToken>>,
}

impl SessionCache {
    pub fn create_session(&self, player_id: i64) -> String {
        let token_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        self.sessions.write().insert(token_id.clone(), SessionToken {
            token_id: token_id.clone(),
            player_id,
            issued_at: now,
            expires_at: now + Duration::from_secs(86400), // 24h
        });

        token_id
    }

    pub fn verify_session(&self, token_id: &str) -> Result<i64, AuthError> {
        let sessions = self.sessions.read();
        let session = sessions.get(token_id).ok_or(AuthError::InvalidSession)?;

        if SystemTime::now() > session.expires_at {
            return Err(AuthError::SessionExpired);
        }

        Ok(session.player_id)
    }

    pub fn revoke_session(&self, token_id: &str) {
        self.sessions.write().remove(token_id);
    }
}
```

#### 3. New Auth Flow

**Initial Authentication:**
```rust
// 1. Client requests challenge
POST /auth/challenge
{ "player_id": 123, "public_key_hint": "abc..." }

// 2. Server responds with nonce
{ "nonce": "3f8a9c...", "expires_in": 60 }

// 3. Client signs nonce with private key
let signature = sign_with_private_key(&nonce);

POST /auth/verify
{
    "player_id": 123,
    "nonce": "3f8a9c...",
    "signature": "base64_signature..."
}

// 4. Server verifies and issues session token
{
    "session_token": "uuid-v4-token",
    "expires_at": "2024-10-10T12:00:00Z",
    "player": { ... }
}

// 5. Client uses session token for all subsequent requests
GET /matches/active
Headers: { "Authorization": "Bearer uuid-v4-token" }
```

**WebSocket Authentication:**
```rust
// Instead of signing hourly challenge, use session token
ClientMessage::Authenticate {
    session_token: "uuid-v4-token"
}

// Or allow re-signing with nonce for WebSocket connections
ClientMessage::AuthenticateWithNonce {
    player_id: 123,
    nonce: "requested_earlier",
    signature: "base64_sig..."
}
```

---

## Implementation Steps - Detailed File Changes

### Phase 1: Dependencies & Core Infrastructure

#### 1.1 Update `Cargo.toml` files

**`server/Cargo.toml`** - Add dependencies:
```toml
[dependencies]
# ... existing dependencies ...
uuid = { version = "1.0", features = ["v4", "serde"] }
```

**`common/Cargo.toml`** - Add for shared types:
```toml
[dependencies]
# ... existing dependencies ...
uuid = { version = "1.0", features = ["v4", "serde"] }
```

#### 1.2 Create `server/src/nonce_cache.rs` (NEW FILE - ~150 lines)

```rust
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

pub struct NonceCache {
    nonces: Arc<RwLock<HashMap<String, NonceInfo>>>,
}

struct NonceInfo {
    player_id: i64,
    created_at: SystemTime,
    used: bool,
}

impl NonceCache {
    pub fn new() -> Self { /* ... */ }
    pub fn create_nonce(&self, player_id: i64) -> String { /* ... */ }

    // IMPORTANT: Use Entry API for atomic check-and-set to prevent race conditions
    pub fn verify_and_consume(&self, nonce: &str, player_id: i64) -> Result<(), String> {
        let mut nonces = self.nonces.write();
        match nonces.entry(nonce.to_string()) {
            Entry::Occupied(mut e) => {
                let info = e.get_mut();
                if info.used {
                    return Err("Nonce already used".to_string());
                }
                if info.player_id != player_id {
                    return Err("Wrong player".to_string());
                }
                if info.created_at.elapsed().unwrap() > Duration::from_secs(60) {
                    return Err("Nonce expired".to_string());
                }
                info.used = true; // Atomic with the check above
                Ok(())
            }
            Entry::Vacant(_) => Err("Invalid nonce".to_string())
        }
    }

    pub async fn cleanup_expired(&self) { /* ... */ }
}

fn generate_secure_random_string(len: usize) -> String {
    // Generates 32-char alphanumeric string (~190 bits of entropy)
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
```

**Location**: `server/src/nonce_cache.rs`
**Why**: Manages server-side nonces with crypto-random generation and single-use enforcement
**Note**: In-memory storage (sessions lost on restart - acceptable for this project)

#### 1.3 Create `server/src/session_cache.rs` (NEW FILE - ~200 lines)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct SessionCache {
    sessions: Arc<RwLock<HashMap<String, SessionToken>>>,
}

pub struct SessionToken {
    pub token_id: String,
    pub player_id: i64,
    pub issued_at: SystemTime,
    pub expires_at: SystemTime,
}

impl SessionCache {
    pub fn new() -> Self { /* ... */ }
    pub fn create_session(&self, player_id: i64) -> String { /* ... */ }
    pub fn verify_session(&self, token_id: &str) -> Result<i64, String> { /* ... */ }
    pub fn revoke_session(&self, token_id: &str) { /* ... */ }
    pub fn revoke_all_for_player(&self, player_id: i64) { /* ... */ }
    pub async fn cleanup_expired(&self) { /* ... */ }
}
```

**Location**: `server/src/session_cache.rs`
**Why**: Manages session tokens with 24h expiration and revocation support
**Note**: In-memory storage (sessions lost on restart - acceptable for this project)

#### 1.4 Update `server/src/main.rs`

**Add modules** (after line 23):
```rust
mod nonce_cache;
mod session_cache;
```

**Update AppState** (replace lines 31-35):
```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub registry: Arc<ConnectionRegistry>,
    pub nonce_cache: Arc<nonce_cache::NonceCache>,
    pub session_cache: Arc<session_cache::SessionCache>,
}
```

**Initialize caches** (around line 91, before AppState creation):
```rust
let nonce_cache = Arc::new(nonce_cache::NonceCache::new());
let session_cache = Arc::new(session_cache::SessionCache::new());

// Start cleanup tasks
let nonce_cache_clone = nonce_cache.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        nonce_cache_clone.cleanup_expired().await;
    }
});

let session_cache_clone = session_cache.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        session_cache_clone.cleanup_expired().await;
    }
});

let state = AppState {
    db: Arc::new(db),
    registry: Arc::new(ConnectionRegistry::new()),
    nonce_cache,
    session_cache,
};
```

**Lines affected**: Lines 12-23 (add modules), lines 31-35 (AppState), lines 91-94 (initialization)

---

### Phase 2: New Auth Endpoints

#### 2.1 Update `common/src/api.rs` - Add new message types

**Add after existing types** (around line 50):
```rust
// New auth flow requests/responses
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChallengeRequest {
    pub player_id: i64,
    pub public_key_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChallengeResponse {
    pub nonce: String,
    pub expires_in: u64, // seconds
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyRequest {
    pub player_id: i64,
    pub nonce: String,
    pub signature: String, // base64 encoded
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub session_token: String,
    pub expires_at: String, // ISO 8601 timestamp
    pub player: Player,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogoutRequest {
    pub session_token: String,
}
```

**Location**: `common/src/api.rs`
**Lines affected**: Add ~50 new lines after existing types
**Why**: Shared types between client and server for new auth flow

#### 2.2 Create `server/src/auth_endpoints.rs` (NEW FILE - ~200 lines)

```rust
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use crate::{AppState, repository};
use battld_common::*;

// POST /auth/challenge
pub async fn request_challenge(
    State(state): State<AppState>,
    Json(request): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    // Verify player exists and public_key_hint matches
    let player = repository::fetch_player(&state.db, request.player_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    if player.public_key_hint != request.public_key_hint {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Generate and store nonce
    let nonce = state.nonce_cache.create_nonce(request.player_id);

    Ok(Json(ChallengeResponse {
        nonce,
        expires_in: 60,
    }))
}

// POST /auth/verify
pub async fn verify_challenge(
    State(state): State<AppState>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    // Verify nonce exists and not used
    state.nonce_cache
        .verify_and_consume(&request.nonce, request.player_id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get player and verify signature
    let player = repository::fetch_player(&state.db, request.player_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let player_record = state.db.get_player_by_id(request.player_id)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify signature against nonce (not time-based string!)
    if !crate::auth::verify_signature_for_nonce(&player_record, &request.signature, &request.nonce)
        .unwrap_or(false)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create session token
    let session_token = state.session_cache.create_session(request.player_id);

    let expires_at = (SystemTime::now() + Duration::from_secs(86400))
        .duration_since(UNIX_EPOCH)
        .unwrap();

    Ok(Json(AuthResponse {
        session_token,
        expires_at: format!("{}", expires_at.as_secs()),
        player,
    }))
}

// POST /auth/logout
pub async fn logout(
    State(state): State<AppState>,
    Json(request): Json<LogoutRequest>,
) -> Result<StatusCode, StatusCode> {
    state.session_cache.revoke_session(&request.session_token);
    Ok(StatusCode::OK)
}
```

**Location**: `server/src/auth_endpoints.rs` (new file)
**Why**: Handles new 2-step auth flow: challenge request → verify signature → issue session

#### 2.3 Update `server/src/auth.rs`

**Add new function** (after line 100):
```rust
// New function: verify signature against arbitrary nonce (not time-based)
pub fn verify_signature_for_nonce(
    player: &crate::database::PlayerRecord,
    encrypted_token: &str,
    nonce: &str, // The nonce we want to verify against
) -> Result<bool, Box<dyn std::error::Error>> {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign};
    use rsa::sha2::Sha256;
    use base64::{Engine as _, engine::general_purpose};
    use sha2::Digest;

    // Decode the player's public key
    let public_key = match RsaPublicKey::from_pkcs1_pem(&player.public_key) {
        Ok(key) => key,
        Err(_) => RsaPublicKey::from_public_key_pem(&player.public_key)?,
    };

    // Decode the base64 signature
    let signature = general_purpose::STANDARD.decode(encrypted_token)?;

    // Hash the nonce (not time-based string!)
    let mut hasher = Sha256::new();
    hasher.update(nonce.as_bytes());
    let hashed = hasher.finalize();

    // Verify the signature using PKCS1v15 with SHA256
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let verification_result = public_key.verify(padding, &hashed, &signature);

    Ok(verification_result.is_ok())
}
```

**Update authenticate_request** (modify around line 44-66):
```rust
pub async fn authenticate_request(
    db: &Database,
    session_cache: &session_cache::SessionCache, // ADD THIS PARAM
    headers: &HeaderMap,
) -> Result<i64, StatusCode> {
    // Extract Authorization header with format "Bearer <session_token>"
    let session_token = headers.get(HEADER_AUTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify session token (NEW WAY - no more time-based verification!)
    session_cache
        .verify_session(session_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}
```

**Lines affected**:
- Add function after line 100 (~30 lines)
- Modify `authenticate_request` (lines 44-66)
- Keep old functions for backward compatibility initially

#### 2.4 Update `server/src/main.rs` - Add new routes

**Add module** (after line 11):
```rust
mod auth_endpoints;
```

**Update router** (modify around lines 99-108):
```rust
let api_routes = Router::new()
    // NEW AUTH ENDPOINTS
    .route("/auth/challenge", post(auth_endpoints::request_challenge))
    .route("/auth/verify", post(auth_endpoints::verify_challenge))
    .route("/auth/logout", post(auth_endpoints::logout))
    // EXISTING ROUTES
    .route("/player", post(auth::create_player))
    .route("/player", get(players::get_player))
    .route("/player/current", get(players::post_player))
    .route("/player/:id", get(players::get_player_by_id))
    .route("/matches/active", get(players::get_active_matches))
    .route("/stats", get(stats::get_stats))
    .route("/leaderboard", get(stats::get_leaderboard))
    .layer(rate_limit::create_rate_limiter()) // Applied to ALL routes including /auth/*
    .with_state(state.clone());
```

**Lines affected**: Line 11 (add module), lines 99-108 (add routes)
**Note**: Existing rate limiter already applies to all API routes including new auth endpoints

#### 2.5 Update endpoints that call `authenticate_request`

**Files to modify**:
- `server/src/players.rs` - Lines ~15, ~30, ~45 (all functions)
- `server/src/stats.rs` - Lines ~10, ~25 (all functions)

**Change pattern**:
```rust
// OLD:
let player_id = authenticate_request(&state.db, &headers).await?;

// NEW:
let player_id = authenticate_request(&state.db, &state.session_cache, &headers).await?;
```

**Affected functions**:
- `get_player()` in players.rs:~15
- `post_player()` in players.rs:~30
- `get_player_by_id()` in players.rs:~45
- `get_active_matches()` in players.rs:~60
- `get_stats()` in stats.rs:~10
- `get_leaderboard()` in stats.rs:~25

---

### Phase 3: Client-Side Changes

#### 3.1 Update `client/src/auth.rs` - New auth flow

**Add helper function** (around line 100):
```rust
// Request challenge from server
async fn request_auth_challenge(
    config: &Config,
    player_id: i64,
    public_key_hint: &str,
) -> Result<ChallengeResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/auth/challenge", config.server_url);

    let response = client
        .post(&url)
        .json(&ChallengeRequest {
            player_id,
            public_key_hint: public_key_hint.to_string(),
        })
        .send()
        .await?;

    Ok(response.json().await?)
}

// Verify challenge and get session token
async fn verify_auth_challenge(
    config: &Config,
    player_id: i64,
    nonce: &str,
    private_key: &RsaPrivateKey,
) -> Result<AuthResponse, Box<dyn std::error::Error>> {
    // Sign the nonce (not time-based string!)
    let signature = sign_data(nonce.as_bytes(), private_key)?;

    let client = reqwest::Client::new();
    let url = format!("{}/auth/verify", config.server_url);

    let response = client
        .post(&url)
        .json(&VerifyRequest {
            player_id,
            nonce: nonce.to_string(),
            signature,
        })
        .send()
        .await?;

    Ok(response.json().await?)
}
```

**Modify `try_auto_login`** (around line 50):
```rust
pub async fn try_auto_login(session: &mut SessionState) -> Result<bool, Box<dyn std::error::Error>> {
    let config = &session.config;

    // Check if we have stored credentials
    let (player_id, public_key_hint) = match load_stored_credentials()? {
        Some(creds) => creds,
        None => return Ok(false),
    };

    // Load private key
    let private_key = load_private_key()?;

    // NEW 2-STEP FLOW:
    // 1. Request challenge
    let challenge_response = request_auth_challenge(
        config,
        player_id,
        &public_key_hint
    ).await?;

    // 2. Sign nonce and verify
    let auth_response = verify_auth_challenge(
        config,
        player_id,
        &challenge_response.nonce,
        &private_key,
    ).await?;

    // Store session token
    session.player_id = Some(auth_response.player.id);
    session.player_name = Some(auth_response.player.name.clone());
    session.session_token = Some(auth_response.session_token.clone());

    // Connect WebSocket with session token
    let ws_client = connect_websocket(config, &auth_response.session_token).await?;
    session.ws_client = Some(ws_client);

    Ok(true)
}
```

**Lines affected**:
- Add helper functions after line 100 (~80 lines)
- Modify `try_auto_login` (lines 50-80, complete rewrite)

#### 3.2 Update `client/src/state.rs`

**Add field to SessionState** (around line 10):
```rust
pub struct SessionState {
    pub config: Config,
    pub player_id: Option<i64>,
    pub player_name: Option<String>,
    pub session_token: Option<String>, // ADD THIS
    pub ws_client: Option<WebSocketClient>,
}
```

**Update initialization** (around line 20):
```rust
pub fn new_with_config(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    Ok(SessionState {
        config: Config::load(config_path)?,
        player_id: None,
        player_name: None,
        session_token: None, // ADD THIS
        ws_client: None,
    })
}
```

**Lines affected**: Line 10 (add field), line 20 (initialization)

#### 3.3 Update `client/src/websocket.rs`

**Modify connection** (around line 30):
```rust
pub async fn connect_websocket(
    config: &Config,
    session_token: &str, // ADD THIS PARAM
) -> Result<WebSocketClient, Box<dyn std::error::Error>> {
    let url = format!("{}/ws", config.server_url.replace("http", "ws"));
    let (ws_stream, _) = connect_async(&url).await?;

    let client = WebSocketClient::new(ws_stream);

    // Authenticate with session token (not signature!)
    client.send(ClientMessage::Authenticate {
        token: session_token.to_string(), // Send session token
    })?;

    // Wait for auth confirmation
    // ...

    Ok(client)
}
```

**Lines affected**: Lines 30-50 (modify connection and auth)

#### 3.4 Update `common/src/api.rs` - WebSocket messages

**Modify ClientMessage** (around line 80):
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Authenticate { token: String }, // CHANGED: now accepts session_token directly
    Ping,
    JoinMatchmaking { game_type: GameType },
    ResumeMatch,
    MakeMove { move_data: serde_json::Value },
}
```

**Lines affected**: Line ~85 (change Authenticate variant)

#### 3.5 Update `server/src/websocket.rs`

**Modify authenticate_token** (around line 258):
```rust
async fn authenticate_token(
    session_cache: &session_cache::SessionCache, // ADD PARAM
    token: &str
) -> Result<i64, String> {
    // Token is now a session token (not "player_id:signature")
    session_cache
        .verify_session(token)
        .map_err(|e| format!("Invalid session: {}", e))
}
```

**Update handler** (around line 176):
```rust
match client_msg {
    ClientMessage::Authenticate { token } => {
        // Authenticate using session token
        match authenticate_token(&state.session_cache, &token).await { // PASS session_cache
            Ok(pid) => {
                player_id = Some(pid);
                // ... rest of auth logic
            }
            Err(e) => {
                let response = ServerMessage::AuthFailed { reason: e };
                let _ = tx.send(response);
                break;
            }
        }
    }
    // ... rest of match arms
}
```

**Lines affected**: Lines 258-280 (authenticate_token), lines 176-208 (handler)

---

### Phase 4: Deprecate Old Auth

#### 4.1 Update `common/src/auth.rs`

**Mark as deprecated** (line 1):
```rust
// DEPRECATED: This time-based auth will be removed in v3.0.0
// Use session tokens via /auth/challenge and /auth/verify instead

#[deprecated(note = "Use session token authentication instead")]
pub fn global_seed() -> u64 { /* ... */ }

#[deprecated(note = "Use session token authentication instead")]
pub fn not_so_secret() -> (String, u64) { /* ... */ }
```

**Lines affected**: Lines 1, 8, 16 (add deprecation warnings)

#### 4.2 Optional: Keep old auth for backward compatibility

In `server/src/auth.rs`, keep old `authenticate_request` as `authenticate_request_legacy`:
```rust
#[deprecated]
pub async fn authenticate_request_legacy(
    db: &Database,
    headers: &HeaderMap,
) -> Result<i64, StatusCode> {
    // OLD IMPLEMENTATION - still works for old clients
    // ... existing code ...
}
```

---

### Phase 5: Testing

#### 5.1 Create `server/src/nonce_cache_tests.rs` (NEW FILE)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nonce_single_use() { /* ... */ }

    #[tokio::test]
    async fn test_nonce_expiration() { /* ... */ }

    #[tokio::test]
    async fn test_nonce_wrong_player() { /* ... */ }
}
```

#### 5.2 Create `server/src/session_cache_tests.rs` (NEW FILE)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_expiration() { /* ... */ }

    #[tokio::test]
    async fn test_session_revocation() { /* ... */ }

    #[tokio::test]
    async fn test_revoke_all_for_player() { /* ... */ }
}
```

---

## Summary of File Changes

| File | Type | Lines Changed | Description |
|------|------|---------------|-------------|
| `server/Cargo.toml` | Modify | +2 | Add uuid dependency |
| `common/Cargo.toml` | Modify | +1 | Add uuid dependency |
| `server/src/nonce_cache.rs` | **NEW** | +150 | Nonce management |
| `server/src/session_cache.rs` | **NEW** | +200 | Session token management |
| `server/src/auth_endpoints.rs` | **NEW** | +200 | New auth flow endpoints |
| `server/src/main.rs` | Modify | +30 | Add caches, routes, cleanup tasks |
| `server/src/auth.rs` | Modify | +50 | Add nonce verification function |
| `server/src/players.rs` | Modify | ~20 | Update authenticate_request calls |
| `server/src/stats.rs` | Modify | ~10 | Update authenticate_request calls |
| `server/src/websocket.rs` | Modify | ~40 | Use session tokens |
| `common/src/api.rs` | Modify | +60 | Add new auth types |
| `client/src/auth.rs` | Modify | +100 | New 2-step auth flow |
| `client/src/state.rs` | Modify | +5 | Add session_token field |
| `client/src/websocket.rs` | Modify | ~20 | Use session tokens |
| `common/src/auth.rs` | Modify | +10 | Add deprecation warnings |

**Total**: 3 new files, 12 modified files, ~900 lines of code

---

## Migration Strategy

This is a **breaking change** - old clients won't work with new server.

### Option A: Big Bang (Recommended for small project)
1. Release v2.0.0 with new auth
2. Announce breaking change
3. Users must update client and server together

### Option B: Versioned API (Overkill for this project)
1. Keep old auth at `/v1/*` endpoints
2. New auth at `/v2/*` endpoints
3. Deprecate v1 after 3 months

---

## Security Improvements Achieved

| Issue | Before | After |
|-------|--------|-------|
| **Predictability** | 8,760 possible values/year | Cryptographically random nonces |
| **Replay attacks** | Valid for 1 hour | Single-use nonces (60s expiration) |
| **Session control** | No logout mechanism | Can revoke sessions |
| **Token lifetime** | Forever (until next hour) | 24h sessions, renewable |
| **Audit trail** | None | Can track all active sessions |
| **Compromised keys** | No way to invalidate | Revoke all sessions for that player |

### Still Using SSH Keys ✅
- Client still uses private key to sign challenges
- Server still verifies with stored public key
- No passwords, no third-party auth
- Pure terminal-friendly cryptography

### What We're NOT Changing
- ✅ SSH key pairs for authentication (kept!)
- ✅ Terminal-only client (kept!)
- ✅ No passwords (kept!)
- ✅ No web UI for auth (kept!)
- ✅ Local key storage (kept!)

---

## Code Size Estimate

- `server/src/nonce_cache.rs`: ~150 lines
- `server/src/session_cache.rs`: ~200 lines
- `server/src/auth.rs`: Modify ~100 lines
- `client/src/auth.rs`: Modify ~150 lines
- New endpoints: ~100 lines
- Tests: ~300 lines

**Total: ~900 lines of new/modified code**

---

## Alternative: Hybrid Approach (Easier Migration)

Keep current system but add nonce-based as opt-in:

```rust
// Accept both old and new auth methods
match auth_header {
    "Bearer session-token-..." => verify_session_token(token),
    "Bearer signature-..." => verify_legacy_signature(signature), // Old way
}
```

This allows gradual migration without breaking existing clients.

---

## Design Decisions

1. **Storage**: ✅ In-memory (Arc<RwLock<HashMap>>)
   - Sessions lost on server restart - users simply re-authenticate
   - Acceptable tradeoff: simplicity vs persistence
   - No external dependencies (Redis, etc.)

2. **Session duration**: ✅ 24 hours
   - Reasonable balance between security and UX
   - Can be adjusted later if needed

3. **Migration**: ✅ Breaking change (v2.0.0)
   - Small user base makes big-bang acceptable
   - Clear version bump signals breaking change

4. **Nonce cleanup**: ✅ Background task (every 60s)
   - Automatic cleanup of expired nonces
   - Session cleanup every hour (3600s)

---

## Conclusion

The new system maintains the **SSH-key terminal authentication** that makes Battld unique, while fixing the security weaknesses of deterministic time-based challenges.

Users will still:
- Generate SSH keys locally
- Sign challenges with their private key
- Never use passwords
- Authenticate purely in the terminal

But now with:
- ✅ Cryptographically secure random challenges
- ✅ Single-use nonces (no replay attacks)
- ✅ Session management and revocation
- ✅ Proper logout capability
- ✅ Atomic nonce verification (no race conditions)
- ✅ Rate limiting on all auth endpoints
- ✅ Simple in-memory storage (acceptable session loss on restart)

This is production-ready security while keeping the terminal hacker aesthetic! 🔐

---

## Production Readiness Checklist

✅ **Security**
- Cryptographically random nonces (32 chars, ~190 bits entropy)
- Single-use nonces with 60s expiration
- Atomic check-and-set prevents race conditions
- 24h session tokens with revocation support
- SSH key-based authentication maintained

✅ **Rate Limiting**
- Existing RATE_LIMIT_RPS applies to all endpoints including /auth/*
- No additional rate limiting needed

✅ **Storage**
- In-memory (Arc<RwLock<HashMap>>)
- Sessions lost on restart - users re-authenticate (acceptable)
- No external dependencies required

✅ **HTTP/HTTPS**
- Works in both HTTP (local dev) and HTTPS (production)
- No special TLS handling required

✅ **Migration**
- Breaking change in v2.0.0
- Clear version bump signals incompatibility
- Users update client + server together

This plan is **comprehensive and ready for implementation**.
