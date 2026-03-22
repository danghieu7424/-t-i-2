use axum::{
    extract::{Multipart, State, Query}, 
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
    pub user_id: String,
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
    pub budget: f64, // THÊM TRƯỜNG NÀY
    pub slug: String,
}

#[derive(Deserialize)]
pub struct UpdateBudgetRequest {
    pub category_slug: String,
    pub amount_limit: f64,
    pub user_id: String,
}

// Thêm query parameter để nhận user_id từ Frontend gửi lên
#[derive(Deserialize)]
pub struct DashQuery { pub user_id: String }

pub fn expense_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_dashboard_data))
        .route("/upload", post(upload_invoice))
        .route("/budget", post(update_budget))
}

async fn update_budget(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateBudgetRequest>,
) -> impl axum::response::IntoResponse {
    // Dùng ON DUPLICATE KEY UPDATE để nếu có rồi thì ghi đè, chưa có thì tạo mới
    // Lưu ý: Hieu cần tạo UNIQUE KEY cho bảng budgets (user_id, category_slug) trong MySQL trước
    let query = "
        INSERT INTO budgets (user_id, category_slug, amount_limit, month_year)
        VALUES (?, ?, ?, CURDATE())
        ON DUPLICATE KEY UPDATE amount_limit = VALUES(amount_limit)
    ";

    sqlx::query(query)
        .bind(&payload.user_id)
        .bind(&payload.category_slug)
        .bind(payload.amount_limit)
        .execute(&state.db)
        .await
        .unwrap();

    axum::http::StatusCode::OK
}

