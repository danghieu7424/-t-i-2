use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, MySql}; // Thêm MySql để ép kiểu
use std::sync::Arc;
use base64::{engine::general_purpose, Engine as _};
use crate::AppState;

// 1. Phải có struct này để các hàm bên dưới nhận diện được Expense
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Expense {
    pub id: Option<i32>,
    pub user_id: i32,
    pub merchant: String,
    pub bill_date: Option<String>,
    pub amount: f64,
    pub category: String,
    pub is_warning: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DashboardData {
    pub month_labels: Vec<String>,
    pub dien_series: Vec<f64>,
    pub nuoc_series: Vec<f64>,
    pub nl_series: Vec<f64>,
    pub stats: Vec<StatItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatItem {
    pub title: String,
    pub amount: f64,
    pub percent: f64,
    pub slug: String,
}

pub fn expense_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_dashboard_data))
        .route("/upload", post(upload_invoice))
}

async fn get_dashboard_data(State(state): State<Arc<AppState>>) -> Json<DashboardData> {
    // Ép kiểu cụ thể cho sqlx để tránh lỗi "cannot infer type"
    let rows = sqlx::query(
        "SELECT 
            DATE_FORMAT(bill_date, '%m') as month, 
            category, 
            CAST(SUM(amount) AS DOUBLE) as total 
         FROM expenses 
         WHERE bill_date >= DATE_SUB(CURDATE(), INTERVAL 12 MONTH)
         GROUP BY month, category 
         ORDER BY bill_date ASC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut dien = vec![0.0; 12];
    let mut nuoc = vec![0.0; 12];
    let mut nl = vec![0.0; 12];
    let labels = vec!["T4","T5","T6","T7","T8","T9","T10","T11","T12","T1","T2","T3"];

    for row in rows {
        let m: String = row.get(0);
        let cat: String = row.get(1);
        let val: f64 = row.get(2);
        // Map tháng (1-12) vào index biểu đồ (T4 là index 0)
        let month_num = m.parse::<usize>().unwrap_or(1);
        let idx = if month_num >= 4 { month_num - 4 } else { month_num + 8 };
        
        if idx < 12 {
            match cat.as_str() {
                "Điện" => dien[idx] = val,
                "Nước" => nuoc[idx] = val,
                "Nguyên liệu" => nl[idx] = val,
                _ => {}
            }
        }
    }

    let calc_pct = |curr: f64, prev: f64| {
        if prev == 0.0 { if curr > 0.0 { 100.0 } else { 0.0 } }
        else { ((curr - prev) / prev) * 100.0 }
    };

    let stats = vec![
        StatItem { title: "Tiền Điện".into(), amount: dien[11], percent: calc_pct(dien[11], dien[10]), slug: "dien".into() },
        StatItem { title: "Tiền Nước".into(), amount: nuoc[11], percent: calc_pct(nuoc[11], nuoc[10]), slug: "nuoc".into() },
        StatItem { title: "Nguyên liệu".into(), amount: nl[11], percent: calc_pct(nl[11], nl[10]), slug: "nguyen-lieu".into() },
    ];

    Json(DashboardData { 
        month_labels: labels.into_iter().map(|s| s.into()).collect(), 
        dien_series: dien, 
        nuoc_series: nuoc, 
        nl_series: nl, 
        stats 
    })
}

async fn upload_invoice(State(state): State<Arc<AppState>>, mut multipart: Multipart) -> Json<Expense> {
    let mut image_data = Vec::new();
    let mut user_id = 1;

    // Sửa lỗi infer type cho multipart
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            image_data = field.bytes().await.unwrap_or_default().to_vec();
        } else if name == "user_id" {
            user_id = field.text().await.unwrap_or_default().parse().unwrap_or(1);
        }
    }

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={}", api_key);
    
    let base64_image = general_purpose::STANDARD.encode(&image_data);
    let client = reqwest::Client::new();
    let prompt = "Bạn là chuyên gia OCR hóa đơn tại Việt Nam. 
    Hãy đọc ảnh này và trích xuất thông tin. 
    Yêu cầu QUAN TRỌNG:
    1. 'merchant': Tên công ty/đơn vị bán hàng (Ví dụ: 'TỔNG CÔNG TY ĐIỆN LỰC MIỀN BẮC').
    2. 'bill_date': Tìm ngày lập hóa đơn, chuyển về YYYY-MM-DD.
    3. 'amount': Tìm con số sau chữ 'TỔNG TIỀN THANH TOÁN'. Phải lấy số cuối cùng, bỏ chữ 'VNĐ', bỏ dấu phẩy/chấm phân tách. Trả về kiểu số nguyên.
    4. 'category': Nếu tên merchant chứa 'ĐIỆN LỰC' -> 'Điện'. Nếu chứa 'NƯỚC' -> 'Nước'. 

    Chỉ trả về DUY NHẤT JSON. Không giải thích. Nếu không tìm thấy, không được để trống mà phải đoán dựa trên ngữ cảnh ảnh.";

    let payload = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}, {"inline_data": {"mime_type": "image/jpeg", "data": base64_image}}]}]
    });

    let res = client.post(api_url).json(&payload).send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let ai_text = res["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("");
    let clean_json = ai_text.trim_start_matches("```json").trim_end_matches("```").trim();
    let parsed: serde_json::Value = serde_json::from_str(clean_json).unwrap_or_default();

    let m = parsed["merchant"].as_str().unwrap_or("Không rõ").to_string();
    let d = parsed["bill_date"].as_str().unwrap_or("2026-03-20").to_string();
    let a = parsed["amount"].as_f64().unwrap_or(0.0);
    let c = parsed["category"].as_str().unwrap_or("Khác").to_string();

    let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category = ?")
        .bind(&c).fetch_one(&state.db).await.unwrap_or(0.0);
    let warn = if avg_cost > 0.0 { a > (avg_cost * 1.2) } else { false };

    let result = sqlx::query("INSERT INTO expenses (user_id, merchant, bill_date, amount, category, is_warning) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(user_id).bind(&m).bind(&d).bind(a).bind(&c).bind(warn)
        .execute(&state.db).await.unwrap();

    Json(Expense { 
        id: Some(result.last_insert_id() as i32), 
        user_id, 
        merchant: m, 
        bill_date: Some(d), 
        amount: a, 
        category: c, 
        is_warning: warn 
    })
}