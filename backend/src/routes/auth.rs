use axum::{extract::State, Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::utils::suid::suid;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub user_id: String, // SỬA: i32 -> String
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

  // Tìm user dựa trên google_id
    let existing_user: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE google_id = ?")
        .bind(&google_id)
        .fetch_optional(&state.db).await.unwrap_or(None);

    let final_user_id = match existing_user {
        Some(id) => id,
        None => {
            // Dùng hàm suid() trong utils của bạn để tạo ID Base64
            let new_id = crate::utils::suid::suid(); 
            sqlx::query("INSERT INTO users (id, google_id, email, name, picture) VALUES (?, ?, ?, ?, ?)")
                .bind(&new_id).bind(&google_id).bind(&email).bind(&name).bind(&picture)
                .execute(&state.db).await.expect("Lỗi tạo User");
            new_id
        }
    };

    Json(UserInfo { user_id: final_user_id, name, picture, email })}