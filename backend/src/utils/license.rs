// src/routes/video/license.rs

use axum::{extract::{State, Json}, response::Json as AxumJson}; // <-- SỬA: Đã thêm State và Json
use serde::{Serialize, Deserialize};
use ring::{agreement, rand, error, aead}; // <-- SỬA: Đã thêm agreement, error, và rand
use rand::SystemRandom;
use base64;
use crate::AppState; // <-- SỬA: Đã thêm AppState
use std::convert::TryInto;
use sqlx::Row; 

// --- CÁC HÀM TIỆN ÍCH CẦN TÁCH RA ---

// Hàm Dẫn xuất Khóa Phiên (ECDH + HKDF)
fn derive_shared_key(shared_secret: agreement::SharedSecret<'_>) -> Result<Vec<u8>, error::Unspecified> {
    // ... [Dán lại hàm này] ...
    let salt = [0u8; 0]; 
    let info = LICENSE_INFO; // Dùng hằng số đã định nghĩa
    agreement::kdf::derive(
        &agreement::HKDF_SHA256, 
        shared_secret.as_ref(), // IKM
        &salt, 
        info, 
        16, // Kích thước K_sess (16 bytes cho AES-128)
    )
}

// Hàm xử lý chính: Cấp phép License
// Sửa: Lỗi E0412/E0531: Thêm State, Json, LicenseRequest, LicenseResponse
pub async fn issue_license(
    State(state): State<AppState>, 
    AxumJson(req): AxumJson<LicenseRequest>, // <-- SỬA: Dùng AxumJson để tránh xung đột tên
) -> Result<AxumJson<LicenseResponse>, axum::http::StatusCode> {

    // ... (Phần logic đã sửa) ...
    
    // 1. LẤY CONTENT KEY ($K_c$) TỪ DB (Không đổi)
    // ...

    // Khắc phục lỗi E0425 (client_pk_bytes không được định nghĩa)
    let client_pk_bytes = base64::decode(&req.client_pk)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let rng = SystemRandom::new(); // Khởi tạo RNG

    // 2. ECDH & HKDF: Tính toán K_sess
    // Thay vì dùng hằng số, ta dùng khóa từ AppState
    let server_sk = agreement::EphemeralPrivateKey::try_from(
        &agreement::ECDH_P256, 
        &state.server_sk_bytes
    ).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?; 
    
    // ...
    // Khắc phục lỗi E0425 (derive_shared_key không được định nghĩa)
    let k_sess = agreement::agree_ephemeral(
        server_sk, 
        &agreement::PublicKey::try_from(&agreement::ECDH_P256, &client_pk_bytes) // <-- client_pk_bytes đã được định nghĩa
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?,
        error::Unspecified,
        derive_shared_key // <-- Đã đặt hàm này trong file
    ).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    // 3. AES-GCM: Bọc Content Key ($K_c$) (Không đổi, đã thêm 'rng' và 'aes_key', v.v.)
    // ... Khắc phục lỗi E0425: payload và nonce_bytes
    
    let cipher_key: agreement::SharedSecret = k_sess.try_into().unwrap();
    let mut key_bytes = [0u8; 16];
    key_bytes.copy_from_slice(cipher_key.as_ref());

    let aes_key = ring::aead::UnboundKey::new(&ring::aead::AES_128_GCM, &key_bytes)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes);
    let mut sealing_key = ring::aead::SealingKey::new(aes_key, ring::aead::Nonce::assume_unique_for_key(nonce_bytes));

    let mut payload = kc_bytes.to_vec(); // kc_bytes được định nghĩa trong bước 1
    ring::aead::seal_in_place_append_tag(
        &sealing_key,
        ring::aead::Aad::empty(), 
        &mut payload,
    ).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 4. Trả về Response
    // Khắc phục lỗi E0425/E0422: payload, nonce_bytes, LicenseResponse
    Ok(AxumJson(LicenseResponse {
        encrypted_key: base64::encode(&payload),
        nonce: base64::encode(&nonce_bytes),
        server_pk: base64::encode(&state.server_pk_bytes),
    }))
}