// src/utils/token.rs
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use jsonwebtoken::{EncodingKey, DecodingKey, Header, Validation, encode, decode, TokenData, errors::Error as JwtError};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoClaims {
    pub sub: String,       // file path relative (ví dụ: "videos/abc.mp4")
    pub exp: usize,        // epoch seconds
    pub ip: Option<String> // optional bound IP as string
}

// Thêm struct Claims dành riêng cho User
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String, // Sẽ chứa user_id thực sự
    pub exp: usize,  // Thời gian hết hạn
}

fn jwt_secret() -> String {
    env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env")
}

/// Tạo token với expiry_seconds và optional ip
pub fn create_signed_token(file_path: &str, expiry_seconds: i64, ip: Option<String>) -> Result<String, JwtError> {
    let secret = jwt_secret();
    let exp = (Utc::now() + Duration::seconds(expiry_seconds)).timestamp() as usize;
    let claims = VideoClaims {
        sub: file_path.to_string(),
        exp,
        ip,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

/// Verify token -> trả về claims nếu đúng
pub fn verify_token(token: &str) -> Result<TokenData<VideoClaims>, JwtError> {
    let secret = jwt_secret();
    let validation = Validation::default();
    decode::<VideoClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
}



/// Đúc chìa khóa (JWT) cho người dùng đăng nhập
pub fn create_auth_token(user_id: &str, expiry_seconds: i64) -> Result<String, JwtError> {
    let secret = jwt_secret();
    let exp = (Utc::now() + Duration::seconds(expiry_seconds)).timestamp() as usize;
    let claims = AuthClaims {
        sub: user_id.to_string(),
        exp,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

/// Xác thực chìa khóa (Kiểm tra xem token có bị làm giả hay hết hạn không)
pub fn verify_auth_token(token: &str) -> Result<TokenData<AuthClaims>, JwtError> {
    let secret = jwt_secret();
    decode::<AuthClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
}