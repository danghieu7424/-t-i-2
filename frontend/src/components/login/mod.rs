// src/components/login/mod.rs
use leptos::*;

#[component]
pub fn Login() -> impl IntoView {
    create_effect(move |_| {
        let client_id = "382574203305-ud2irfgr6bl243mmq6le9l67e29ire7d.apps.googleusercontent.com";
        let js = format!(r#"
            window.handleLogin = (res) => {{
                fetch('http://localhost:5000/api/auth/google', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ token: res.credential }})
                }})
                .then(r => r.json())
                .then(user => {{
                    // Lưu thông tin vào máy để Dashboard lấy ra dùng
                    localStorage.setItem('user_id', user.user_id);
                    localStorage.setItem('user_name', user.name);
                    localStorage.setItem('user_pic', user.picture);
                    // Đăng nhập xong thì bay vào Dashboard
                    window.location.href = '/'; 
                }});
            }};
            google.accounts.id.initialize({{ client_id: "{}", callback: window.handleLogin }});
            // ID này phải khớp với div bên dưới: "google-btn-slot"
            google.accounts.id.renderButton(document.getElementById("google-btn-slot"), {{
                theme: "filled_blue", size: "large", shape: "pill", width: "300"
            }});
        "#, client_id);
        let _ = js_sys::eval(&js);
    });

    view! {
        <div class="login-screen">
            <div class="login-card">
                <div class="logo">"📊"</div>
                <h1>"AI Expense Manager"</h1>
                <p>"Quản lý chi tiết hóa đơn bằng trí tuệ nhân tạo"</p>

                // CÁI NÀY LÀ NƠI NÚT GOOGLE SẼ HIỆN LÊN
                <div
                    id="google-btn-slot"
                    style="display: flex; justify-content: center; margin: 20px 0;"
                ></div>

                <p class="footer">"Sử dụng Google để bảo mật dữ liệu của bạn"</p>
            </div>
        </div>
    }
}