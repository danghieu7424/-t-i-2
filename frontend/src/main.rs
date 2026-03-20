use leptos::*;
use leptos_router::*;

// 1. KHAI BÁO MODULE ĐỂ RUST TÌM THẤY CODE
pub mod components {
    pub mod login;
    pub mod Dashboard;
    pub mod StatCard;
    pub mod Nav;           // Thêm dòng này
    pub mod UploadModal;   // Thêm dòng này
}

// 2. IMPORT CÁC COMPONENT SAU KHI ĐÃ KHAI BÁO MOD
use components::login::Login;
use components::Dashboard::Dashboard;

fn main() {
    // Kích hoạt logging để debug
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <main>
                <Routes>
                    // Trang Login
                    <Route path="/login" view=Login />

                    // Dashboard được bảo vệ
                    <Route
                        path="/"
                        view=move || {
                            let storage = window().local_storage().ok().flatten();
                            let is_logged_in = storage
                                .and_then(|s| s.get_item("user_id").ok().flatten())
                                .is_some();
                            if is_logged_in {

                                // Ép kiểu IntoView để sửa lỗi type annotations needed
                                view! { <Dashboard /> }
                                    .into_view()
                            } else {
                                view! { <Redirect path="/login" /> }.into_view()
                            }
                        }
                    />
                </Routes>
            </main>
        </Router>
    }
}