use axum::{
    extract::{Multipart, State, Query, Path}, 
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, MySql};
use std::sync::Arc;
use base64::{engine::general_purpose, Engine as _};
use crate::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentExpense {
    pub id: i32,
    pub merchant: String,
    pub bill_date: String,
    pub amount: f64,
    pub category_slug: String,
    pub category_name: String, 
    pub items: serde_json::Value, // Chứa mảng mặt hàng chi tiết
}

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
    pub recent_expenses: Vec<RecentExpense>, // Đã thêm vào Dashboard
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatItem {
    pub title: String,
    pub amount: f64,
    pub percent: f64,
    pub budget: f64,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct UpdateBudgetRequest {
    pub category_slug: String,
    pub amount_limit: f64,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct DashQuery { 
    pub user_id: String,
    pub month: Option<String> // Định dạng: "YYYY-MM"
}

pub fn expense_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_dashboard_data))
        .route("/upload", post(upload_invoice))
        .route("/budget", post(update_budget))
        .route("/:id", delete(delete_expense))
}

// --- API XÓA GIAO DỊCH ---
async fn delete_expense(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Query(params): Query<DashQuery>
) -> impl axum::response::IntoResponse {
    // Luôn check user_id để bảo mật, tránh việc user này xóa bill của user khác
    let result = sqlx::query("DELETE FROM expenses WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(&params.user_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn update_budget(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateBudgetRequest>,
) -> impl axum::response::IntoResponse {
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
    let target_month = params.month.unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
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

    let stats_query = format!("
        SELECT 
            c.display_name, c.slug, 
            COALESCE(curr.total, 0) as total_amount,
            COALESCE(prev.total, 0) as prev_amount,
            COALESCE(b.amount_limit, 0) as budget_limit
        FROM categories c
        LEFT JOIN (
            SELECT category_slug, SUM(amount) as total FROM expenses 
            WHERE user_id = ? AND DATE_FORMAT(bill_date, '%Y-%m') = ?
            GROUP BY category_slug
        ) curr ON c.slug = curr.category_slug
        LEFT JOIN (
            SELECT category_slug, SUM(amount) as total FROM expenses 
            WHERE user_id = ? AND DATE_FORMAT(bill_date, '%Y-%m') = DATE_FORMAT(DATE_SUB(CONCAT(?, '-01'), INTERVAL 1 MONTH), '%Y-%m')
            GROUP BY category_slug
        ) prev ON c.slug = prev.category_slug
        LEFT JOIN budgets b ON c.slug = b.category_slug AND b.user_id = ?
        WHERE c.user_id = ? OR c.user_id = 'system'
        GROUP BY c.slug, c.display_name, curr.total, prev.total, b.amount_limit
    ");

    let stats_rows = sqlx::query(&stats_query)
        .bind(&params.user_id).bind(&target_month)
        .bind(&params.user_id).bind(&target_month)
        .bind(&params.user_id).bind(&params.user_id)
        .fetch_all(&state.db).await.unwrap_or_default();

    // 3. LỌC CHI TIẾT GIAO DỊCH THEO THÁNG (VÀ MỚI NHẤT LÊN ĐẦU)
    let recent_rows = sqlx::query(
        "SELECT 
            e.id, e.merchant, DATE_FORMAT(e.bill_date, '%Y-%m-%d') as bill_date_str, 
            CAST(e.amount AS DOUBLE) as amount_f64, e.category_slug, e.raw_ai_data,
            COALESCE(c.display_name, e.category_slug) as category_name
         FROM expenses e
         LEFT JOIN categories c ON e.category_slug = c.slug AND (c.user_id = ? OR c.user_id = 'system')
         WHERE e.user_id = ? AND DATE_FORMAT(e.bill_date, '%Y-%m') = ?
         ORDER BY e.bill_date DESC, e.id DESC"
    )
    .bind(&params.user_id) // Bind cho điều kiện JOIN
    .bind(&params.user_id) // Bind cho điều kiện WHERE
    .bind(&target_month)
    .fetch_all(&state.db).await.unwrap_or_default();

    let recent_expenses: Vec<RecentExpense> = recent_rows.into_iter().map(|r| {
        let raw_data: Option<String> = r.get("raw_ai_data");
        RecentExpense {
            id: r.get("id"),
            merchant: r.get("merchant"),
            bill_date: r.get("bill_date_str"),
            amount: r.get("amount_f64"),
            category_slug: r.get("category_slug"),
            category_name: r.get("category_name"), // MAP VÀO STRUCT
            items: raw_data.map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!([]))).unwrap_or(serde_json::json!([])),
        }
    }).collect();

    let mut dien = vec![0.0; 12];
    let mut nuoc = vec![0.0; 12];
    let mut nl = vec![0.0; 12];
    let labels = vec!["T4","T5","T6","T7","T8","T9","T10","T11","T12","T1","T2","T3"];

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

    let stats = stats_rows.into_iter().map(|row| {
        let title: String = row.get("display_name");
        let slug: String = row.get("slug");
        let amount: f64 = row.get("total_amount");
        let prev_amount: f64 = row.get("prev_amount");
        let budget: f64 = row.get("budget_limit");

        let percent = if prev_amount > 0.0 { ((amount - prev_amount) / prev_amount) * 100.0 } else { 0.0 };
        StatItem { title, amount, percent, budget, slug }
    }).collect();

    Json(DashboardData { 
        month_labels: labels.into_iter().map(|s| s.into()).collect(), 
        dien_series: dien, nuoc_series: nuoc, nl_series: nl, 
        stats, recent_expenses 
    })
}

