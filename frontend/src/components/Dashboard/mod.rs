// src/components/Dashboard/mod.rs
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
    pub budget: f64, // FIX: Đã thêm budget vào đây
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

    // Sửa tương tự cho dash_resource để lọc đúng dữ liệu của User đó
    let dash_resource = create_resource(|| (), move |_| {
        let storage = window().local_storage().unwrap().unwrap();
        let user_id = storage.get_item("user_id")
            .ok()
            .flatten()
            .unwrap_or_else(|| "1".to_string());
        
        // Bỏ dấu & ở cuối chuỗi format
        let url = format!("{}/api/expenses?user_id={}", domain, user_id);   
        async move {
            gloo_net::http::Request::get(&url)
                .send().await.unwrap()
                .json::<DashboardData>().await.unwrap_or_default()
        }
    });

    let upload_domain = state.domain.clone();
    // Trong Dashboard component
    let upload_action = move |file: web_sys::File| {
        let url = format!("{}/api/expenses/upload", upload_domain);
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            
            // SỬA: Lấy SUID chuẩn từ storage. Nếu không có mới lấy "1"
            let user_id = storage.get_item("user_id")
                .ok()
                .flatten()
                .unwrap_or_else(|| "1".to_string());
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            
            // QUAN TRỌNG: Gửi SUID chuỗi lên field "user_id"
            form_data.append_with_str("user_id", &user_id).unwrap(); 

            let _ = gloo_net::http::Request::post(&url)
                .body(form_data).expect("Failed").send().await;
            let _ = window.location().reload();
        });
    };

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
                    // TỔNG TIỀN CHI TRONG THÁNG (Vùng tóm tắt trên biểu đồ)
                    <Suspense fallback=|| {
                        view! { <div>"..."</div> }
                    }>
                        {move || {
                            dash_resource
                                .get()
                                .map(|d| {
                                    let total_this_month: f64 = d
                                        .stats
                                        .iter()
                                        .map(|s| s.amount)
                                        .sum();
                                    let total_budget: f64 = d.stats.iter().map(|s| s.budget).sum();
                                    view! {
                                        <Suspense fallback=|| {
                                            view! { <div class="summary-loading">"..."</div> }
                                        }>
                                            {move || {
                                                dash_resource
                                                    .get()
                                                    .map(|d| {
                                                        let current_total: f64 = d
                                                            .stats
                                                            .iter()
                                                            .map(|s| s.amount)
                                                            .sum();
                                                        let prev_total_dien = d
                                                            .dien_series
                                                            .get(d.dien_series.len().saturating_sub(2))
                                                            .unwrap_or(&0.0);
                                                        let prev_total_nuoc = d
                                                            .nuoc_series
                                                            .get(d.nuoc_series.len().saturating_sub(2))
                                                            .unwrap_or(&0.0);
                                                        let prev_total_nl = d
                                                            .nl_series
                                                            .get(d.nl_series.len().saturating_sub(2))
                                                            .unwrap_or(&0.0);
                                                        let last_month_total = prev_total_dien + prev_total_nuoc
                                                            + prev_total_nl;
                                                        let total_diff_pct = if last_month_total > 0.0 {
                                                            ((current_total - last_month_total) / last_month_total)
                                                                * 100.0
                                                        } else {
                                                            0.0
                                                        };
                                                        let is_total_up = total_diff_pct > 0.0;
                                                        // 1. Tính tổng tháng này (Tháng cuối cùng trong series)

                                                        // 2. Tính tổng tháng trước (Cần lấy từ series dữ liệu gốc)
                                                        // Giả sử series có 12 tháng, index 11 là hiện tại, index 10 là tháng trước

                                                        // 3. Tính % tăng trưởng tổng

                                                        view! {
                                                            <div class="month-summary-header">
                                                                <div class="summary-item">
                                                                    <span class="label">"Tổng chi tháng này"</span>
                                                                    <div class="value-row">
                                                                        <h2 class="total-value">
                                                                            {format_thousands(current_total)} " VNĐ"
                                                                        </h2>
                                                                        <span class=if is_total_up {
                                                                            "pct-badge up"
                                                                        } else {
                                                                            "pct-badge down"
                                                                        }>
                                                                            {if is_total_up { "↑ " } else { "↓ " }}
                                                                            {format!("{:.1}%", total_diff_pct.abs())}
                                                                        </span>
                                                                    </div>
                                                                    <p class="prev-label">
                                                                        "Tháng trước: " {format_thousands(last_month_total)}
                                                                        " VNĐ"
                                                                    </p>
                                                                </div>

                                                                <div class="summary-item divider"></div>

                                                                <div class="summary-item">
                                                                    <span class="label">"Ngân sách kế hoạch"</span>
                                                                    <h2 class="budget-value">
                                                                        {format_thousands(d.stats.iter().map(|s| s.budget).sum())}
                                                                        " VNĐ"
                                                                    </h2>
                                                                </div>
                                                            </div>
                                                        }
                                                    })
                                            }}
                                        </Suspense>
                                    }
                                })
                        }}
                    </Suspense>

                    <div class="chart-container">
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
                                                    view! {
                                                        <StatCard
                                                            title=s.title
                                                            amount=format_thousands(s.amount)
                                                            percent=s.percent
                                                            budget=format_thousands(s.budget)
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