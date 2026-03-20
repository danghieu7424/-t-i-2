// src/components/StatCard/mod.rs
use leptos::*;

#[component]
pub fn StatCard(
    #[prop(into)] title: String,
    #[prop(into)] amount: String,
    percent: f64,
    #[prop(into)] category_slug: String, // Ví dụ: "dien", "nuoc"
) -> impl IntoView {
    let is_up = percent > 0.0;
    let status_class = if is_up { "status up" } else { "status down" };
    let card_class = format!("stat-card category-{}", category_slug);

    view! {
        <div class=card_class>
            <p class="title">{title}</p>
            <h3>{amount} " VNĐ"</h3>
            <div class=status_class>
                {if is_up { "▲ " } else { "▼ " }} {format!("{:.2}%", percent.abs())}
            </div>
        </div>
    }
}