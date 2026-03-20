use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{engine::general_purpose, Engine as _};
use rand::{thread_rng, RngCore};

// --- Cấu hình và Trạng thái ---

// Mốc thời gian tuỳ chọn (2023-11-14 22:33:20 +07:00)
// Tương đương 1700000000000 miligiây
const EPOCH: u64 = 1700000000000;

// Trạng thái được chia sẻ giữa các luồng (thread-safe)
static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

// --- Cấu trúc ID (Giống như JS) ---
// 10 bit WorkerId + DatacenterId (giống workerId = 1n, datacenterId = 1n)
const WORKER_ID_BITS: u64 = 5;
const DATACENTER_ID_BITS: u64 = 5;
const SEQUENCE_BITS: u64 = 12;

const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1; // 4095
const TIMESTAMP_SHIFT: u64 = WORKER_ID_BITS + DATACENTER_ID_BITS + SEQUENCE_BITS; // 22

/// Lấy thời gian hiện tại theo miligiây kể từ EPOCH (u64)
fn current_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Thời gian bị ngược!")
        .as_millis() as u64
}

/// Chờ đến miligiây tiếp theo
fn wait_for_next_ms(last_timestamp: u64) -> u64 {
    let mut timestamp = current_milliseconds();
    while timestamp <= last_timestamp {
        // Rust không cần vòng lặp busy-wait dài, chỉ cần gọi lại
        timestamp = current_milliseconds();
    }
    timestamp
}

/// Sinh ID Snowflake-like (64-bit, có thứ tự)
///
/// Trả về ID dưới dạng u64
pub fn generate_suid_u64(worker_id: u64, datacenter_id: u64) -> u64 {
    // Để an toàn luồng, ta sử dụng một block để giới hạn phạm vi đồng bộ hóa
    // Tuy nhiên, việc sử dụng `AtomicU64` giúp ta tránh `Mutex` nặng nề cho các thao tác đơn giản này.
    
    let mut timestamp = current_milliseconds();
    
    // Đọc trạng thái cũ
    let last_timestamp = LAST_TIMESTAMP.load(Ordering::Relaxed);
    let mut sequence = SEQUENCE.load(Ordering::Relaxed);

    if timestamp < last_timestamp {
        // Lỗi: Thời gian hệ thống bị lùi (quay ngược đồng hồ)
        panic!("Clock moved backwards. Refusing to generate id for {} milliseconds", last_timestamp - timestamp);
    }

    if timestamp == last_timestamp {
        // Cùng miligiây, tăng Sequence
        sequence = (sequence + 1) & MAX_SEQUENCE; 

        if sequence == 0 {
            // Sequence tràn (Overflow), chờ miligiây tiếp theo
            timestamp = wait_for_next_ms(last_timestamp);
        }
    } else {
        // Miligiây mới, reset Sequence
        sequence = 0;
    }

    // Cập nhật trạng thái
    LAST_TIMESTAMP.store(timestamp, Ordering::Relaxed);
    SEQUENCE.store(sequence, Ordering::Relaxed);

    // Ghép ID (Bitwise OR)
    (timestamp - EPOCH) << TIMESTAMP_SHIFT |
    (datacenter_id << (WORKER_ID_BITS + SEQUENCE_BITS)) |
    (worker_id << SEQUENCE_BITS) |
    sequence
}


// ===== Chuyển sang chuỗi ngắn Base64URL (kiểu YouTube) =====

// Engine Base64URL không padding
const BASE64URL_ENGINE: general_purpose::GeneralPurpose = general_purpose::GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    general_purpose::NO_PAD,
);

/// Chuyển u64 sang chuỗi ngắn Base64URL
pub fn to_base64url(num: u64) -> String {
    // Chuyển u64 thành mảng 8 bytes (big-endian)
    let bytes = num.to_be_bytes(); 
    
    // Mã hóa 8 bytes thành Base64URL
    let mut encoded = BASE64URL_ENGINE.encode(&bytes);

    // Cắt bớt phần sau, thường là 11 ký tự như YouTube ID (8 bytes => 11 ký tự Base64)
    if encoded.len() > 11 {
        encoded.truncate(11);
    }
    encoded
}


// ===== Hàm tạo ID chuỗi (Hàm chính) =====

/// Tạo ID chuỗi Snowflake-like Base64URL
///
/// Mặc định dùng worker_id=1, datacenter_id=1
pub fn suid() -> String {
    let id_u64 = generate_suid_u64(1, 1);
    to_base64url(id_u64)
}

pub fn generate_random_hex() -> String {
    // 1. Tạo mảng đệm 16 byte (128 bit)
    let mut bytes = [0u8; 16];
    
    // 2. Điền dữ liệu ngẫu nhiên vào mảng
    // thread_rng() là bộ sinh số ngẫu nhiên an toàn luồng, hiệu năng cao
    thread_rng().fill_bytes(&mut bytes);

    // 3. Chuyển sang chuỗi Hex
    hex::encode(bytes)
}

/// (Tùy chọn) Nếu bạn cần đúng 16 ký tự Hex (tương ứng 8 byte dữ liệu)
pub fn generate_short_hex() -> String {
    let mut bytes = [0u8; 8];
    thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}