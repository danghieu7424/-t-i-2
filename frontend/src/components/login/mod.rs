// src/components/Login/mod.rs
use leptos::*;
use crate::store::GlobalState;

#[component]
pub fn Login() -> impl IntoView {
    let state = use_context::<GlobalState>().expect("Phải có GlobalState");
    let domain = state.domain.clone();

    create_effect(move |_| {
        let client_id = "382574203305-ud2irfgr6bl243mmq6le9l67e29ire7d.apps.googleusercontent.com";
        let api_url = format!("{}/api/auth/google", domain); 

        let js = format!(r#"
            window.handleLogin = (res) => {{
                // BẬT OVERLAY BẰNG JS GỐC
                document.getElementById('login-loading').style.display = 'flex';
                
                fetch('{}', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ token: res.credential }})
                }})
                .then(r => r.json())
                .then(user => {{
                    localStorage.setItem('user_id', user.user_id);
                    localStorage.setItem('user_name', user.name);
                    localStorage.setItem('user_pic', user.picture);
                    window.location.href = '/'; 
                }})
                .catch(err => {{
                    document.getElementById('login-loading').style.display = 'none';
                    alert("Lỗi đăng nhập!");
                }});
            }};
            google.accounts.id.initialize({{ client_id: "{}", callback: window.handleLogin }});
            google.accounts.id.renderButton(document.getElementById("google-btn-slot"), {{
                theme: "filled_blue", size: "large", shape: "pill", width: "300"
            }});
        "#, api_url, client_id);
        let _ = js_sys::eval(&js);
    });

    view! {
        <div class="login-screen">
            // LỚP PHỦ LOADING BỊ ẨN MẶC ĐỊNH
            <div id="login-loading" class="global-loading-overlay" style="display: none;">
                <div class="spinner"></div>
                <h2>"Đang xác thực bảo mật..."</h2>
            </div>

            <div class="login-card">
                <div class="logo">"📊"</div>
                <h1>"AI Expense Manager"</h1>
                <div
                    id="google-btn-slot"
                    style="display: flex; justify-content: center; margin: 20px 0;"
                ></div>
            </div>
        </div>
    }
}