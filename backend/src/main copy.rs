use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::Row;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use base64::{engine::general_purpose, Engine as _};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Expense {
    id: Option<i32>,
    merchant: String,
    bill_date: Option<String>, // Định dạng YYYY-MM-DD
    amount: f64,
    category: String,
    is_warning: bool,
}
struct AppState {
    db: MySqlPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    
    let database_url = "mysql://dh7424_tttn:tttn123@db.dh74.io.vn/dh7424_tttn";
    
    // Thêm log để biết đang chạy đến đâu
    println!("🔍 Đang kết nối tới MySQL...");

    let pool = match MySqlPool::connect(database_url).await {
        Ok(p) => {
            println!("✅ Kết nối MySQL thành công!");
            p
        },
        Err(e) => {
            eprintln!("❌ LỖI KẾT NỐI DB: {}. Kiểm tra MySQL đã bật chưa?", e);
            return Ok(()); // Trả về Ok để không hiện exit code 1 khó hiểu
        }
    };

    let shared_state = Arc::new(AppState { db: pool });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/expenses", get(get_expenses))
        .route("/api/upload", post(upload_invoice))
        .layer(cors)
        .with_state(shared_state);

    // Lưu ý: React của bạn đang gọi 3001, nên hãy chỉnh Backend thành 3001
    let addr = "127.0.0.1:3001";
    println!("🚀 Backend đang chạy tại http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Lấy danh sách chi phí (Dùng query thô để tránh lỗi Driver lúc build)
async fn get_expenses(State(state): State<Arc<AppState>>) -> Json<Vec<Expense>> {
    let rows = sqlx::query("SELECT id, merchant, amount, category, is_warning FROM expenses ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let expenses = rows.into_iter().map(|row| {
        Expense {
            id: Some(row.get(0)),
            merchant: row.get(1),
            bill_date: row.get(2),
            amount: row.get::<f64, _>(3),
            category: row.get(4),
            is_warning: row.get(5),
        }
    }).collect();

    Json(expenses)
}

async fn upload_invoice(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<Expense> {
    let mut image_data = Vec::new();

    // 1. Lấy dữ liệu ảnh từ Multipart
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(name) = field.name() {
            if name == "file" {
                image_data = field.bytes().await.unwrap().to_vec();
            }
        }
    }

    // 2. Gọi Gemini API để phân tích ảnh thật
    let api_key = "AIzaSyCa_gmMUF6PRd3jLOmjMr-FSTbAX6PWQE0"; // 🔑 Thay Key của bạn ở đây
    let api_url = format!(
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={}",
    api_key
);

    let base64_image = general_purpose::STANDARD.encode(&image_data);
    let client = reqwest::Client::new();

    // Prompt yêu cầu AI trả về JSON chuẩn
    let prompt = "Bạn là trợ lý kế toán chuyên nghiệp. Hãy phân tích hóa đơn này và trả về DUY NHẤT 1 khối JSON (không kèm chữ khác).
    Yêu cầu trích xuất:
    1. 'merchant': Tên công ty/cửa hàng.
    2. 'bill_date': Ngày lập hóa đơn (định dạng YYYY-MM-DD).
    3. 'amount': Tổng tiền thanh toán cuối cùng (số nguyên).
    4. 'category': Phân loại chính xác vào 1 trong các nhóm sau: 
       ['Điện', 'Nước', 'Nguyên liệu', 'Lương', 'Viễn thông', 'Mặt bằng', 'Khác'].
    
    Lưu ý: Nếu là hóa đơn EVN thì category là 'Điện'. Hóa đơn công ty cấp nước thì là 'Nước'.";

    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                { 
                    "text": prompt 
                },
                { 
                    "inline_data": { 
                        "mime_type": "image/jpeg", 
                        "data": base64_image 
                    } 
                }
            ]
        }]
    });

    // ... (đoạn gọi client.post giữ nguyên)
    let response = client.post(api_url).json(&payload).send().await.expect("Lỗi gọi Gemini");
    let json_res: serde_json::Value = response.json().await.expect("Lỗi đọc JSON từ Gemini");

    // 1. Lấy text thô từ Gemini
   // 1. Lấy chuỗi text từ cấu trúc phức tạp của Gemini
    let ai_text = json_res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("");

    if ai_text.is_empty() {
        eprintln!("⚠️ AI không trả về nội dung. Kiểm tra API Key hoặc Payload!");
    }

    // 2. Làm sạch chuỗi JSON (Gemini rất hay bọc trong ```json ... ```)
    let clean_json = ai_text
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim()
        .to_string();

    println!("🤖 AI Response Thật: {}", clean_json);

    // 3. Parse và xử lý dữ liệu
    let parsed: serde_json::Value = serde_json::from_str(&clean_json).unwrap_or_else(|e| {
        eprintln!("❌ Lỗi parse JSON từ AI: {}", e);
        serde_json::json!({
            "merchant": "Lỗi đọc hóa đơn",
            "amount": 0.0,
            "category": "Khác"
        })
    });

    // ... (Đoạn parse JSON giữ nguyên như cũ)

    let merchant = parsed["merchant"].as_str().unwrap_or("Không xác định").to_string();
    let bill_date = parsed["bill_date"].as_str().unwrap_or("2026-01-01").to_string(); // Ngày từ AI
    let amount = parsed["amount"].as_f64().unwrap_or(0.0);
    let category = parsed["category"].as_str().unwrap_or("Khác").to_string();

    // LOGIC PHẢN BIỆN: Tính trung bình chi phí của đúng LOẠI (Category) đó trong 30 ngày qua
    let avg_cost: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category = ? AND created_at >= DATE_SUB(NOW(), INTERVAL 1 MONTH)"
    )
    .bind(&category)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0.0);

    // Cảnh báo nếu vượt 20% so với trung bình cùng loại
    let is_warning = if avg_cost > 0.0 { amount > (avg_cost * 1.2) } else { false };

    // Lưu vào MySQL với trường bill_date
    let result = sqlx::query(
        "INSERT INTO expenses (merchant, bill_date, amount, category, is_warning) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&merchant)
    .bind(&bill_date)
    .bind(amount)
    .bind(&category)
    .bind(is_warning)
    .execute(&state.db)
    .await
    .expect("Lỗi lưu DB");

    Json(Expense {
        id: Some(result.last_insert_id() as i32),
        merchant,
        bill_date: Some(bill_date),
        amount,
        category,
        is_warning,
    })
}