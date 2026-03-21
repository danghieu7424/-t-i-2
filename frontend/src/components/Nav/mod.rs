use leptos::*;
use crate::store::GlobalState;

#[component]
pub fn Nav() -> impl IntoView {
    // Lấy thông tin User từ LocalStorage (đã lưu lúc Login thành công)
   let state = use_context::<GlobalState>().expect("No state");
    
    // Nếu Hieu đã cập nhật state.user lúc login, ta dùng ở đây:
    // let user = state.user.get(); 
    
    // Tạm thời lấy từ storage nhưng Hieu có thể bind vào Signal sau
    let storage = window().local_storage().ok().flatten().unwrap();
    let name = storage.get_item("user_name").ok().flatten().unwrap_or("User".into());
    let pic = storage.get_item("user_pic").ok().flatten().unwrap_or_default();

    // Hàm xử lý Đăng xuất
    let logout = move |_| {
        let _ = window().local_storage().unwrap().unwrap().clear();
        let _ = window().location().set_href("/login");
    };

    view! {
        <nav class="top-nav">
            <div class="nav-left">
                // Logo có thể thay bằng ảnh .svg hoặc emoji
                <span class="logo-icon">""</span>
                <span class="logo-text">"AI EXPENSE"</span>
            </div>

            <div class="nav-right">
                <div class="user-info">
                    <span class="user-name">{name}</span>
                    <img class="user-avatar" src=pic alt="avatar" />
                </div>

                <button class="btn-logout" on:click=logout>
                    "Đăng xuất"
                </button>
            </div>
        </nav>
    }
}