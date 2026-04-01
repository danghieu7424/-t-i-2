use axum::{
    extract::{Multipart, State, Query, Path}, 
    routing::{get, post, delete, put},
    response::IntoResponse,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, MySql};
use std::sync::Arc;
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use crate::AppState;
use chrono::{NaiveDate, Datelike}; 
use crate::routes::auth::AuthUser; // NGƯỜI GÁC CỔNG
use std::sync::OnceLock;
use tokio::sync::Mutex as TokioMutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentExpense {
    pub id: i32,
    pub merchant: String,
    pub bill_date: String,
    pub amount: f64,
    pub category_slug: String,
    pub category_name: String, 
    pub items: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum JobStatus {
    Pending,    // Đang xếp hàng
    Processing, // Đang được Gemini phân tích
    Completed,  // Đã xong và lưu DB
    Failed(String), // Lỗi
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChartSeries {
    pub label: String,
    pub data: Vec<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DashboardData {
    pub month_labels: Vec<String>,
    pub chart_series: Vec<ChartSeries>,
    pub stats: Vec<StatItem>,
    pub recent_expenses: Vec<RecentExpense>,
    pub current_page: u32, // THÊM
    pub total_pages: u32,  // THÊM
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
    // ĐÃ BỎ: pub user_id: String,
}

#[derive(Deserialize)]
pub struct DashQuery { 
    pub month: Option<String>,
    pub page: Option<u32>,   // THÊM: Trang hiện tại
    pub limit: Option<u32>,  // THÊM: Số lượng/trang
}

#[derive(Deserialize)]
pub struct ManualExpenseReq {
    pub merchant: String,
    pub bill_date: String,
    pub amount: f64,
    pub category_name: String,
    // ĐÃ BỎ: pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Subscription {
    pub id: i32,
    pub merchant: String,
    pub amount: f64,
    pub category_name: String,
    pub start_date: String,
}

#[derive(Deserialize)]
pub struct SubReq {
    pub merchant: String,
    pub amount: f64,
    pub category_name: String,
    // ĐÃ BỎ: pub user_id: String,
}

#[derive(Deserialize)]
pub struct EditExpenseReq {
    pub id: i32,
    pub merchant: String,
    pub bill_date: String,
    pub amount: f64,
    pub category_name: String,
    pub items: serde_json::Value,
    // ĐÃ BỎ: pub user_id: String,
}

static AI_JOBS: OnceLock<Arc<TokioMutex<HashMap<String, JobStatus>>>> = OnceLock::new();

fn create_slug(input: &str) -> String {
    let mut slug = String::new();
    for c in input.to_lowercase().chars() {
        match c {
            'á'|'à'|'ả'|'ã'|'ạ'|'ă'|'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ'|'â'|'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ' => slug.push('a'),
            'đ' => slug.push('d'),
            'é'|'è'|'ẻ'|'ẽ'|'ẹ'|'ê'|'ế'|'ề'|'ể'|'ễ'|'ệ' => slug.push('e'),
            'í'|'ì'|'ỉ'|'ĩ'|'ị' => slug.push('i'),
            'ó'|'ò'|'ỏ'|'õ'|'ọ'|'ô'|'ố'|'ồ'|'ổ'|'ỗ'|'ộ'|'ơ'|'ớ'|'ờ'|'ở'|'ỡ'|'ợ' => slug.push('o'),
            'ú'|'ù'|'ủ'|'ũ'|'ụ'|'ư'|'ứ'|'ừ'|'ử'|'ữ'|'ự' => slug.push('u'),
            'ý'|'ỳ'|'ỷ'|'ỹ'|'ỵ' => slug.push('y'),
            ' ' => slug.push('-'),
            _ if c.is_ascii_alphanumeric() => slug.push(c),
            _ => slug.push('-'),
        }
    }
    let mut deduped = String::new();
    let mut last_char = ' ';
    for c in slug.chars() {
        if c == '-' && last_char == '-' { continue; }
        deduped.push(c);
        last_char = c;
    }
    deduped.trim_matches('-').to_string()
}

fn get_jobs() -> Arc<TokioMutex<HashMap<String, JobStatus>>> {
    AI_JOBS.get_or_init(|| Arc::new(TokioMutex::new(HashMap::new()))).clone()
}

pub fn expense_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_dashboard_data))
        .route("/upload", post(upload_invoice))
        .route("/upload/status/:job_id", get(check_job_status)) // <-- THÊM DÒNG NÀY
        .route("/manual", post(add_manual_expense))
        .route("/edit", put(edit_expense))
        .route("/budget", post(update_budget))
        .route("/subscription", post(add_subscription))
        .route("/subscriptions", get(get_active_subs))
        .route("/subscription/:id", delete(delete_sub))
        .route("/:id", delete(delete_expense))
}

