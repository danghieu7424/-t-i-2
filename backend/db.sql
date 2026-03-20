-- 1. Tạo bảng người dùng
CREATE TABLE IF NOT EXISTS users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    google_id VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    picture TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 2. Tạo bảng hóa đơn chi phí
CREATE TABLE IF NOT EXISTS expenses (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id INT NOT NULL,
    merchant VARCHAR(255) NOT NULL,
    bill_date DATE,
    amount DOUBLE NOT NULL,
    category VARCHAR(100) NOT NULL,
    is_warning BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    -- Ràng buộc: Xóa user thì xóa luôn hóa đơn của user đó
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);