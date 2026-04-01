// src/components/StatCard/mod.rs
use leptos::*;
use crate::store::GlobalState;
use wasm_bindgen_futures::spawn_local; // Đảm bảo đã import để dùng cho update_limit

#[component]
pub fn StatCard(
    title: String,
    amount: String,
    percent: f64,
    budget: String,
    progress: f64,
    category_slug: String,
    color: String, // Tham số màu động được truyền từ Dashboard sang
) -> impl IntoView {
    let state = use_context::<GlobalState>().expect("No state");
    
    // 1. Phục hồi nguyên vẹn tính năng nhập ngân sách của bạn
    let update_limit = {
        let category_slug = category_slug.clone(); 
        let domain = state.domain.clone();
        
        move |_| {
            let category_slug = category_slug.clone(); 
            let domain = domain.clone();
            
            let val = window().prompt_with_message("Nhập hạn mức mới cho mục này:").unwrap();
            
            if let Some(new_limit) = val {
                let storage = window().local_storage().unwrap().unwrap();
                let uid = storage.get_item("user_id").ok().flatten().unwrap_or_else(|| "1".into());
                
                spawn_local(async move {
                    let req = serde_json::json!({
                        "category_slug": category_slug,
                        "amount_limit": new_limit.parse::<f64>().unwrap_or(0.0),
                        "user_id": uid
                    });
                    
                    let _ = gloo_net::http::Request::post(&format!("{}/api/expenses/budget", domain))
                        .json(&req).unwrap().send().await;
                        
                    let _ = window().location().reload();
                });
            }
        }
    };

    // 2. Xác định màu sắc thanh Progress theo 3 cấp độ (Logic cũ của bạn)
    let progress_color = if progress >= 120.0 {
        "#ff0000" // Đỏ đậm (Crimson) khi vượt quá 20% chỉ tiêu
    } else if progress >= 100.0 {
        "#ff4d4d" // Đỏ thường khi vừa vượt ngưỡng
    } else if progress >= 80.0 {
        "#f39c12" // Vàng cam khi sắp chạm mốc
    } else {
        "#2ecc71" // Xanh lá khi còn an toàn
    };

    let progress_style = format!("width: {}%; background: {}; transition: width 0.3s ease;", 
        if progress > 100.0 { 100.0 } else { progress },
        progress_color
    );
        
    // 3. Phục hồi các class CSS và thêm hiệu ứng viền màu đồng bộ Chart
    let is_critical = progress >= 120.0;
    let card_class = format!(
        "stat-card-v2 category-{} {} {}", 
        category_slug,
        if progress >= 100.0 { "over-budget" } else if progress >= 80.0 { "near-limit" } else { "" },
        if is_critical { "critical-shake" } else { "" } 
    );
        
    let is_up = percent > 0.0; 

    // Ép viền thẻ thành màu đỏ nếu vượt ngân sách, nếu an toàn thì dùng màu đặc trưng của category
    let border_color = if progress >= 100.0 { "#ff4d4d".to_string() } else { color };
    let card_style = format!("border-left: 4px solid {}; position: relative;", border_color);

    view! {
        <div class=card_class style=card_style>
            {move || {
                if progress >= 120.0 {
                    view! {
                        <span
                            class="badge-alert critical"
                            style="position: absolute; top: -10px; right: 10px;"
                        >
                            "QUÁ 20% HẠN MỨC!"
                        </span>
                    }
                        .into_view()
                } else if progress >= 100.0 {
                    view! {
                        <span
                            class="badge-alert"
                            style="position: absolute; top: -10px; right: 10px;"
                        >
                            "VƯỢT HẠN MỨC!"
                        </span>
                    }
                        .into_view()
                } else {
                    view! {}.into_view()
                }
            }}

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

            <div class="stat-budget-info" on:click=update_limit style="cursor: pointer">
                <div class="budget-header">
                    <p class="label">"Hạn mức kế hoạch (Click để đổi)"</p>
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