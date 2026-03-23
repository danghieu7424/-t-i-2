// src/components/StatCard/mod.rs
use leptos::*;
use crate::store::GlobalState;

#[component]
pub fn StatCard(
    title: String,
    amount: String,
    percent: f64,
    budget: String,
    progress: f64,
    category_slug: String,
) -> impl IntoView {
    let state = use_context::<GlobalState>().expect("No state");
    
    // 1. Khắc phục lỗi E0525: Tạo bản sao cho các biến String để closure có thể chạy nhiều lần
    let update_limit = {
        let category_slug = category_slug.clone(); // Clone lần 1 cho closure cha
        let domain = state.domain.clone();
        
        move |_| {
            let category_slug = category_slug.clone(); // Clone lần 2 cho spawn_local
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

    // Xác định màu sắc thanh Progress theo 3 cấp độ
    let progress_color = if progress >= 120.0 {
        "#ff0000" // Đỏ đậm (Crimson) khi vượt quá 20% chỉ tiêu
    } else if progress >= 100.0 {
        "#ff4d4d" // Đỏ thường khi vừa vượt ngưỡng
    } else if progress >= 80.0 {
        "#f39c12" // Vàng cam khi sắp chạm mốc
    } else {
        "#2ecc71" // Xanh lá khi còn an toàn
    };

    let progress_style = format!("width: {}%; background: {};", 
        if progress > 100.0 { 100.0 } else { progress },
        progress_color
    );
        
    // Thêm class "critical" nếu vượt quá 120%
    let is_critical = progress >= 120.0;
    let card_class = format!(
        "stat-card-v2 category-{} {} {}", 
        category_slug,
        if progress >= 100.0 { "over-budget" } else if progress >= 80.0 { "near-limit" } else { "" },
        if is_critical { "critical-shake" } else { "" } // Hiệu ứng rung nếu quá 20%
    );
        
    let is_up = percent > 0.0; // Xác định tăng hay giảm

    view! {
        <div class=card_class>
            {move || {
                if progress >= 120.0 {
                    view! { <span class="badge-alert critical">"QUÁ 20% HẠN MỨC!"</span> }
                        .into_view()
                } else if progress >= 100.0 {
                    view! { <span class="badge-alert">"VƯỢT HẠN MỨC!"</span> }.into_view()
                } else {
                    view! {}.into_view()
                }
            }} <div class="stat-main-info">
                <p class="label">{title}</p>
                <div class="value-group" style="display: flex; align-items: baseline; gap: 10px;">
                    <h3>{amount} " VNĐ"</h3>
                    // Trong view! của StatCard
                    <span
                        class={ { if is_up { "pct up" } else { "pct down" } } }
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
            </div> <div class="stat-budget-info" on:click=update_limit style="cursor: pointer">
                <div class="budget-header">
                    <p class="label">"Hạn mức kế hoạch"</p>
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