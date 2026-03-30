-- 1. Bảng người dùng (Dùng id từ suid.rs)
CREATE TABLE users (
    id VARCHAR(20) PRIMARY KEY, -- Base64 ID từ suid.rs
    google_id VARCHAR(255) UNIQUE NOT NULL, -- Mã 'sub' của Google
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    picture TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 2. Bảng danh mục chi phí (Để quản lý slug và icon)
CREATE TABLE categories (
    slug VARCHAR(50) PRIMARY KEY, -- 'dien', 'nuoc', 'nguyen-lieu'
    display_name VARCHAR(100) NOT NULL,
    icon VARCHAR(10) -- '⚡', '💧', '📦'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 3. Bảng hạn mức (Budgets) theo từng User và Danh mục
CREATE TABLE budgets (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id VARCHAR(20),
    category_slug VARCHAR(50),
    amount_limit DOUBLE NOT NULL DEFAULT 0,
    month_year DATE NOT NULL, -- Lưu ngày đầu tháng để quản lý theo tháng
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_slug) REFERENCES categories(slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 4. Bảng chi phí (Expenses)
CREATE TABLE expenses (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id VARCHAR(20),
    merchant VARCHAR(255),
    amount DOUBLE NOT NULL,
    bill_date DATE,
    category_slug VARCHAR(50),
    is_warning BOOLEAN DEFAULT FALSE,
    raw_ai_data TEXT, -- Lưu JSON gốc từ Gemini để đối soát
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_slug) REFERENCES categories(slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Dữ liệu mẫu cho Categories
INSERT INTO categories (slug, display_name, icon) VALUES 
('dien', 'Tiền Điện', '⚡'),
('nuoc', 'Tiền Nước', '💧'),
('nguyen-lieu', 'Nguyên liệu', '📦'),
('khac', 'Khác', '❓');

ALTER TABLE budgets ADD UNIQUE KEY `user_category_idx` (`user_id`, `category_slug`);

-- Thêm user_id vào bảng categories
ALTER TABLE categories ADD COLUMN user_id VARCHAR(255) DEFAULT 'system';

-- Cập nhật UNIQUE KEY để 1 user không bị trùng tên danh mục
ALTER TABLE categories ADD UNIQUE KEY `user_slug_idx` (`user_id`, `slug`);