// =====================================
// TẤT CẢ CÁC API DƯỚI ĐÂY ĐỀU CÓ AuthUser ĐỨNG ĐẦU TIÊN
// =====================================

async fn get_active_subs(
    auth: AuthUser, 
    State(state): State<Arc<AppState>>
) -> Json<Vec<Subscription>> {
    let rows = sqlx::query("SELECT id, merchant, amount, category_name, DATE_FORMAT(start_date, '%Y-%m-%d') as d FROM subscriptions WHERE user_id = ? AND is_active = 1")
        .bind(&auth.user_id).fetch_all(&state.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| Subscription { id: r.get("id"), merchant: r.get("merchant"), amount: r.get("amount"), category_name: r.get("category_name"), start_date: r.get("d") }).collect())
}

async fn delete_sub(
    auth: AuthUser, 
    State(state): State<Arc<AppState>>, 
    Path(id): Path<i32>
) -> impl IntoResponse {
    // Bảo mật: Chỉ cho phép XÓA dịch vụ của chính user đó!
    let _ = sqlx::query("UPDATE subscriptions SET is_active = 0 WHERE id = ? AND user_id = ?")
        .bind(id).bind(&auth.user_id).execute(&state.db).await;
    axum::http::StatusCode::OK
}

async fn edit_expense(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EditExpenseReq>,
) -> impl axum::response::IntoResponse {
    if payload.amount <= 0.0 {
        return (axum::http::StatusCode::BAD_REQUEST, "Số tiền phải lớn hơn 0").into_response();
    }

    let safe_merchant = if payload.merchant.trim().is_empty() { "Chi tiêu".to_string() } else { payload.merchant.clone() };
    let cat_name = if payload.category_name.trim().is_empty() { "Khác".to_string() } else { payload.category_name.clone() };
    let slug = create_slug(&cat_name);

    let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES (?, ?, ?)")
        .bind(&slug).bind(&cat_name).bind(&auth.user_id).execute(&state.db).await;

    let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category_slug = ?")
        .bind(&slug).fetch_one(&state.db).await.unwrap_or(0.0);
    let warn = if avg_cost > 0.0 { payload.amount > (avg_cost * 1.2) } else { false };

    let items_str = payload.items.to_string();

    let update_res = sqlx::query(
        "UPDATE expenses SET merchant = ?, bill_date = ?, amount = ?, category_slug = ?, is_warning = ?, raw_ai_data = ? WHERE id = ? AND user_id = ?"
    )
    .bind(&safe_merchant).bind(&payload.bill_date).bind(payload.amount).bind(&slug).bind(warn).bind(&items_str)
    .bind(payload.id).bind(&auth.user_id) // DÙNG AUTH
    .execute(&state.db).await;

    match update_res {
        Ok(_) => axum::http::StatusCode::OK.into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Lỗi cập nhật DB: {}", e)).into_response(),
    }
}