async fn get_dashboard_data(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DashQuery>
) -> Json<DashboardData> {
    // 1. Lấy dữ liệu biểu đồ (12 tháng gần nhất)
    let chart_rows = sqlx::query(
        "SELECT 
            DATE_FORMAT(bill_date, '%m') as month, 
            category_slug, 
            CAST(SUM(amount) AS DOUBLE) as total 
         FROM expenses 
         WHERE user_id = ? AND bill_date >= DATE_SUB(CURDATE(), INTERVAL 12 MONTH)
         GROUP BY month, category_slug"
    )
    .bind(&params.user_id)
    .fetch_all(&state.db).await.unwrap_or_default();

    // 2. Lấy Stats kèm Budget thực tế từ bảng budgets
    let stats_rows = sqlx::query(
        "SELECT 
            c.display_name, 
            c.slug, 
            COALESCE(curr.total, 0) as total_amount,
            COALESCE(prev.total, 0) as prev_amount,
            COALESCE(b.amount_limit, 0) as budget_limit
        FROM categories c
        -- Lấy dữ liệu tháng hiện tại
        LEFT JOIN (
            SELECT category_slug, SUM(amount) as total 
            FROM expenses 
            WHERE user_id = ? AND MONTH(bill_date) = MONTH(CURDATE()) AND YEAR(bill_date) = YEAR(CURDATE())
            GROUP BY category_slug
        ) curr ON c.slug = curr.category_slug
        -- Lấy dữ liệu tháng trước
        LEFT JOIN (
            SELECT category_slug, SUM(amount) as total 
            FROM expenses 
            WHERE user_id = ? AND MONTH(bill_date) = MONTH(DATE_SUB(CURDATE(), INTERVAL 1 MONTH)) 
            AND YEAR(bill_date) = YEAR(DATE_SUB(CURDATE(), INTERVAL 1 MONTH))
            GROUP BY category_slug
        ) prev ON c.slug = prev.category_slug
        LEFT JOIN budgets b ON c.slug = b.category_slug AND b.user_id = ?
        GROUP BY c.slug, c.display_name, curr.total, prev.total, b.amount_limit"
    )
    .bind(&params.user_id)
    .bind(&params.user_id)
    .bind(&params.user_id)
    .fetch_all(&state.db).await.unwrap_or_default();

    // Khởi tạo mảng series cho biểu đồ
    let mut dien = vec![0.0; 12];
    let mut nuoc = vec![0.0; 12];
    let mut nl = vec![0.0; 12];
    let labels = vec!["T4","T5","T6","T7","T8","T9","T10","T11","T12","T1","T2","T3"];

    // Duyệt chart_rows để đổ vào biểu đồ
    for row in chart_rows {
        let month_str: String = row.get("month");
        let slug: String = row.get("category_slug");
        let val: f64 = row.get("total");
        
        let month_num = month_str.parse::<usize>().unwrap_or(1);
        let idx = if month_num >= 4 { month_num - 4 } else { month_num + 8 };
        
        if idx < 12 {
            match slug.as_str() {
                "dien" => dien[idx] = val,
                "nuoc" => nuoc[idx] = val,
                "nguyen-lieu" => nl[idx] = val,
                _ => {}
            }
        }
    }

    // Duyệt stats_rows để hiển thị các thẻ StatCard (Lấy budget thực tế từ DB)
    let stats = stats_rows.into_iter().map(|row| {
        let title: String = row.get("display_name");
        let slug: String = row.get("slug");
        let amount: f64 = row.get("total_amount");
        let prev_amount: f64 = row.get("prev_amount");
        let budget: f64 = row.get("budget_limit");

        // TÍNH PHẦN TRĂM TĂNG GIẢM TẠI ĐÂY
        let percent = if prev_amount > 0.0 {
            ((amount - prev_amount) / prev_amount) * 100.0
        } else {
            0.0
        };

        StatItem { title, amount, percent, budget, slug }
    }).collect();

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
    let mut user_id = String::new();

    // 1. Thu thập dữ liệu từ Multipart
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            image_data = field.bytes().await.unwrap_or_default().to_vec();
        } else if name == "user_id" {
            user_id = field.text().await.unwrap_or_default();
        }
    }

    if user_id.is_empty() { user_id = "guest".to_string(); }

    // 2. Lấy danh sách Category Slugs động
    let category_list: Vec<String> = sqlx::query_scalar("SELECT slug FROM categories")
        .fetch_all(&state.db).await.unwrap_or_default();
    
    let categories_str = category_list.iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(", ");

    // 3. Chuẩn bị Gemini API
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={}", api_key);
    
    let base64_image = general_purpose::STANDARD.encode(&image_data);
    let client = reqwest::Client::new();
    
    let prompt = format!(
        "Bạn là chuyên gia OCR hóa đơn tại Việt Nam. Trích xuất JSON:
        1. 'merchant': Tên đơn vị bán hàng.
        2. 'bill_date': YYYY-MM-DD.
        3. 'amount': Tổng tiền (số nguyên).
        4. 'category_slug': Chọn 1 trong: {}. 
        Chỉ trả về JSON, không giải thích.",
        categories_str
    );

    let payload = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}, {"inline_data": {"mime_type": "image/jpeg", "data": base64_image}}]}]
    });

    // ... (Đoạn tạo payload giữ nguyên) ...

    let response = client.post(api_url).json(&payload).send().await.expect("Lỗi kết nối mạng");
    let status = response.status();
    let res_json = response.json::<serde_json::Value>().await.unwrap();

    // blind spot: Nếu API lỗi (Key sai, hết hạn...), in ra mã lỗi của Google
    if !status.is_success() {
        println!("\n❌ [ERROR] Gemini API trả về lỗi: {}", status);
        println!("Chi tiết: {:?}", res_json);
        return Json(Expense { id: None, user_id, merchant: "Lỗi API".into(), bill_date: None, amount: 0.0, category: "khac".into(), is_warning: false });
    }

    // Kiểm tra xem có candidate nào không
    let ai_text = res_json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or_else(|| {
        println!("⚠️ [WARN] AI không trả về text. Lý do: {:?}", res_json["candidates"][0]["finishReason"]);
        ""
    });

    println!("\n--- [DEBUG] AI RAW RESPONSE ---");
    println!("{}", ai_text);
    println!("-------------------------------\n");
    
    // ... (Đoạn xử lý JSON bên dưới giữ nguyên) ...

    // 5. Làm sạch và Parse JSON
    let clean_json = ai_text.replace("```json", "").replace("```", "").trim().to_string();
    let parsed: serde_json::Value = serde_json::from_str(&clean_json).unwrap_or_default();

    let m = parsed["merchant"].as_str().unwrap_or("Không rõ").to_string();
    let d = parsed["bill_date"].as_str().unwrap_or("2026-03-22").to_string();
    
    // Ép kiểu Amount linh hoạt
    let a = if parsed["amount"].is_number() {
        parsed["amount"].as_f64().unwrap_or(0.0)
    } else {
        parsed["amount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)
    };
    
    let mut c = parsed["category_slug"].as_str().unwrap_or("khac").to_string();
    if !category_list.contains(&c) { c = "khac".to_string(); }

    // 6. Tính toán cảnh báo
    let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category_slug = ?")
        .bind(&c).fetch_one(&state.db).await.unwrap_or(0.0);
    let warn = if avg_cost > 0.0 { a > (avg_cost * 1.2) } else { false };

    // 7. Lưu Database
    let result = sqlx::query(
        "INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&m).bind(&d).bind(a).bind(&c).bind(warn)
    .execute(&state.db).await.expect("Lỗi Insert DB");

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