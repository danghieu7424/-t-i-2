// src/components/UploadModal/mod.rs
use leptos::*;
use wasm_bindgen::JsCast; 
use wasm_bindgen_futures::spawn_local;
use crate::store::GlobalState; 

#[component]
pub fn UploadModal(
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>
) -> impl IntoView {
    
    let state = use_context::<GlobalState>().expect("No state");
    
    // 🛡️ FIX CHÍ MẠNG E0525: Gói String vào store_value để nó trở thành kiểu Copy
    let api_domain = store_value(state.domain.clone());

    // Trạng thái đang tải ảnh
    let (is_uploading, set_is_uploading) = create_signal(false);
    let (is_dragging, set_is_dragging) = create_signal(false);
    
    let handle_file = move |file: web_sys::File| {
        if is_uploading.get() { return; } // CHẶN SPAM KÉO THẢ

        // Lấy domain ra TRƯỚC KHI bị async move tịch thu
        let domain_url = api_domain.get_value();

        spawn_local(async move {
            set_is_uploading.set(true); // BẬT LOADING

            let window = web_sys::window().expect("no window");
            let storage = window.local_storage().expect("no storage").expect("no storage");
            let user_id = storage.get_item("user_id").unwrap().unwrap_or("1".to_string());
            let token = storage.get_item("auth_token").unwrap().unwrap_or_default();
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            form_data.append_with_str("user_id", &user_id).unwrap();

            let url = format!("{}/api/expenses/upload", domain_url);
            let res = gloo_net::http::Request::post(&url)
                .header("Authorization", &format!("Bearer {}", token))
                .body(form_data).expect("Failed to build body").send().await;
            
            if let Ok(response) = res {
                if response.status() == 401 {
                    set_is_uploading.set(false);
                    let _ = window.alert_with_message("Phiên đăng nhập hết hạn! Vui lòng F5 và đăng nhập lại.");
                    return;
                }
                if response.status() == 200 {
                    let data: serde_json::Value = response.json().await.unwrap();
                    if let Some(job_id) = data["job_id"].as_str() {
                        let job_id = job_id.to_string();
                        
                        // 🛡️ BỔ SUNG LONG POLLING: Đợi AI phân tích xong mới báo cáo
                        loop {
                            let _ = gloo_timers::future::TimeoutFuture::new(2000).await; // Nghỉ 2 giây
                            
                            let status_url = format!("{}/api/expenses/upload/status/{}", domain_url, job_id);
                            if let Ok(status_res) = gloo_net::http::Request::get(&status_url)
                                .header("Authorization", &format!("Bearer {}", token))
                                .send().await 
                            {
                                if let Ok(status_data) = status_res.json::<serde_json::Value>().await {
                                    let state_str = status_data["state"].as_str().unwrap_or("");
                                    
                                    if state_str == "Completed" {
                                        set_is_uploading.set(false); 
                                        set_is_open.set(false); // ĐÓNG MODAL
                                        let _ = window.alert_with_message("✅ AI đã xử lý xong!");
                                        let _ = window.location().reload(); // F5 load lại trang để thấy dữ liệu
                                        break;
                                    } else if state_str == "Failed" {
                                        set_is_uploading.set(false);
                                        let err = status_data["error"].as_str().unwrap_or("Lỗi không xác định");
                                        let _ = window.alert_with_message(&format!("❌ Lỗi AI: {}", err));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    set_is_uploading.set(false);
                    let _ = window.alert_with_message("Lỗi đẩy hóa đơn lên Server!");
                }
            } else {
                set_is_uploading.set(false);
                let _ = window.alert_with_message("Lỗi kết nối mạng!");
            }
        });
    };

    let on_file_input_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Some(files) = target.files() {
            if let Some(file) = files.get(0) { handle_file(file); }
        }
        
        // 🛡️ FIX LỖI ĐƠ: Xóa sạch value của thẻ input. 
        target.set_value("");
    };
    
    let on_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
        if is_uploading.get() { return; } // CHẶN DROP KHI ĐANG LOADING
        
        let web_ev: &web_sys::DragEvent = ev.as_ref();
        if let Some(dt) = web_ev.data_transfer() {
            if let Some(files) = dt.files() {
                // Sửa thành web_sys::FileList để lấy item chuẩn xác
                let file_list: web_sys::FileList = files;
                if let Some(file) = file_list.get(0) { handle_file(file); }
            }
        }
    };

    view! {
        <div
            class=move || {
                if is_open.get() { "upload-modal-backdrop active" } else { "upload-modal-backdrop" }
            }
            // Không cho tắt modal khi đang tải
            on:click=move |_| {
                if !is_uploading.get() {
                    set_is_open.set(false)
                }
            }
        >
            <div
                class=move || {
                    if is_dragging.get() {
                        "upload-modal-content dragging"
                    } else {
                        "upload-modal-content"
                    }
                }
                on:click=|ev| ev.stop_propagation()
                on:dragover=move |ev| {
                    ev.prevent_default();
                    if !is_uploading.get() {
                        set_is_dragging.set(true);
                    }
                }
                on:dragleave=move |ev| {
                    ev.prevent_default();
                    set_is_dragging.set(false);
                }
                on:drop=on_drop
            >
                // HIỂN THỊ LOADING UI
                <Show when=move || is_uploading.get()>
                    <div style="position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.8); z-index: 10; display: flex; flex-direction: column; justify-content: center; align-items: center; border-radius: 12px;">
                        <div class="spinner"></div>
                        <p style="color: #2ecc71; font-weight: bold; margin-top: 15px;">
                            "AI Đang phân tích..."
                        </p>
                    </div>
                </Show>

                <h3>"Thêm hóa đơn chi phí AI"</h3>
                <p class="subtitle">"Chụp ảnh, chọn file hoặc kéo thả vào đây"</p>

                <div
                    class="upload-options-grid"
                    style=move || {
                        if is_uploading.get() { "opacity: 0.5; pointer-events: none;" } else { "" }
                    }
                >
                    <label class="upload-item">
                        <input
                            type="file"
                            accept="image/*"
                            capture="environment"
                            class="hidden"
                            on:change=on_file_input_change.clone()
                            disabled=move || is_uploading.get()
                        />
                        <span class="icon">"📸"</span>
                        <span class="label">"Chụp hóa đơn"</span>
                    </label>

                    <label class="upload-item">
                        <input
                            type="file"
                            accept="image/*"
                            class="hidden"
                            on:change=on_file_input_change
                            disabled=move || is_uploading.get()
                        />
                        <span class="icon">"📁"</span>
                        <span class="label">"Chọn từ máy"</span>
                    </label>
                </div>

                <div
                    class="drag-drop-zone"
                    style=move || if is_uploading.get() { "opacity: 0.5" } else { "" }
                >
                    <span class="icon">"⬇️"</span>
                    <span class="label">"Kéo thả ảnh vào đây"</span>
                </div>

                <button
                    class="btn-close-modal"
                    disabled=move || is_uploading.get()
                    style=move || {
                        if is_uploading.get() { "opacity: 0.5; cursor: not-allowed;" } else { "" }
                    }
                    on:click=move |_| set_is_open.set(false)
                >
                    "Đóng"
                </button>
            </div>
        </div>
    }
}