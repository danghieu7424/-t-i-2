use leptos::*;
use crate::components::StatCard::StatCard;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Expense {
    pub id: Option<i32>,
    pub user_id: i32,
    pub merchant: String,
    pub bill_date: Option<String>,
    pub amount: f64,
    pub category: String,
    pub is_warning: bool,
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let (show_options, set_show_options) = create_signal(false);

    // 1. Fetch dữ liệu từ API cổng 5000
    let expenses_resource = create_resource(|| (), |_| async move {
        gloo_net::http::Request::get("http://localhost:5000/api/expenses")
            .send()
            .await
            .unwrap()
            .json::<Vec<Expense>>()
            .await
            .unwrap_or_default()
    });

    // 2. Vẽ biểu đồ dựa trên dữ liệu thật
    create_effect(move |_| {
        if let Some(data) = expenses_resource.get() {
            let dien_data: Vec<f64> = data.iter().filter(|e| e.category == "Điện").map(|e| e.amount).collect();
            let nuoc_data: Vec<f64> = data.iter().filter(|e| e.category == "Nước").map(|e| e.amount).collect();
            let nl_data: Vec<f64> = data.iter().filter(|e| e.category == "Nguyên liệu").map(|e| e.amount).collect();

            let dien_json = serde_json::to_string(&dien_data).unwrap_or("[]".to_string());
            let nuoc_json = serde_json::to_string(&nuoc_data).unwrap_or("[]".to_string());
            let nl_json = serde_json::to_string(&nl_data).unwrap_or("[]".to_string());

            let js_code = format!(r#"
                let canvas = document.getElementById('payment-chart-canvas');
                if (canvas) {{
                    let old = Chart.getChart(canvas); if(old) old.destroy();
                    new Chart(canvas, {{
                        type: 'line',
                        data: {{
                            labels: ['T4','T5','T6','T7','T8','T9','T10','T11','T12','T1','T2','T3'],
                            datasets: [
                                {{ label: 'Điện', data: {}, borderColor: '#f1c40f', tension: 0.3, fill: true, backgroundColor: 'rgba(241,196,15,0.1)' }},
                                {{ label: 'Nước', data: {}, borderColor: '#3498db', tension: 0.3, fill: true, backgroundColor: 'rgba(52,152,219,0.1)' }},
                                {{ label: 'Nguyên liệu', data: {}, borderColor: '#2ecc71', tension: 0.3, fill: true, backgroundColor: 'rgba(46,204,113,0.1)' }}
                            ]
                        }},
                        options: {{ 
                            maintainAspectRatio: false,
                            plugins: {{ legend: {{ labels: {{ color: '#f8f9fa', font: {{ size: 13, weight: 'bold' }} }} }} }},
                            scales: {{ 
                                y: {{ ticks: {{ color: '#adb5bd' }}, grid: {{ color: 'rgba(255,255,255,0.05)' }} }},
                                x: {{ ticks: {{ color: '#adb5bd' }}, grid: {{ display: false }} }}
                            }}
                        }}
                    }});
                }}
            "#, dien_json, nuoc_json, nl_json);
            let _ = js_sys::eval(&js_code);
        }
    });

    let on_file_upload = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Some(files) = target.files() {
            if let Some(file) = files.get(0) {
                spawn_local(async move {
                    let window = web_sys::window().expect("no window");
                    let storage = window.local_storage().expect("no storage").expect("storage missing");
                    let user_id = storage.get_item("user_id").unwrap().unwrap_or("1".to_string());
                    let form_data = web_sys::FormData::new().unwrap();
                    form_data.append_with_blob("file", &file).unwrap();
                    form_data.append_with_str("user_id", &user_id).unwrap();

                    let _ = gloo_net::http::Request::post("http://localhost:5000/api/expenses/upload")
                        .body(form_data).expect("body error").send().await;
                    let _ = window.location().reload();
                });
            }
        }
    };

    view! {
        <div class="dashboard">
            <Suspense fallback=move || {
                view! { <p>"Đang tải dữ liệu chi phí..."</p> }
            }>
                {move || {
                    expenses_resource
                        .get()
                        .map(|data| {
                            let total_dien: f64 = data
                                .iter()
                                .filter(|e| e.category == "Điện")
                                .map(|e| e.amount)
                                .sum();
                            let total_nuoc: f64 = data
                                .iter()
                                .filter(|e| e.category == "Nước")
                                .map(|e| e.amount)
                                .sum();
                            let total_nl: f64 = data
                                .iter()
                                .filter(|e| e.category == "Nguyên liệu")
                                .map(|e| e.amount)
                                .sum();

                            view! {
                                <div class="grid-stats">
                                    <StatCard
                                        title="Tiền Điện"
                                        amount=format!("{:.0} VNĐ", total_dien)
                                        percent=12.5
                                        category_slug="dien"
                                    />
                                    <StatCard
                                        title="Tiền Nước"
                                        amount=format!("{:.0} VNĐ", total_nuoc)
                                        percent=-2.1
                                        category_slug="nuoc"
                                    />
                                    <StatCard
                                        title="Nguyên liệu"
                                        amount=format!("{:.0} VNĐ", total_nl)
                                        percent=5.0
                                        category_slug="nguyen-lieu"
                                    />
                                </div>
                            }
                        })
                }}
            </Suspense>

            <div
                class="chart-container"
                style="background: var(--bg-surface); margin-top: 25px; padding: 25px; border-radius: 15px;"
            >
                <h3 style="color: var(--text-main); margin-bottom: 20px;">
                    "Phân tích biến động chi phí thực tế"
                </h3>
                <div style="height: 380px; position: relative;">
                    <canvas id="payment-chart-canvas"></canvas>
                </div>
            </div>

            <div class="fab-container">
                <button
                    class="btn-main-add"
                    on:click=move |_| set_show_options.update(|v| *v = !*v)
                >
                    {move || if show_options.get() { "×" } else { "+" }}
                </button>
                <div class=move || {
                    if show_options.get() { "upload-menu active" } else { "upload-menu" }
                }>
                    <label class="menu-item">
                        <input
                            type="file"
                            accept="image/*"
                            capture="environment"
                            class="hidden"
                            on:change=on_file_upload.clone()
                        />
                        <span class="icon">"📸"</span>
                        <span class="label">"Chụp ảnh"</span>
                    </label>
                    <label class="menu-item">
                        <input
                            type="file"
                            accept="image/*"
                            class="hidden"
                            on:change=on_file_upload
                        />
                        <span class="icon">"📁"</span>
                        <span class="label">"Chọn file"</span>
                    </label>
                </div>
            </div>
        </div>
    }
}