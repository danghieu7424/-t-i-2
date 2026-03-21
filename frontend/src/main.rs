use leptos::*;
use leptos_router::*;
// Giả sử file store của bạn nằm ở src/store.rs
mod store; 
use store::{init_global_state, GlobalState};

pub mod components {
    pub mod login;
    pub mod Dashboard;
    pub mod StatCard;
    pub mod Nav;           
    pub mod UploadModal;   
}

use components::login::Login;
use components::Dashboard::Dashboard;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    // Khởi tạo GlobalState một lần duy nhất tại gốc
    let state = init_global_state();
    provide_context(state);

    view! {
        <Router>
            <main>
                <Routes>
                    <Route path="/login" view=Login />
                    <Route
                        path="/"
                        view=move || {
                            let storage = window().local_storage().ok().flatten();
                            let is_logged_in = storage
                                .and_then(|s| s.get_item("user_id").ok().flatten())
                                .is_some();
                            if is_logged_in {
                                // Kiểm tra login từ context hoặc storage

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