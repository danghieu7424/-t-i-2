use leptos::*;
use wasm_bindgen::JsCast; 
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn UploadModal(
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>
) -> impl IntoView {
    
    let handle_file = move |file: web_sys::File| {
        spawn_local(async move {
            set_is_open.set(false);
            let window = web_sys::window().expect("no window");
            let storage = window.local_storage().expect("no storage").expect("no storage");
            let user_id = storage.get_item("user_id").unwrap().unwrap_or("1".to_string());
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            form_data.append_with_str("user_id", &user_id).unwrap();

            let _ = gloo_net::http::Request::post("http://localhost:5000/api/expenses/upload")
                .body(form_data).expect("Failed to build body").send().await;
            
            let _ = window.location().reload();
        });
    };

    let on_file_input_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Some(files) = target.files() {
            if let Some(file) = files.get(0) {
                handle_file(file);
            }
        }
    };

    let (is_dragging, set_is_dragging) = create_signal(false);
    
    let on_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
        
        let web_ev: &web_sys::DragEvent = ev.as_ref();
        
        if let Some(dt) = web_ev.data_transfer() {
            if let Some(files) = dt.files() {
                let file_list: web_sys::FileList = files;
                if let Some(file) = file_list.get(0) {
                    handle_file(file);
                }
            }
        }
    };

    view! {
        <div
            class=move || {
                if is_open.get() { "upload-modal-backdrop active" } else { "upload-modal-backdrop" }
            }
            on:click=move |_| set_is_open.set(false)
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
                // THÊM move VÀO ĐÂY ĐỂ FIX LỖI E0373
                on:dragover=move |ev| {
                    ev.prevent_default();
                    set_is_dragging.set(true);
                }
                on:dragleave=move |ev| {
                    ev.prevent_default();
                    set_is_dragging.set(false);
                }
                on:drop=on_drop
            >
                <h3>"Thêm hóa đơn chi phí AI"</h3>
                <p class="subtitle">"Chụp ảnh, chọn file hoặc kéo thả vào đây"</p>

                <div class="upload-options-grid">
                    <label class="upload-item">
                        <input
                            type="file"
                            accept="image/*"
                            capture="environment"
                            class="hidden"
                            on:change=on_file_input_change.clone()
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
                        />
                        <span class="icon">"📁"</span>
                        <span class="label">"Chọn từ máy"</span>
                    </label>
                </div>

                <div class="drag-drop-zone">
                    <span class="icon">"⬇️"</span>
                    <span class="label">"Kéo thả ảnh vào đây"</span>
                </div>

                <button class="btn-close-modal" on:click=move |_| set_is_open.set(false)>
                    "Đóng"
                </button>
            </div>
        </div>
    }.into_view()
}