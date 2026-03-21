// src/components/StatCard/mod.rs
use leptos::*;

#[component]
pub fn StatCard(
    title: String,
    amount: String,
    percent: f64,
    budget: String,
    progress: f64, // Thêm prop để nhận % thực tế
    category_slug: String,
) -> impl IntoView {
    let is_up = percent > 0.0;
    // Cảnh báo đỏ nếu tiêu quá 90% ngân sách
    let progress_style = format!("width: {}%; background: {}", 
        if progress > 100.0 { 100.0 } else { progress },
        if progress > 90.0 { "#ff4d4d" } else { "var(--accent-color, #4caf50)" }
    );
    
    view! {
        <div class=format!("stat-card-v2 category-{}", category_slug)>
            <div class="stat-main-info">
                <p class="label">{title}</p>
                <div class="value-group">
                    <h3>{amount} " VNĐ"</h3>
                    <span class=if is_up {
                        "pct up"
                    } else {
                        "pct down"
                    }>
                        {if is_up { "▲ " } else { "▼ " }} {format!("{:.2}%", percent.abs())}
                    </span>
                </div>
                <p class="sub-label">"So với tháng trước"</p>
            </div>

            <div class="stat-budget-info">
                <p class="label">"Hạn mức kế hoạch"</p>
                <h4>{budget} " VNĐ"</h4>
                <div class="progress-bar">
                    <div class="progress-fill" style=progress_style></div>
                </div>
                <p class="progress-text">{format!("{:.1}%", progress)}</p>
            </div>
        </div>
    }
}