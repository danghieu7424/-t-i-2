use leptos::*;

#[component]
fn AddInvoiceModal() -> impl IntoView {
    view! {
        <div class="modal">
            <h3>"Tải lên hóa đơn AI"</h3>
            <div class="upload-options">
                // Lựa chọn 1: Chụp ảnh (Dùng camera trên điện thoại)
                <label class="opt-card">
                    <input type="file" accept="image/*" capture="environment" class="hidden" />
                    <div class="icon">"📸"</div>
                    <span>"Chụp ảnh trực tiếp"</span>
                </label>

                // Lựa chọn 2: Chọn từ máy
                <label class="opt-card">
                    <input type="file" accept="image/*" class="hidden" />
                    <div class="icon">"🖼️"</div>
                    <span>"Chọn từ thư viện"</span>
                </label>
            </div>
        </div>
    }
}