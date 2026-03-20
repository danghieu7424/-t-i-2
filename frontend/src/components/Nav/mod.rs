use leptos::*;

#[component]
pub fn Nav() -> impl IntoView {
    // Lấy thông tin User từ LocalStorage (đã lưu lúc Login thành công)
    let storage = window().local_storage().ok().flatten().expect("Không thể truy cập LocalStorage");
    
    // blind spot: Fallback dữ liệu nếu chưa có thông tin trong máy
    let user_name = storage.get_item("user_name").ok().flatten().unwrap_or("Người dùng".into());
    let user_pic = storage.get_item("user_pic").ok().flatten().unwrap_or("https://ui-avatars.com/api/?name=User".into());

    // Hàm xử lý Đăng xuất
    let logout = move |_| {
        let _ = window().local_storage().unwrap().unwrap().clear();
        let _ = window().location().set_href("/login");
    };

    view! {
        <nav class="top-nav">
            <div class="nav-left">
                // Logo có thể thay bằng ảnh .svg hoặc emoji
                <span class="logo-icon">"📊"</span>
                <span class="logo-text">"AI EXPENSE"</span>
            </div>

            <div class="nav-right">
                <div class="user-info">
                    <span class="user-name">{user_name}</span>
                    <img class="user-avatar" src=user_pic alt="avatar" />
                </div>

                <button class="btn-logout" on:click=logout>
                    <span class="icon">"🚪"</span>
                    "Đăng xuất"
                </button>
            </div>
        </nav>
    }
}