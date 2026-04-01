// src/components/StatCard/mod.rs
use leptos::*;
use crate::store::GlobalState;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn StatCard(
    title: String,
    amount: String,
    percent: f64,
    budget: String,
    progress: f64,
    category_slug: String,
    color: String,
) -> impl IntoView {
    let state = use_context::<GlobalState>().expect("No state");
    
    // THÊM: State chống Spam Click
    let (is_updating, set_is_updating) = create_signal(false);
    
    let update_limit = {
        let category_slug = category_slug.clone(); 
        let domain = state.domain.clone();
        
        move |_| {
            // Khóa không cho bấm nếu đang tải
            if is_updating.get() { return; }

            let val = window().prompt_with_message("Nhập hạn mức mới cho mục này (Ví dụ: 500000):");
            if let Ok(Some(new_limit)) = val {
                let parsed_limit = new_limit.parse::<f64>().unwrap_or(0.0);
                if parsed_limit <= 0.0 { return; } // Validate dữ liệu đầu vào

                let storage = window().local_storage().unwrap().unwrap();
                let uid = storage.get_item("user_id").ok().flatten().unwrap_or_else(|| "1".into());
                // NỀN MÓNG BẢO MẬT: Lấy token để sau này gắn vào Header
                let token = storage.get_item("auth_token").ok().flatten().unwrap_or_default();
                
                let category_slug_clone = category_slug.clone();
                let domain_clone = domain.clone();

                spawn_local(async move {
                    set_is_updating.set(true); // Bật cờ chặn Spam
                    
                    let req = serde_json::json!({
                        "category_slug": category_slug_clone,
                        "amount_limit": parsed_limit,
                        "user_id": uid
                    });
                    
                    let _ = gloo_net::http::Request::post(&format!("{}/api/expenses/budget", domain_clone))
                        .header("Authorization", &format!("Bearer {}", token)) // Chuẩn bị cho JWT
                        .json(&req).unwrap().send().await;
                        
                    // FIX: Bỏ window().location().reload()! 
                    // Thay vào đó, báo cho người dùng biết để họ chỉ cần đổi tab hoặc để tự động refetch từ Dashboard
                    let _ = window().alert_with_message("Đã cập nhật ngân sách thành công! Dữ liệu sẽ tự đồng bộ.");
                    set_is_updating.set(false); // Tắt cờ chặn Spam
                });
            }
        }
    };

    let progress_color = if progress >= 120.0 { "#ff0000" } 
    else if progress >= 100.0 { "#ff4d4d" } 
    else if progress >= 80.0 { "#f39c12" } 
    else { "#2ecc71" };

    let progress_style = format!("width: {}%; background: {}; transition: width 0.3s ease;", 
        if progress > 100.0 { 100.0 } else { progress }, progress_color
    );
        
    let is_critical = progress >= 120.0;
    let card_class = move || format!(
        "stat-card-v2 category-{} {} {} {}", 
        category_slug,
        if progress >= 100.0 { "over-budget" } else if progress >= 80.0 { "near-limit" } else { "" },
        if is_critical { "critical-shake" } else { "" },
        if is_updating.get() { "is-loading" } else { "" } // Thêm hiệu ứng mờ khi đang lưu
    );
        
    let is_up = percent > 0.0; 
    let border_color = if progress >= 100.0 { "#ff4d4d".to_string() } else { color };
    let card_style = format!("border-left: 4px solid {}; position: relative; transition: all 0.3s;", border_color);

    view! {
        <div class=card_class style=card_style>
            <div class="stat-main-info">
                <p class="label">{title}</p>
                <div class="value-group" style="display: flex; align-items: baseline; gap: 10px;">
                    <h3>{amount} " VNĐ"</h3>
                    <span
                        class=if is_up { "pct up" } else { "pct down" }
                        style=format!(
                            "color: {}; font-weight: bold;",
                            if is_up { "#ff4d4d" } else { "#2ecc71" },
                        )
                    >
                        {if is_up { "↑ " } else { "↓ " }}
                        {format!("{:.1}%", percent.abs())}
                        <span style="font-size: 0.7rem; opacity: 0.6; margin-left: 4px;">
                            "so với t.trước"
                        </span>
                    </span>
                </div>
            </div>

            <div
                class="stat-budget-info"
                on:click=update_limit
                style=move || {
                    if is_updating.get() { "cursor: wait; opacity: 0.5" } else { "cursor: pointer" }
                }
            >
                <div class="budget-header">
                    <p class="label">
                        {move || {
                            if is_updating.get() {
                                "Đang cập nhật..."
                            } else {
                                "Hạn mức kế hoạch (Click để đổi)"
                            }
                        }}
                    </p>
                    <span class="budget-percent">{format!("{:.1}%", progress)}</span>
                </div>
                <h4>{budget} " VNĐ"</h4>
                <div class="progress-bar">
                    <div class="progress-fill" style=progress_style></div>
                </div>
            </div>
        </div>
    }
}