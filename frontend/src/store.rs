// src/store.rs
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: i32,
    pub name: String,
    pub picture: String,
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct GlobalState {
    pub domain: String,
    pub user: RwSignal<Option<UserInfo>>,
    pub client_sk_bytes: [u8; 32],
}

pub fn init_global_state() -> GlobalState {
    let sk = p256::SecretKey::random(&mut rand::rngs::OsRng);
    GlobalState {
        // domain: "".to_string(), // Cổng Backend của bạn
        // domain: "http://localhost:5000".to_string(), // Cổng Backend của bạn
        domain: "https://des.dh74.io.vn".to_string(), // Cổng Backend của bạn
        user: create_rw_signal(None),
        client_sk_bytes: sk.to_bytes().into(),
    }
}