async fn upload_invoice(State(state): State<Arc<AppState>>, mut multipart: Multipart) -> Json<Expense> {
    let mut image_data = Vec::new();
    let mut user_id = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            image_data = field.bytes().await.unwrap_or_default().to_vec();
        } else if name == "user_id" {
            user_id = field.text().await.unwrap_or_default();
        }
    }

    if user_id.is_empty() { user_id = "guest".to_string(); }

    let category_list: Vec<String> = sqlx::query_scalar("SELECT slug FROM categories WHERE user_id = ? OR user_id = 'system'")
        .bind(&user_id).fetch_all(&state.db).await.unwrap_or_default();
    
    let categories_str = category_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", ");

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={}", api_key);
    
    let base64_image = general_purpose::STANDARD.encode(&image_data);
    let client = reqwest::Client::new();
    
    // --- DẠY AI TRẢ VỀ THÊM TÊN TIẾNG VIỆT CÓ DẤU ---
    let prompt = format!(
        "Bạn là chuyên gia OCR hóa đơn. Trích xuất JSON:
        1. 'merchant': Tên siêu thị/đơn vị.
        2. 'bill_date': YYYY-MM-DD.
        3. 'amount': Tổng tiền (số nguyên).
        4. 'category_slug': Phân loại vào: {}. NẾU KHÔNG THUỘC LOẠI NÀO, HÃY TỰ TẠO 1 SLUG MỚI (viết thường, không dấu, ngăn cách bằng dấu gạch ngang, vd: 'sieu-thi', 'quan-ao').
        5. 'category_name': Tên hiển thị của danh mục bằng tiếng Việt có dấu chuẩn xác (vd: 'Siêu thị', 'Quần áo').
        6. 'items': Mảng mặt hàng [{{ \"name\": \"...\", \"quantity\": 1, \"price\": 0, \"total\": 0 }}].
        Chỉ trả về JSON.",
        categories_str
    );

    let payload = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}, {"inline_data": {"mime_type": "image/jpeg", "data": base64_image}}]}]
    });

    let response = client.post(api_url).json(&payload).send().await.expect("Lỗi mạng");
    let status = response.status();
    let res_json = response.json::<serde_json::Value>().await.unwrap();

    if !status.is_success() {
        return Json(Expense { id: None, user_id, merchant: "Lỗi API".into(), bill_date: None, amount: 0.0, category: "khac".into(), is_warning: false });
    }

    let ai_text = res_json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("");
    let clean_json = ai_text.replace("```json", "").replace("```", "").trim().to_string();
    let parsed: serde_json::Value = serde_json::from_str(&clean_json).unwrap_or_default();

    let m = parsed["merchant"].as_str().unwrap_or("Không rõ").to_string();
    let d = parsed["bill_date"].as_str().unwrap_or("2026-03-22").to_string();
    let a = if parsed["amount"].is_number() { parsed["amount"].as_f64().unwrap_or(0.0) } else { parsed["amount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) };
    let mut c = parsed["category_slug"].as_str().unwrap_or("khac").to_string();
    
    if !category_list.contains(&c) { 
        // Lấy tên tiếng Việt có dấu từ AI, nếu AI quên thì mới fallback về slug
        let mut display_name = parsed["category_name"].as_str().unwrap_or(&c).to_string();
        
        // Viết hoa chữ cái đầu tiên cho đẹp (vd: "Quần áo")
        if let Some(first_char) = display_name.chars().next() {
            display_name = format!("{}{}", first_char.to_uppercase(), &display_name[first_char.len_utf8()..]);
        }

        let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES (?, ?, ?)")
            .bind(&c).bind(&display_name).bind(&user_id)
            .execute(&state.db).await;
    }

    let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category_slug = ?")
        .bind(&c).fetch_one(&state.db).await.unwrap_or(0.0);
    let warn = if avg_cost > 0.0 { a > (avg_cost * 1.2) } else { false };

    let items_json_str = parsed["items"].to_string();
    let items_str = if items_json_str == "null" { "[]".to_string() } else { items_json_str };

    let result = sqlx::query(
        "INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning, raw_ai_data) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&user_id).bind(&m).bind(&d).bind(a).bind(&c).bind(warn).bind(&items_str)
    .execute(&state.db).await.expect("Lỗi Insert DB");

    Json(Expense { id: Some(result.last_insert_id() as i32), user_id, merchant: m, bill_date: Some(d), amount: a, category: c, is_warning: warn })
}