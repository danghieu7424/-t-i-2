// src/components/UploadModal/mod.rs
use leptos::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn UploadModal(
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>
) -> impl IntoView {
    
    // THÊM: Trạng thái đang tải ảnh
    let (is_uploading, set_is_uploading) = create_signal(false);
    let (is_dragging, set_is_dragging) = create_signal(false);
    
    let handle_file = move |file: web_sys::File| {
        if is_uploading.get() { return; } // CHẶN SPAM KÉO THẢ

        spawn_local(async move {
            set_is_uploading.set(true); // BẬT LOADING

            let window = web_sys::window().expect("no window");
            let storage = window.local_storage().expect("no storage").expect("no storage");
            let user_id = storage.get_item("user_id").unwrap().unwrap_or("1".to_string());
            let token = storage.get_item("auth_token").unwrap().unwrap_or_default();
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            form_data.append_with_str("user_id", &user_id).unwrap();

            let res = gloo_net::http::Request::post("http://localhost:5000/api/expenses/upload")
                .header("Authorization", &format!("Bearer {}", token))
                .body(form_data).expect("Failed to build body").send().await;
            
            set_is_uploading.set(false); // TẮT LOADING
            set_is_open.set(false); // CHỈ ĐÓNG MODAL KHI THÀNH CÔNG
            
            if res.is_ok() {
                let _ = window.location().reload(); // (Tạm thời giữ reload ở đây, sau này thay bằng refetch)
            } else {
                let _ = window.alert_with_message("Lỗi tải hóa đơn!");
            }
        });
    };

    let on_file_input_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Some(files) = target.files() {
            if let Some(file) = files.get(0) { handle_file(file); }
        }
    };
    
    let on_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
        if is_uploading.get() { return; } // CHẶN DROP KHI ĐANG LOADING
        
        let web_ev: &web_sys::DragEvent = ev.as_ref();
        if let Some(dt) = web_ev.data_transfer() {
            if let Some(files) = dt.files() {
                if let Some(file) = files.get(0) { handle_file(file); }
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