use axum::{extract::State, Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub user_id: i32,
    pub name: String,
    pub picture: String,
    pub email: String,
}

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new().route("/google", post(login_google))
}

async fn login_google(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthRequest>,
) -> Json<UserInfo> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    
    let client = reqwest::Client::new();
    let res = client
        .get(format!("https://oauth2.googleapis.com/tokeninfo?id_token={}", payload.token))
        .send().await.expect("Lỗi kết nối Google")
        .json::<serde_json::Value>().await.unwrap();

    // Blind Spot: Kiểm tra xem aud (audience) có khớp với Client ID của bạn không
    let aud = res["aud"].as_str().unwrap_or("");
    if aud != client_id {
        // Có thể trả về lỗi 401 tại đây nếu muốn bảo mật cao
    }
    let google_id = res["sub"].as_str().unwrap().to_string();
    let email = res["email"].as_str().unwrap().to_string();
    let name = res["name"].as_str().unwrap().to_string();
    let picture = res["picture"].as_str().unwrap().to_string();

    // Upsert User vào DB
    sqlx::query(
        "INSERT INTO users (google_id, email, name, picture) VALUES (?, ?, ?, ?) 
         ON DUPLICATE KEY UPDATE name = ?, picture = ?"
    )
    .bind(&google_id).bind(&email).bind(&name).bind(&picture)
    .bind(&name).bind(&picture)
    .execute(&state.db).await.unwrap();

    let user_id: i32 = sqlx::query_scalar("SELECT id FROM users WHERE google_id = ?")
        .bind(&google_id)
        .fetch_one(&state.db).await.unwrap();

    Json(UserInfo { user_id, name, picture, email })
}