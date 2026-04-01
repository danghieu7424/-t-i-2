use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub user_id: String, 
    pub token: String, // THÊM: Trả về JWT Token cho Frontend
    pub name: String,
    pub picture: String,
    pub email: String,
}

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new().route("/google", post(login_google))
}

// ==========================================
// 🛡️ BẢO MẬT: AXUM EXTRACTOR (NGƯỜI GÁC CỔNG)
// ==========================================
pub struct AuthUser {
    pub user_id: String, // Chỉ khi token hợp lệ, nó mới móc user_id thật ra đưa cho bạn
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Lấy Header "Authorization: Bearer <token>"
        let auth_header = parts.headers.get("Authorization")
            .and_then(|value| value.to_str().ok())
            .filter(|s| s.starts_with("Bearer "))
            .map(|s| s.trim_start_matches("Bearer ").to_string());

        // 2. Giải mã Token
        if let Some(token) = auth_header {
            if let Ok(token_data) = crate::utils::token::verify_auth_token(&token) {
                // Thành công: Trả về user_id chính chủ đã được mã hóa trong token
                return Ok(AuthUser { user_id: token_data.claims.sub });
            }
        }
        // Thất bại: Đuổi về ngay lập tức với mã 401
        Err((StatusCode::UNAUTHORIZED, "Token không hợp lệ, bị làm giả hoặc đã hết hạn!"))
    }
}

// ==========================================
// LOGIC ĐĂNG NHẬP TẠO TOKEN
// ==========================================
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

    let google_id = res["sub"].as_str().unwrap().to_string();
    let email = res["email"].as_str().unwrap().to_string();
    let name = res["name"].as_str().unwrap().to_string();
    let picture = res["picture"].as_str().unwrap().to_string();

    let existing_user: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE google_id = ?")
        .bind(&google_id).fetch_optional(&state.db).await.unwrap_or(None);

    let final_user_id = match existing_user {
        Some(id) => id,
        None => {
            let new_id = crate::utils::suid::suid(); 
            sqlx::query("INSERT INTO users (id, google_id, email, name, picture) VALUES (?, ?, ?, ?, ?)")
                .bind(&new_id).bind(&google_id).bind(&email).bind(&name).bind(&picture)
                .execute(&state.db).await.expect("Lỗi tạo User");
            new_id
        }
    };

    // TẠO TOKEN BẢO MẬT (Hạn 30 ngày = 2592000 giây)
    let jwt_token = crate::utils::token::create_auth_token(&final_user_id, 2592000).unwrap();

    Json(UserInfo { 
        user_id: final_user_id, 
        token: jwt_token, // Trả token về cho Frontend cất vào LocalStorage
        name, picture, email 
    })
}