async fn add_manual_expense(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ManualExpenseReq>,
) -> impl axum::response::IntoResponse {
    if payload.amount <= 0.0 {
        return (axum::http::StatusCode::BAD_REQUEST, "Số tiền phải lớn hơn 0").into_response();
    }

    let today = chrono::Local::now().naive_local().date();
    let parsed_date = NaiveDate::parse_from_str(&payload.bill_date, "%Y-%m-%d").unwrap_or(today);
    if parsed_date > today {
        return (axum::http::StatusCode::BAD_REQUEST, "Không thể nhập giao dịch ở tương lai").into_response();
    }
    let safe_date_str = parsed_date.format("%Y-%m-%d").to_string();

    let safe_merchant = if payload.merchant.trim().is_empty() { "Chi tiêu thủ công".to_string() } else { payload.merchant.clone() };
    let cat_name = if payload.category_name.trim().is_empty() { "Khác".to_string() } else { payload.category_name.clone() };
    let slug = create_slug(&cat_name);

    let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES (?, ?, ?)")
        .bind(&slug).bind(&cat_name).bind(&auth.user_id).execute(&state.db).await;

    let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category_slug = ?")
        .bind(&slug).fetch_one(&state.db).await.unwrap_or(0.0);
    let warn = if avg_cost > 0.0 { payload.amount > (avg_cost * 1.2) } else { false };

    let insert_res = sqlx::query(
        "INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning, raw_ai_data) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&auth.user_id).bind(&safe_merchant).bind(&safe_date_str).bind(payload.amount).bind(&slug).bind(warn).bind("[]") // DÙNG AUTH
    .execute(&state.db).await;

    match insert_res {
        Ok(_) => axum::http::StatusCode::OK.into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Lỗi DB: {}", e)).into_response(),
    }
}

async fn delete_expense(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> impl axum::response::IntoResponse {
    // Bảo mật kép: AND user_id = auth.user_id
    let result = sqlx::query("DELETE FROM expenses WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(&auth.user_id) 
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn update_budget(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateBudgetRequest>,
) -> impl axum::response::IntoResponse {
    let query = "
        INSERT INTO budgets (user_id, category_slug, amount_limit, month_year)
        VALUES (?, ?, ?, CURDATE())
        ON DUPLICATE KEY UPDATE amount_limit = VALUES(amount_limit)
    ";

    sqlx::query(query)
        .bind(&auth.user_id) // DÙNG AUTH
        .bind(&payload.category_slug)
        .bind(payload.amount_limit)
        .execute(&state.db)
        .await
        .unwrap();

    axum::http::StatusCode::OK
}

async fn add_subscription(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubReq>,
) -> impl axum::response::IntoResponse {
    if payload.amount <= 0.0 {
        return (axum::http::StatusCode::BAD_REQUEST, "Số tiền phải lớn hơn 0").into_response();
    }
    let cat_name = if payload.category_name.trim().is_empty() { "Khác".to_string() } else { payload.category_name.clone() };
    let slug = create_slug(&cat_name);
    let user_id = &auth.user_id;

    let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES (?, ?, ?)")
        .bind(&slug).bind(&cat_name).bind(user_id).execute(&state.db).await;

    let res = sqlx::query("INSERT INTO subscriptions (user_id, merchant, amount, category_slug, category_name) VALUES (?, ?, ?, ?, ?)")
        .bind(user_id).bind(&payload.merchant).bind(payload.amount).bind(&slug).bind(&cat_name)
        .execute(&state.db).await;

    if res.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi DB").into_response();
    }

    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    let bill_date = format!("{}-01", current_month);

    let existing_bill = sqlx::query("SELECT id, amount, raw_ai_data FROM expenses WHERE user_id = ? AND merchant = 'Hóa đơn định kỳ tổng hợp' AND DATE_FORMAT(bill_date, '%Y-%m') = ?")
        .bind(user_id).bind(&current_month).fetch_optional(&state.db).await.unwrap_or(None);

    let new_item = serde_json::json!({"name": payload.merchant, "quantity": 1.0, "price": payload.amount, "total": payload.amount});

    if let Some(row) = existing_bill {
        let id: i32 = row.get("id");
        let mut amt: f64 = row.get("amount");
        let raw_data: String = row.get("raw_ai_data");
        
        let mut items: Vec<serde_json::Value> = serde_json::from_str(&raw_data).unwrap_or_default();
        items.push(new_item);
        amt += payload.amount;
        let new_raw = serde_json::to_string(&items).unwrap_or_default();

        let _ = sqlx::query("UPDATE expenses SET amount = ?, raw_ai_data = ? WHERE id = ?")
            .bind(amt).bind(&new_raw).bind(id).execute(&state.db).await;
    } else {
        let items = vec![new_item];
        let new_raw = serde_json::to_string(&items).unwrap_or_default();
        let _ = sqlx::query("INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning, raw_ai_data) VALUES (?, 'Hóa đơn định kỳ tổng hợp', ?, ?, 'dinh-ky', 0, ?)")
            .bind(user_id).bind(&bill_date).bind(payload.amount).bind(&new_raw).execute(&state.db).await;
    }

    axum::http::StatusCode::OK.into_response()
}

async fn get_dashboard_data(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<DashQuery>
) -> Json<DashboardData> {
    let target_month = params.month.unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    
    // ĐÃ SỬA: Sử dụng user_id từ Token mã hóa, an toàn 100%
    let user_id = &auth.user_id;

    let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES ('dinh-ky', 'Định kỳ', ?)")
        .bind(user_id).execute(&state.db).await;

    if target_month == current_month {
        let existing_bill: Option<i32> = sqlx::query_scalar("SELECT id FROM expenses WHERE user_id = ? AND merchant = 'Hóa đơn định kỳ tổng hợp' AND DATE_FORMAT(bill_date, '%Y-%m') = ?")
            .bind(user_id).bind(&target_month).fetch_optional(&state.db).await.unwrap_or(None);

        if existing_bill.is_none() {
            let active_subs = sqlx::query(
                "SELECT merchant, amount FROM subscriptions 
                 WHERE user_id = ? AND is_active = 1 
                 AND DATE_FORMAT(start_date, '%Y-%m') <= ?"
            )
            .bind(user_id).bind(&target_month).fetch_all(&state.db).await.unwrap_or_default();

            if !active_subs.is_empty() {
                let mut total_amt = 0.0;
                let mut items = Vec::new();
                for row in &active_subs {
                    let m: String = row.get("merchant");
                    let a: f64 = row.get("amount");
                    total_amt += a;
                    items.push(serde_json::json!({"name": m, "quantity": 1.0, "price": a, "total": a}));
                }
                
                let items_str = serde_json::to_string(&items).unwrap_or("[]".to_string());
                let bill_date = format!("{}-01", target_month);

                let _ = sqlx::query("INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning, raw_ai_data) VALUES (?, 'Hóa đơn định kỳ tổng hợp', ?, ?, 'dinh-ky', 0, ?)")
                    .bind(user_id).bind(&bill_date).bind(total_amt).bind(&items_str).execute(&state.db).await;
            }
        }
    }

    let category_names_rows = sqlx::query(
        "SELECT slug, display_name FROM categories WHERE user_id = ? OR user_id = 'system'"
    )
    .bind(user_id)
    .fetch_all(&state.db).await.unwrap_or_default();

    let mut name_map: HashMap<String, String> = HashMap::new();
    for row in category_names_rows {
        let slug: String = row.get("slug");
        let name: String = row.get("display_name");
        name_map.insert(slug, name);
    }

    let chart_rows = sqlx::query(
        "SELECT 
            DATE_FORMAT(bill_date, '%m') as month, 
            category_slug, 
            CAST(SUM(amount) AS DOUBLE) as total 
         FROM expenses 
         WHERE user_id = ? AND bill_date >= DATE_SUB(CURDATE(), INTERVAL 12 MONTH)
         GROUP BY month, category_slug"
    )
    .bind(user_id)
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
        .bind(user_id).bind(&target_month)
        .bind(user_id).bind(&target_month)
        .bind(user_id).bind(user_id)
        .fetch_all(&state.db).await.unwrap_or_default();

    // 🛡️ CHỐNG THỦNG RAM: Logic Phân Trang (Pagination)
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * limit;

    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(id) FROM expenses WHERE user_id = ? AND DATE_FORMAT(bill_date, '%Y-%m') = ?"
    )
    .bind(user_id).bind(&target_month).fetch_one(&state.db).await.unwrap_or(0);
    let total_pages = (total_count as f64 / limit as f64).ceil() as u32;

    let recent_rows = sqlx::query(
        "SELECT 
            e.id, e.merchant, DATE_FORMAT(e.bill_date, '%Y-%m-%d') as bill_date_str, 
            CAST(e.amount AS DOUBLE) as amount_f64, e.category_slug, e.raw_ai_data,
            COALESCE(c.display_name, e.category_slug) as category_name
         FROM expenses e
         LEFT JOIN categories c ON e.category_slug = c.slug AND (c.user_id = ? OR c.user_id = 'system')
         WHERE e.user_id = ? AND DATE_FORMAT(e.bill_date, '%Y-%m') = ?
         ORDER BY e.bill_date DESC, e.id DESC
         LIMIT ? OFFSET ?"
    )
    .bind(user_id)
    .bind(user_id)
    .bind(&target_month)
    .bind(limit as i32)  // Gắn Limit
    .bind(offset as i32) // Gắn Offset
    .fetch_all(&state.db).await.unwrap_or_default();

    let recent_expenses: Vec<RecentExpense> = recent_rows.into_iter().map(|r| {
        let raw_data: Option<String> = r.get("raw_ai_data");
        RecentExpense {
            id: r.get("id"),
            merchant: r.get("merchant"),
            bill_date: r.get("bill_date_str"),
            amount: r.get("amount_f64"),
            category_slug: r.get("category_slug"),
            category_name: r.get("category_name"),
            items: raw_data.map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!([]))).unwrap_or(serde_json::json!([])),
        }
    }).collect();

    let current_month_num = chrono::Local::now().month() as usize;
    let mut labels = Vec::new();
    let mut month_to_idx = HashMap::new();
    
    for i in 0..12 {
        let mut m = (current_month_num + 12 - 11 + i) % 12;
        if m == 0 { m = 12; }
        labels.push(format!("T{}", m));
        month_to_idx.insert(m, i); 
    }
    
    let mut category_data: HashMap<String, Vec<f64>> = HashMap::new();

    for row in chart_rows {
        let month_str: String = row.get("month");
        let slug: String = row.get("category_slug");
        let val: f64 = row.get("total");
        
        let month_num = month_str.parse::<usize>().unwrap_or(1);
        
        if let Some(&idx) = month_to_idx.get(&month_num) {
            let series = category_data.entry(slug).or_insert_with(|| vec![0.0; 12]);
            series[idx] += val;
        }
    }

    let chart_series: Vec<ChartSeries> = category_data.into_iter().map(|(slug, data)| {
        let label = name_map.get(&slug).cloned().unwrap_or(slug);
        ChartSeries { label, data }
    }).collect();

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
        chart_series, 
        stats, 
        recent_expenses,
        current_page: page,     // Trả về Frontend
        total_pages             // Trả về Frontend
    })
}

