use leptos::*;
use crate::components::Nav::Nav;
use crate::components::StatCard::StatCard;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use crate::store::GlobalState;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DashboardData {
    pub month_labels: Vec<String>,
    pub dien_series: Vec<f64>,
    pub nuoc_series: Vec<f64>,
    pub nl_series: Vec<f64>,
    pub stats: Vec<StatItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatItem {
    pub title: String,
    pub amount: f64,
    pub percent: f64,
    pub budget: f64, // THÊM TRƯỜNG NÀY
    pub slug: String,
}

fn format_thousands(n: f64) -> String {
    let s = format!("{:.0}", n);
    let bytes = s.as_bytes();
    let mut result = String::new();
    let len = bytes.len();
    for (i, &byte) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push('.'); }
        result.push(byte as char);
    }
    result
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let state = use_context::<GlobalState>().expect("Phải có GlobalState");
    let domain = state.domain.clone(); 
    
    let (is_dragging, set_is_dragging) = create_signal(false);

    // 1. Dùng create_resource với domain từ context
   let dash_resource = create_resource(|| (), move |_| {
        let url = format!("{}/api/expenses", domain);
        async move {
            gloo_net::http::Request::get(&url)
                .send().await.unwrap()
                .json::<DashboardData>().await.unwrap_or_default()
        }
    });

    let upload_domain = state.domain.clone();
    let upload_action = move |file: web_sys::File| {
        let url = format!("{}/api/expenses/upload", upload_domain);
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            
            // FIX LỖI E0593: unwrap_or_else cần nhận 1 đối số lỗi (thường dùng |_|)
            let user_id = storage.get_item("user_id")
                .unwrap_or(None) 
                .unwrap_or_else(|| "1".to_string());
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            form_data.append_with_str("user_id", &user_id).unwrap();

            let _ = gloo_net::http::Request::post(&url)
                .body(form_data).expect("Failed").send().await;
            let _ = window.location().reload();
        });
    };

    // Tạo các wrapper để dùng trong view mà không bị move mất gốc
    let on_file_change = {
        let upload = upload_action.clone();
        move |ev: ev::Event| {
            let target = event_target::<web_sys::HtmlInputElement>(&ev);
            if let Some(files) = target.files() {
                if let Some(file) = files.get(0) { upload(file); }
            }
        }
    };

    let on_drop = {
        let upload = upload_action.clone();
        move |ev: ev::DragEvent| {
            ev.prevent_default();
            set_is_dragging.set(false);
            let web_ev: &web_sys::DragEvent = ev.as_ref();
            if let Some(dt) = web_ev.data_transfer() {
                if let Some(files) = dt.files() {
                    let file_list: web_sys::FileList = files;
                    if let Some(file) = file_list.get(0) { upload(file); }
                }
            }
        }
    };

    // Logic vẽ biểu đồ (Giữ nguyên)
    create_effect(move |_| {
        if let Some(d) = dash_resource.get() {
            let js_code = format!(r#"
                let canvas = document.getElementById('payment-chart-canvas');
                if (canvas) {{
                    let old = Chart.getChart(canvas); if(old) old.destroy();
                    new Chart(canvas, {{
                        type: 'line',
                        data: {{
                            labels: {:?},
                            datasets: [
                                {{ label: 'Điện', data: {:?}, borderColor: '#f1c40f', tension: 0.3, fill: true, backgroundColor: 'rgba(241,196,15,0.1)' }},
                                {{ label: 'Nước', data: {:?}, borderColor: '#3498db', tension: 0.3, fill: true, backgroundColor: 'rgba(52,152,219,0.1)' }},
                                {{ label: 'Nguyên liệu', data: {:?}, borderColor: '#2ecc71', tension: 0.3, fill: true, backgroundColor: 'rgba(46,204,113,0.1)' }}
                            ]
                        }},
                        options: {{ maintainAspectRatio: false, plugins: {{ legend: {{ labels: {{ color: '#fff' }} }} }} }}
                    }});
                }}
            "#, d.month_labels, d.dien_series, d.nuoc_series, d.nl_series);
            let _ = js_sys::eval(&js_code);
        }
    });

    view! {
        <div class="dashboard-wrapper">
            <Nav />
            <div class="dashboard-container">
                <div class="main-content">
                    <div class="chart-container">
                        <h3>"Biến động chi phí thực tế (12 tháng)"</h3>
                        <div style="height: 380px;">
                            <canvas id="payment-chart-canvas"></canvas>
                        </div>
                    </div>

                    <Suspense fallback=|| {
                        view! { <div class="loading">"Đang nạp..."</div> }
                    }>
                        {move || {
                            dash_resource
                                .get()
                                .map(|d| {
                                    view! {
                                        <div class="stats-list-vertical">
                                            {d
                                                .stats
                                                .into_iter()
                                                .map(|s| {
                                                    let progress = if s.budget > 0.0 {
                                                        (s.amount / s.budget) * 100.0
                                                    } else {
                                                        0.0
                                                    };
                                                    // Tính toán % progress thực tế
                                                    view! {
                                                        <StatCard
                                                            title=s.title
                                                            amount=format_thousands(s.amount)
                                                            percent=s.percent
                                                            budget=format_thousands(s.budget)
                                                            // Truyền progress thực tế vào
                                                            progress=progress
                                                            category_slug=s.slug
                                                        />
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    }
                                })
                        }}
                    </Suspense>
                </div>

                <aside class="side-panel">
                    <div class="upload-widget">
                        <h3>"Thêm hóa đơn mới"</h3>
                        <div
                            class=move || {
                                if is_dragging.get() { "drop-zone dragging" } else { "drop-zone" }
                            }
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
                            <span class="icon">"📄"</span>
                            <span class="text">"Kéo thả hóa đơn vào đây"</span>
                            <div class="action-buttons">
                                <label class="btn-action">
                                    <input
                                        type="file"
                                        accept="image/*"
                                        capture="environment"
                                        class="hidden"
                                        on:change=on_file_change.clone()
                                    />
                                    "📸 Chụp ảnh"
                                </label>
                                <label class="btn-action">
                                    <input
                                        type="file"
                                        accept="image/*"
                                        class="hidden"
                                        on:change=on_file_change
                                    />
                                    "📁 Chọn file"
                                </label>
                            </div>
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    }.into_view()
}