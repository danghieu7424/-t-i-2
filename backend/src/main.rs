use axum::{
    extract::{ Request, DefaultBodyLimit },
    middleware::{ self, Next },
    response::{ Response },
    routing::Router,
    http::{ Method, HeaderValue },
};
use axum::http::header::{ self, HeaderName };
use tower_http::{ cors::CorsLayer, services::{ ServeDir, ServeFile } };
use sqlx::{ mysql::MySqlPoolOptions, MySqlPool }; // Giữ lại MySqlPoolOptions và MySqlPool
use dotenvy::dotenv;
// use axum::routing::{get}; // Chỉ cần get cho api_handler
use std::io::{ self, Write };
use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::Path;
use p256::{ SecretKey, elliptic_curve::sec1::ToEncodedPoint };
use hex;

// 1. Khai báo và Import Module Route
mod utils;
mod routes;
// use routes::{ auth, user };
use routes::{ auth, expenses }; // <-- SỬA: Thêm crypto

// Thêm Khóa Server vào AppState
#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
}

// Lưu ý: User struct đã được chuyển sang src/routes/user.rs để giữ main.rs gọn gàng.
fn visible_len(s: &str) -> usize {
    // Regex bỏ các đoạn \x1b[...m
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    let clean = re.replace_all(s, "");
    clean.len()
}

fn redraw(logs: &Vec<String>) {
    // Xóa màn hình và đặt con trỏ về góc trên bên trái
    // print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();

    // Tính độ dài lớn nhất của toàn bộ log (sau khi bỏ ANSI)
    let max_len = logs
        .iter()
        .map(|s| {
            let time = Local::now().format("%H:%M:%S%.3f").to_string();
            visible_len(&format!("[{}] {}", time, s))
        })
        .max()
        .unwrap_or(0);

    let width = max_len + 2; // thêm padding

    // Vẽ khung trên
    println!("┌{}┐", "─".repeat(width));

    // In từng dòng log trong khung
    for entry in logs {
        let time = Local::now().format("%H:%M:%S%.3f").to_string();
        let content = format!("\x1b[90m[{}]\x1b[0m {}", time, entry);

        let visible = visible_len(&format!("[{}] {}", time, entry));
        let padding = if max_len > visible { max_len - visible } else { 0 };

        println!("│ {}{} │", content, " ".repeat(padding));
    }

    // Vẽ khung dưới
    println!("└{}┘", "─".repeat(width));
}
// 3. Phục hồi Middleware
async fn my_logging_middleware(req: Request, next: Next) -> Response {
    let mut logs = Vec::new();
    let method = req.method().clone();
    let uri = req.uri().clone();
    //\x1b[90mĐã nhận Request:
    logs.push(format!("\x1b[1;32m==> \x1b[1;93m{}\x1b[0m {}", method, uri));

    let response = next.run(req).await;
    let status = response.status();

    let status_color = match status.as_u16() {
        200..=299 => "\x1b[1;32m", // xanh lá cho thành công
        300..=399 => "\x1b[1;36m", // xanh dương nhạt cho redirect
        400..=499 => "\x1b[1;93m", // vàng cho lỗi client
        500..=599 => "\x1b[1;91m", // đỏ cho lỗi server
        _ => "\x1b[0m", // mặc định
    };

    // \x1b[90mĐã gửi Response:
    logs.push(format!("\x1b[1;34m<== \x1b[0m({}{}{}) {}", status_color, status, "\x1b[0m", uri));
    redraw(&logs);
    response
}

// ==- KHẮC PHỤC LỖI E0601: MAIN FUNCTION NOT FOUND ==-
#[tokio::main]
async fn main() {
    dotenv().ok();
    print!("\x1b[2J\x1b[H");
    let port: u16 = std::env
        ::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let database_url = std::env::var("DATABASE_URL").expect("Chưa set DATABASE_URL trong .env");

    // Khắc phục lỗi DB connect (thêm .connect)
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await
        .expect("\x1b[31mKhông thể kết nối đến MySQL\x1b[0m");
    println!("✅ \x1b[32mĐã kết nối MySQL thành công!\x1b[0m");
        
    // Lưu vào AppState
    let state = AppState {
        db: pool,
    };

    // Cấu hình CORS (Đã sửa lỗi allow_any_origin)
    // main.rs
    let allowed_origins = [
        "https://test.dh74.io.vn".parse::<HeaderValue>().unwrap(),
        "https://des.dh74.io.vn".parse::<HeaderValue>().unwrap(),
        "http://localhost:8080".parse::<HeaderValue>().unwrap(),
        "http://localhost:5000".parse::<HeaderValue>().unwrap(),
        "http://192.168.7.10:5000".parse::<HeaderValue>().unwrap(),
        "http://127.0.0.1:8080".parse::<HeaderValue>().unwrap(),
    ];

    let allowed_headers = [
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::ACCEPT,
        HeaderName::from_static("x-requested-with"),
        HeaderName::from_static("sec-fetch-dest"),
        HeaderName::from_static("sec-fetch-mode"),
        HeaderName::from_static("cache-control"),
        HeaderName::from_static("x-auth-token"),
        HeaderName::from_static("sec-fetch-site"),
    ];

    let cors_layer = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(allowed_headers)
        .allow_credentials(true);
    // .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    // .allow_origin(tower_http::cors::Any)
    // .allow_methods(tower_http::cors::Any)

    // Kiểm tra folder storages nếu chưa có thì tạo
    if !Path::new("storages").exists() {
        fs::create_dir_all("storages").expect("Không tạo được folder storages");
        println!("Đã tạo thư mục storages");
    }

    let spa_service = ServeDir::new("public").fallback(ServeFile::new("public/index.html"));
    let spa_storages = ServeDir::new("storages");

    let shared_state = std::sync::Arc::new(state);

    let app = Router::new()

        .nest("/api/auth", auth::auth_routes())
        .nest("/api/expenses", expenses::expense_routes())

        .fallback_service(spa_service)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024))
        // Áp dụng Middleware và CORS
        .layer(middleware::from_fn(my_logging_middleware))
        .layer(cors_layer)
        .with_state(shared_state);

    let addr_str = format!("localhost:{}", port);
    println!("🚀 \x1b[34mServer đang lắng nghe trên http://{}\x1b[0m", addr_str);
    
    let _addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