// API KIỂM TRA TRẠNG THÁI JOB
async fn check_job_status(
    _auth: AuthUser, // THÊM DẤU _ ĐỂ XÓA WARNING UNUSED VARIABLE
    Path(job_id): Path<String>
) -> Json<serde_json::Value> {
    let jobs = get_jobs();
    let status = jobs.lock().await.get(&job_id).cloned().unwrap_or(JobStatus::Failed("Không tìm thấy Job".to_string()));
    
    let json = match status {
        JobStatus::Pending => serde_json::json!({"state": "Pending"}),
        JobStatus::Processing => serde_json::json!({"state": "Processing"}),
        JobStatus::Completed => serde_json::json!({"state": "Completed"}),
        JobStatus::Failed(e) => serde_json::json!({"state": "Failed", "error": e}),
    };
    Json(json)
}

// API TẢI HÓA ĐƠN (NHẬN ẢNH VÀ ĐẨY VÀO HÀNG ĐỢI)
async fn upload_invoice(
    auth: AuthUser,
    State(state): State<Arc<AppState>>, 
    mut multipart: Multipart
) -> impl axum::response::IntoResponse {
    let mut image_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            image_data = field.bytes().await.unwrap_or_default().to_vec();
        } 
    }

    if image_data.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Không tìm thấy ảnh hóa đơn").into_response();
    }

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Server thiếu GEMINI_API_KEY").into_response();
    }

    let user_id = auth.user_id.clone();
    let job_id = crate::utils::suid::suid(); // Phát thẻ Job ID

    // BƯỚC 1: Đưa vào hàng đợi
    get_jobs().lock().await.insert(job_id.clone(), JobStatus::Pending);

    // 🛡️ CHỐNG LỖI E0382: Tạo bản sao của job_id để đưa cho luồng ngầm
    let thread_job_id = job_id.clone();

    // BƯỚC 2: Tách luồng (Thread) cho chạy ngầm, không bắt Request phải chờ
    tokio::spawn(async move {
        // Đặt lại tên biến bên trong luồng để tái sử dụng code cũ
        let job_id = thread_job_id; 

        // Xếp hàng chờ trạm thu phí (Semaphore)
        let permit = match state.gemini_semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                get_jobs().lock().await.insert(job_id.clone(), JobStatus::Failed("Server quá tải".to_string()));
                return;
            }
        };

        // Đã qua trạm, bắt đầu xử lý
        get_jobs().lock().await.insert(job_id.clone(), JobStatus::Processing);

        let category_list: Vec<String> = sqlx::query_scalar("SELECT slug FROM categories WHERE user_id = ? OR user_id = 'system'")
            .bind(&user_id).fetch_all(&state.db).await.unwrap_or_default();
        let categories_str = category_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", ");

        let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={}", api_key);
        let base64_image = general_purpose::STANDARD.encode(&image_data);
        let client = reqwest::Client::new();
        
        let prompt = format!(
            "Trích xuất hóa đơn. Trả về JSON chính xác. Không kèm text giải thích:
            1. 'merchant': Tên siêu thị/quán ăn.
            2. 'bill_date': YYYY-MM-DD.
            3. 'amount': Tổng tiền (chỉ lấy số).
            4. 'category_slug': Chọn từ: {}. Nếu chưa có, tự tạo slug viết thường.
            5. 'category_name': Tên hiển thị tiếng Việt.
            6. 'items': Mảng mặt hàng [{{\"name\": \"...\", \"quantity\": 1, \"price\": 0, \"total\": 0}}].",
            categories_str
        );

        let payload = serde_json::json!({
            "contents": [{"parts": [{"text": prompt}, {"inline_data": {"mime_type": "image/jpeg", "data": base64_image}}]}],
            "generationConfig": { "response_mime_type": "application/json" }
        });

        let response_result = client.post(api_url).json(&payload).send().await;
        
        drop(permit); // MỞ BARIE CHO LUỒNG KHÁC VÀO GEMINI NGAY LẬP TỨC

        let response = match response_result {
            Ok(res) => res,
            Err(e) => {
                get_jobs().lock().await.insert(job_id.clone(), JobStatus::Failed(format!("Lỗi mạng: {}", e)));
                return;
            }
        };

        if !response.status().is_success() {
            get_jobs().lock().await.insert(job_id.clone(), JobStatus::Failed("AI từ chối xử lý".to_string()));
            return;
        }

        let res_json: serde_json::Value = response.json().await.unwrap_or_default();
        let ai_text = res_json.get("candidates").and_then(|c| c.get(0)).and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts")).and_then(|p| p.get(0)).and_then(|p| p.get("text"))
            .and_then(|t| t.as_str()).unwrap_or("{}");

        let parsed: serde_json::Value = serde_json::from_str(ai_text).unwrap_or_else(|_| serde_json::json!({}));
        let m = parsed["merchant"].as_str().unwrap_or("Không rõ").to_string();
        let d = parsed["bill_date"].as_str().map(|s| s.to_string()).unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
        
        let a = if parsed["amount"].is_number() { 
            parsed["amount"].as_f64().unwrap_or(0.0) 
        } else { 
            parsed["amount"].as_str().unwrap_or("0").replace(",", "").replace(".", "").replace(" ", "").parse::<f64>().unwrap_or(0.0) 
        };
        
        let mut c = parsed["category_slug"].as_str().unwrap_or("khac").to_string();
        if c.is_empty() { c = "khac".to_string(); }
        
        if !category_list.contains(&c) { 
            let mut display_name = parsed["category_name"].as_str().unwrap_or(&c).to_string();
            if let Some(first_char) = display_name.chars().next() {
                display_name = format!("{}{}", first_char.to_uppercase(), &display_name[first_char.len_utf8()..]);
            }
            let _ = sqlx::query("INSERT IGNORE INTO categories (slug, display_name, user_id) VALUES (?, ?, ?)")
                .bind(&c).bind(&display_name).bind(&user_id).execute(&state.db).await;
        }

        let avg_cost: f64 = sqlx::query_scalar::<MySql, f64>("SELECT COALESCE(AVG(amount), 0) FROM expenses WHERE category_slug = ?")
            .bind(&c).fetch_one(&state.db).await.unwrap_or(0.0);
        let warn = if avg_cost > 0.0 { a > (avg_cost * 1.2) } else { false };

        let items_json_str = parsed["items"].to_string();
        let items_str = if items_json_str == "null" { "[]".to_string() } else { items_json_str };

        let insert_res = sqlx::query(
            "INSERT INTO expenses (user_id, merchant, bill_date, amount, category_slug, is_warning, raw_ai_data) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&user_id).bind(&m).bind(&d).bind(a).bind(&c).bind(warn).bind(&items_str)
        .execute(&state.db).await;

        if insert_res.is_ok() {
            get_jobs().lock().await.insert(job_id.clone(), JobStatus::Completed);
        } else {
            get_jobs().lock().await.insert(job_id.clone(), JobStatus::Failed("Lỗi lưu DB".to_string()));
        }
    });

    // BƯỚC 3: Trả Job ID về ngay lập tức cho Frontend
    // Biến `job_id` ở đây là bản gốc, chưa hề bị mang đi đâu cả nên Rust sẽ không phàn nàn nữa
    Json(serde_json::json!({
        "job_id": job_id,
        "message": "Đã đưa vào hàng đợi"
    })).into_response()
}