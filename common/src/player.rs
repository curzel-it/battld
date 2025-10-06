use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: i64,
    pub public_key_hint: String,
    pub public_key: String,
    pub name: String,
    pub score: i64,
}
