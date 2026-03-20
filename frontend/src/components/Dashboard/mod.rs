use leptos::*;
use crate::components::Nav::Nav; // Import Nav tách riêng
use crate::components::UploadModal::UploadModal; // Import Modal tách riêng
use crate::components::StatCard::StatCard;
use serde::{Deserialize, Serialize};

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
    pub slug: String,
}

// Hàm format tiền (giữ nguyên)
fn format_thousands(n: f64) -> String {
    let s = format!("{:.0}", n);
    let bytes = s.as_bytes();
    let mut result = String::new();
    let len = bytes.len();
    for (i, &byte) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('.');
        }
        result.push(byte as char);
    }
    result
}

#[component]
pub fn Dashboard() -> impl IntoView {
    // Signal quản lý đóng/mở Modal Upload
    let (is_upload_open, set_is_upload_open) = create_signal(false);

    let dash_resource = create_resource(|| (), |_| async move {
        gloo_net::http::Request::get("http://localhost:5000/api/expenses")
            .send().await.unwrap()
            .json::<DashboardData>().await.unwrap_or_default()
    });

    // Logic vẽ biểu đồ (giữ nguyên)
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
            // Thêm .into_view() nếu vẫn báo lỗi, nhưng thường chỉ cần khai báo mod ở Bước 1 là hết
            <Nav />

            <div class="dashboard-content">
                <Suspense fallback=|| {
                    view! { <div class="loading">"Đang nạp..."</div> }
                }>
                    {move || {
                        dash_resource
                            .get()
                            .map(|d| {
                                view! {
                                    <div class="grid-stats">
                                        {d
                                            .stats
                                            .into_iter()
                                            .map(|s| {
                                                view! {
                                                    <StatCard
                                                        title=s.title
                                                        amount=format_thousands(s.amount)
                                                        percent=s.percent
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

                <div class="chart-container">
                    <h3>"Biến động chi phí thực tế (12 tháng)"</h3>
                    <div style="height: 380px;">
                        <canvas id="payment-chart-canvas"></canvas>
                    </div>
                </div>
            </div>

            <div class="fab-container">
                <button class="btn-main-add" on:click=move |_| set_is_upload_open.set(true)>
                    "+"
                </button>
            </div>

            // Component này cần được bao bọc cẩn thận
            <UploadModal is_open=is_upload_open set_is_open=set_is_upload_open />
        </div>
    }.into_view() // THÊM .into_view() Ở ĐÂY để sửa lỗi E0282
}