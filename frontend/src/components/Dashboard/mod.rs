// src/components/Dashboard/mod.rs
use leptos::*;
use crate::components::Nav::Nav;
use crate::components::StatCard::StatCard;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use crate::store::GlobalState;
use wasm_bindgen::JsCast;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentExpense {
    pub id: i32,
    pub merchant: String,
    pub bill_date: String,
    pub amount: f64,
    pub category_slug: String,
    pub category_name: String,
    pub items: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DashboardData {
    pub month_labels: Vec<String>,
    pub dien_series: Vec<f64>,
    pub nuoc_series: Vec<f64>,
    pub nl_series: Vec<f64>,
    pub stats: Vec<StatItem>,
    pub recent_expenses: Vec<RecentExpense>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatItem {
    pub title: String,
    pub amount: f64,
    pub percent: f64,
    pub budget: f64,
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
    let _state = use_context::<GlobalState>().expect("Phải có GlobalState");
    let domain = _state.domain.clone(); 
    
    // GIẢI PHÁP VÀNG: Lưu domain vào store_value để biến nó thành Copy type, thoát khỏi lỗi E0525
    let api_domain = store_value(domain.clone());

    let (is_dragging, set_is_dragging) = create_signal(false);
    let (is_uploading, set_is_uploading) = create_signal(false);

    let (selected_month, set_selected_month) = create_signal({
        let date = js_sys::Date::new_0();
        let month = date.get_month() + 1;
        let year = date.get_full_year();
        format!("{}-{:02}", year, month) 
    });
    
    let (is_camera_open, set_is_camera_open) = create_signal(false);
    let (camera_error, set_camera_error) = create_signal(String::new());
    let video_ref = create_node_ref::<html::Video>();
    let canvas_ref = create_node_ref::<html::Canvas>();

    let dash_resource = create_resource(
        move || selected_month.get(), 
        move |month| {
            let domain = domain.clone();
            async move {
                let storage = window().local_storage().unwrap().unwrap();
                let user_id = storage.get_item("user_id").ok().flatten().unwrap_or_else(|| "1".to_string());
                let url = format!("{}/api/expenses?user_id={}&month={}", domain, user_id, month);   
                gloo_net::http::Request::get(&url).send().await.unwrap().json::<DashboardData>().await.unwrap_or_default()
            }
        }
    );

    let upload_domain = _state.domain.clone();
    let upload_action = move |file: web_sys::File| {
        set_is_uploading.set(true); 
        let url = format!("{}/api/expenses/upload", upload_domain);
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            let user_id = storage.get_item("user_id").ok().flatten().unwrap_or_else(|| "1".to_string());
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();
            form_data.append_with_str("user_id", &user_id).unwrap(); 

            let res = gloo_net::http::Request::post(&url).body(form_data).expect("Failed").send().await;
            if res.is_ok() {
                set_is_uploading.set(false);
                dash_resource.refetch();
            } else {
                set_is_uploading.set(false);
                let _ = window.alert_with_message("Có lỗi xảy ra khi upload. Vui lòng thử lại!");
            }
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

    let start_camera = move |_| {
        set_is_camera_open.set(true);
        set_camera_error.set(String::new());
        spawn_local(async move {
            // Dùng JS ngầm để tự động quét và lọc camera siêu rộng
            let js_code = r#"
                (async function() {
                    let stream;
                    try { stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } }); } 
                    catch(e) { stream = await navigator.mediaDevices.getUserMedia({ video: true }); }
                    
                    let devices = await navigator.mediaDevices.enumerateDevices();
                    window.vi_list = devices.filter(d => d.kind === 'videoinput');
                    
                    let track = stream.getVideoTracks()[0];
                    window.vi_idx = window.vi_list.findIndex(d => d.label === track.label);
                    if (window.vi_idx === -1) window.vi_idx = 0;

                    let lbl = track.label.toLowerCase();
                    // Nếu dính ống kính siêu rộng hoặc macro -> Tự động ép về cam thường
                    if (window.vi_list.length > 1 && (lbl.includes('ultra') || lbl.includes('macro') || lbl.includes('wide'))) {
                        let stdIdx = window.vi_list.findIndex(d => {
                            let l = d.label.toLowerCase();
                            return (l.includes('back') || l.includes('sau')) && !l.includes('ultra') && !l.includes('macro') && !l.includes('wide');
                        });
                        if (stdIdx !== -1) {
                            track.stop();
                            window.vi_idx = stdIdx;
                            stream = await navigator.mediaDevices.getUserMedia({ video: { deviceId: { exact: window.vi_list[stdIdx].deviceId } } });
                        }
                    }
                    return stream;
                })()
            "#;

            if let Ok(promise) = js_sys::eval(js_code) {
                let future = wasm_bindgen_futures::JsFuture::from(promise.unchecked_into::<js_sys::Promise>());
                if let Ok(js_stream) = future.await {
                    let media_stream: web_sys::MediaStream = js_stream.unchecked_into();
                    if let Some(video) = video_ref.get() {
                        video.set_src_object(Some(&media_stream));
                        let _ = video.play().unwrap();
                    }
                } else {
                    set_camera_error.set("Quyền Camera bị từ chối!".to_string());
                }
            } else { set_camera_error.set("Lỗi trình duyệt!".to_string()); }
        });
    };

    // THÊM BIẾN MỚI: Xử lý nút bấm xoay vòng các ống kính
    let switch_lens = move |_| {
        spawn_local(async move {
            let js_code = r#"
                (async function() {
                    if (!window.vi_list || window.vi_list.length < 2) return null;
                    window.vi_idx = (window.vi_idx + 1) % window.vi_list.length; // Chuyển sang cam tiếp theo
                    let targetCam = window.vi_list[window.vi_idx];
                    return await navigator.mediaDevices.getUserMedia({ video: { deviceId: { exact: targetCam.deviceId } } });
                })()
            "#;

            // Tắt ống kính cũ trước khi mở ống kính mới
            if let Some(video) = video_ref.get() {
                if let Some(stream) = video.src_object() {
                    let media_stream: web_sys::MediaStream = stream.unchecked_into();
                    let tracks = media_stream.get_tracks();
                    for i in 0..tracks.length() {
                        let track: web_sys::MediaStreamTrack = tracks.get(i).unchecked_into();
                        track.stop();
                    }
                }
            }

            if let Ok(val) = js_sys::eval(js_code) {
                if !val.is_null() {
                    let promise = val.unchecked_into::<js_sys::Promise>();
                    if let Ok(js_value) = wasm_bindgen_futures::JsFuture::from(promise).await {
                        let media_stream: web_sys::MediaStream = js_value.unchecked_into();
                        if let Some(video) = video_ref.get() {
                            video.set_src_object(Some(&media_stream));
                            let _ = video.play().unwrap();
                        }
                    }
                }
            }
        });
    };

    let stop_camera = move || {
        if let Some(video) = video_ref.get() {
            if let Some(stream) = video.src_object() {
                let media_stream: web_sys::MediaStream = stream.unchecked_into();
                let tracks = media_stream.get_tracks();
                for i in 0..tracks.length() {
                    let track: web_sys::MediaStreamTrack = tracks.get(i).unchecked_into();
                    track.stop();
                }
            }
            video.set_src_object(None);
        }
        set_is_camera_open.set(false);
    };

    let close_camera = { let stop = stop_camera.clone(); move |_| { stop(); } };

    let capture_photo = {
        let upload = upload_action.clone();
        let stop = stop_camera.clone();
        move |_| {
            if let (Some(video), Some(canvas)) = (video_ref.get(), canvas_ref.get()) {
                let context = canvas.get_context("2d").unwrap().unwrap().unchecked_into::<web_sys::CanvasRenderingContext2d>();
                let width = video.video_width() as u32;
                let height = video.video_height() as u32;
                if width == 0 || height == 0 { return; }

                canvas.set_width(width);
                canvas.set_height(height);
                let _ = context.draw_image_with_html_video_element(&video, 0.0, 0.0);
                
                let upload_fn = upload.clone();
                let stop_fn = stop.clone();
                
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |blob: web_sys::Blob| {
                    let array = js_sys::Array::new(); array.push(&blob);
                    let options = web_sys::FilePropertyBag::new(); options.set_type("image/jpeg");
                    let file = web_sys::File::new_with_blob_sequence_and_options(&array, "webcam_capture.jpg", &options).unwrap();
                    upload_fn(file); stop_fn();
                }) as Box<dyn FnMut(web_sys::Blob)>);
                
                let _ = canvas.to_blob_with_type(closure.as_ref().unchecked_ref(), "image/jpeg");
                closure.forget();
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
            <Show when=move || is_uploading.get()>
                <div class="global-loading-overlay">
                    <div class="spinner"></div>
                    <h2>"AI đang phân tích hóa đơn..."</h2>
                    <p>"Vui lòng chờ trong giây lát"</p>
                </div>
            </Show>

            <Nav />

            <div class="dashboard-container">
                <div class="main-content">
                    <div
                        class="month-filter-container"
                        style="display: flex; justify-content: flex-end; margin-bottom: 10px;"
                    >
                        <input
                            type="month"
                            class="month-picker"
                            style="padding: 8px 15px; border-radius: 8px; background: #222; color: #fff; border: 1px solid #444; font-weight: bold; cursor: pointer;"
                            prop:value=move || selected_month.get()
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_selected_month.set(val);
                            }
                        />
                    </div>

                    <Suspense fallback=|| {
                        view! { <div>"Đang tải dữ liệu tổng quan..."</div> }
                    }>
                        {move || {
                            dash_resource
                                .get()
                                .map(|d| {
                                    let current_total: f64 = d.stats.iter().map(|s| s.amount).sum();
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
                                    let total_budget: f64 = d.stats.iter().map(|s| s.budget).sum();
                                    let has_budget = total_budget > 0.0;
                                    let budget_diff_pct = if has_budget {
                                        ((current_total - total_budget) / total_budget) * 100.0
                                    } else {
                                        0.0
                                    };
                                    let is_over_budget = budget_diff_pct > 0.0;
                                    // --- 1. TÍNH TOÁN SO VỚI THÁNG TRƯỚC ---

                                    // --- 2. LOGIC MỚI: TÍNH TOÁN SO VỚI TỔNG NGÂN SÁCH ---

                                    view! {
                                        <div class="month-summary-header">
                                            // KHỐI 1: TỔNG CHI THÁNG NÀY
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

                                            // KHỐI 2: TỔNG NGÂN SÁCH VÀ CẢNH BÁO
                                            <div class="summary-item">
                                                <span class="label">"Ngân sách kế hoạch"</span>
                                                <div class="value-row" style="margin-bottom: 6px;">
                                                    // Số tiền chính đứng 1 mình 1 dòng
                                                    <h2 class="budget-value">
                                                        {format_thousands(total_budget)} " VNĐ"
                                                    </h2>
                                                </div>

                                                // Gom Badge và Text phụ đề vào một hàng ngang
                                                <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                                                    {if has_budget {
                                                        view! {
                                                            <span
                                                                class=if is_over_budget {
                                                                    "pct-badge up"
                                                                } else {
                                                                    "pct-badge down"
                                                                }
                                                                style="white-space: nowrap; padding: 4px 10px; font-size: 0.8rem;"
                                                            >
                                                                {if is_over_budget {
                                                                    "⚠️ Vượt "
                                                                } else {
                                                                    "✅ Dư "
                                                                }}
                                                                {format!("{:.1}%", budget_diff_pct.abs())}
                                                            </span>
                                                        }
                                                            .into_view()
                                                    } else {
                                                        view! {
                                                            <span
                                                                class="pct-badge"
                                                                style="background: #333; color: #888; white-space: nowrap; padding: 4px 10px; font-size: 0.8rem;"
                                                            >
                                                                "Chưa đặt"
                                                            </span>
                                                        }
                                                            .into_view()
                                                    }}
                                                    // Bỏ margin-top dư thừa, dùng margin: 0 vì Flexbox gap đã lo khoảng cách
                                                    <p
                                                        class="prev-label"
                                                        style=if has_budget {
                                                            if is_over_budget {
                                                                "margin: 0; font-weight: 600; color: #ff4d4d;"
                                                            } else {
                                                                "margin: 0; font-weight: 600; color: #2ecc71;"
                                                            }
                                                        } else {
                                                            "margin: 0; color: #888;"
                                                        }
                                                    >
                                                        {if has_budget {
                                                            if is_over_budget {
                                                                format!(
                                                                    "🔻 Tiêu lố: {} VNĐ",
                                                                    format_thousands(current_total - total_budget),
                                                                )
                                                            } else {
                                                                format!(
                                                                    "✨ Có thể tiêu: {} VNĐ",
                                                                    format_thousands(total_budget - current_total),
                                                                )
                                                            }
                                                        } else {
                                                            "Hãy thiết lập ở thẻ bên dưới".to_string()
                                                        }}
                                                    </p>
                                                </div>
                                            </div>
                                        </div>
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
                        view! { <div class="loading">"Đang tải danh sách..."</div> }
                    }>
                        {move || {
                            dash_resource
                                .get()
                                .map(|d| {
                                    view! {
                                        <div class="dashboard-bottom-section">
                                            <div class="stats-list-vertical">
                                                {d
                                                    .stats
                                                    .clone()
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

                                            <div
                                                class="recent-transactions-section"
                                                style="margin-top: 40px;"
                                            >
                                                <h3 style="color: #fff; margin-bottom: 20px; font-size: 1.2rem; display: flex; align-items: center; gap: 10px;">
                                                    <span style="background: #f1c40f; width: 6px; height: 20px; border-radius: 3px;"></span>
                                                    "Giao dịch chi tiết ("
                                                    {move || selected_month.get()}
                                                    ")"
                                                </h3>

                                                {if d.recent_expenses.is_empty() {
                                                    view! {
                                                        <div style="background: #1a1a1a; border: 1px dashed #444; border-radius: 12px; padding: 40px; text-align: center;">
                                                            <span style="font-size: 2rem; opacity: 0.5;">"📭"</span>
                                                            <p style="color: #888; margin-top: 10px;">
                                                                "Không có giao dịch nào trong tháng này."
                                                            </p>
                                                        </div>
                                                    }
                                                        .into_view()
                                                } else {
                                                    view! {
                                                        <div class="transactions-list">
                                                            {d
                                                                .recent_expenses
                                                                .into_iter()
                                                                .map(|exp| {
                                                                    let items_array = exp
                                                                        .items
                                                                        .as_array()
                                                                        .cloned()
                                                                        .unwrap_or_default();
                                                                    let has_items = !items_array.is_empty();
                                                                    let exp_id = exp.id;

                                                                    view! {
                                                                        <div class="transaction-card">
                                                                            <div class="tx-header">
                                                                                <div class="tx-info">
                                                                                    <h4>{exp.merchant}</h4>
                                                                                    <span class="tx-date">{exp.bill_date}</span>
                                                                                </div>
                                                                                <div class="tx-amount">
                                                                                    <h4>{format_thousands(exp.amount)} " VNĐ"</h4>
                                                                                    <div style="display: flex; gap: 8px; justify-content: flex-end; align-items: center; margin-top: 6px;">
                                                                                        <span class="tx-cat">{exp.category_name}</span>

                                                                                        // NÚT XÓA ĐƯỢC VIẾT LẠI TRỰC TIẾP Ở ĐÂY ĐỂ TRÁNH LỖI E0525
                                                                                        <button
                                                                                            class="btn-delete-tx"
                                                                                            style="background: #ff4d4d; color: #fff; border: none; padding: 2px 8px; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: bold;"
                                                                                            on:click=move |_| {
                                                                                                let dom = api_domain.get_value();
                                                                                                spawn_local(async move {
                                                                                                    let window = web_sys::window().unwrap();
                                                                                                    let confirm = window
                                                                                                        .confirm_with_message(
                                                                                                            "Bạn có chắc chắn muốn xóa giao dịch này? Dữ liệu sẽ không thể khôi phục!",
                                                                                                        )
                                                                                                        .unwrap_or(false);
                                                                                                    if !confirm {
                                                                                                        return;
                                                                                                    }
                                                                                                    let storage = window.local_storage().unwrap().unwrap();
                                                                                                    let user_id = storage
                                                                                                        .get_item("user_id")
                                                                                                        .ok()
                                                                                                        .flatten()
                                                                                                        .unwrap_or_else(|| "1".to_string());
                                                                                                    let url = format!(
                                                                                                        "{}/api/expenses/{}?user_id={}",
                                                                                                        dom,
                                                                                                        exp_id,
                                                                                                        user_id,
                                                                                                    );
                                                                                                    let res = gloo_net::http::Request::delete(&url)
                                                                                                        .send()
                                                                                                        .await;
                                                                                                    if res.is_ok() {
                                                                                                        dash_resource.refetch();
                                                                                                    } else {
                                                                                                        let _ = window
                                                                                                            .alert_with_message("Lỗi mạng khi xóa giao dịch!");
                                                                                                    }
                                                                                                });
                                                                                            }
                                                                                        >
                                                                                            "Xóa"
                                                                                        </button>
                                                                                    </div>
                                                                                </div>
                                                                            </div>

                                                                            <Show
                                                                                when=move || has_items
                                                                                fallback=|| {
                                                                                    view! {
                                                                                        <p
                                                                                            class="no-items"
                                                                                            style="color: #666; font-style: italic; font-size: 0.85rem; margin: 0;"
                                                                                        >
                                                                                            "Không có chi tiết mặt hàng"
                                                                                        </p>
                                                                                    }
                                                                                }
                                                                            >
                                                                                <div class="tx-items">
                                                                                    <table
                                                                                        class="items-table"
                                                                                        style="width: 100%; border-collapse: collapse; color: #bbb; font-size: 0.9rem;"
                                                                                    >
                                                                                        <thead>
                                                                                            <tr>
                                                                                                <th style="color: #777; border-bottom: 1px solid #333; padding-bottom: 10px; text-align: left; font-weight: 600; font-size: 0.8rem; text-transform: uppercase;">
                                                                                                    "Mặt hàng"
                                                                                                </th>
                                                                                                <th style="color: #777; border-bottom: 1px solid #333; padding-bottom: 10px; text-align: center; font-weight: 600; font-size: 0.8rem; text-transform: uppercase;">
                                                                                                    "SL"
                                                                                                </th>
                                                                                                <th style="color: #777; border-bottom: 1px solid #333; padding-bottom: 10px; text-align: right; font-weight: 600; font-size: 0.8rem; text-transform: uppercase;">
                                                                                                    "Đơn giá"
                                                                                                </th>
                                                                                                <th style="color: #777; border-bottom: 1px solid #333; padding-bottom: 10px; text-align: right; font-weight: 600; font-size: 0.8rem; text-transform: uppercase;">
                                                                                                    "Thành tiền"
                                                                                                </th>
                                                                                            </tr>
                                                                                        </thead>
                                                                                        <tbody>
                                                                                            {items_array
                                                                                                .clone()
                                                                                                .into_iter()
                                                                                                .map(|item| {
                                                                                                    let name = item["name"]
                                                                                                        .as_str()
                                                                                                        .unwrap_or("Không rõ")
                                                                                                        .to_string();
                                                                                                    let qty = item["quantity"].as_f64().unwrap_or(1.0);
                                                                                                    let price = item["price"].as_f64().unwrap_or(0.0);
                                                                                                    let total = item["total"].as_f64().unwrap_or(0.0);
                                                                                                    view! {
                                                                                                        <tr>
                                                                                                            <td style="padding: 10px 0; border-bottom: 1px solid #222;">
                                                                                                                {name}
                                                                                                            </td>
                                                                                                            <td style="padding: 10px 0; border-bottom: 1px solid #222; text-align: center;">
                                                                                                                {qty}
                                                                                                            </td>
                                                                                                            <td style="padding: 10px 0; border-bottom: 1px solid #222; text-align: right;">
                                                                                                                {format_thousands(price)}
                                                                                                            </td>
                                                                                                            <td style="padding: 10px 0; border-bottom: 1px solid #222; text-align: right; color: #fff;">
                                                                                                                {format_thousands(total)}
                                                                                                            </td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                })
                                                                                                .collect_view()}
                                                                                        </tbody>
                                                                                    </table>
                                                                                </div>
                                                                            </Show>
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </div>
                                                    }
                                                        .into_view()
                                                }}
                                            </div>
                                        </div>
                                    }
                                })
                        }}
                    </Suspense>
                </div>

                <aside class="side-panel">
                    <div class="upload-widget">
                        <h3>"Thêm hóa đơn mới"</h3>
                        <Show
                            when=move || is_camera_open.get()
                            fallback=move || {
                                view! {
                                    <div
                                        class=move || {
                                            if is_dragging.get() {
                                                "drop-zone dragging"
                                            } else {
                                                "drop-zone"
                                            }
                                        }
                                        on:dragover=move |ev| {
                                            ev.prevent_default();
                                            set_is_dragging.set(true);
                                        }
                                        on:dragleave=move |ev| {
                                            ev.prevent_default();
                                            set_is_dragging.set(false);
                                        }
                                        on:drop=on_drop.clone()
                                    >
                                        <span class="icon">"📄"</span>
                                        <span class="text">"Kéo thả hóa đơn vào đây"</span>
                                        <div class="action-buttons">
                                            <button class="btn-action" on:click=start_camera.clone()>
                                                "📸 Chụp ảnh"
                                            </button>
                                            <label class="btn-action">
                                                <input
                                                    type="file"
                                                    accept="image/*"
                                                    class="hidden"
                                                    on:change=on_file_change.clone()
                                                />
                                                "📁 Chọn file"
                                            </label>
                                        </div>
                                    </div>
                                }
                            }
                        >
                            <div
                                class="webcam-container"
                                style="display: flex; flex-direction: column; gap: 10px;"
                            >
                                <video
                                    node_ref=video_ref
                                    style="width: 100%; border-radius: 12px; background: #000; object-fit: cover; border: 2px solid #333;"
                                    autoplay
                                    playsinline
                                ></video>
                                <canvas node_ref=canvas_ref style="display: none;"></canvas>
                                <p style="color: #ff4d4d; font-size: 0.8rem; text-align: center;">
                                    {move || camera_error.get()}
                                </p>
                                <div class="action-buttons">
                                    <button
                                        class="btn-action"
                                        style="background: #2ecc71; color: #000;"
                                        on:click=capture_photo.clone()
                                    >
                                        "📸 Bấm Chụp"
                                    </button>

                                    // GIAO DIỆN NÚT CHUYỂN CAM CHUYÊN NGHIỆP
                                    <button
                                        class="btn-action"
                                        style="background: #3498db; color: #fff;"
                                        on:click=switch_lens.clone()
                                    >
                                        "🔄 Đổi ống kính"
                                    </button>

                                    <button
                                        class="btn-action"
                                        style="background: #ff4d4d; color: #fff;"
                                        on:click=close_camera.clone()
                                    >
                                        "❌ Hủy"
                                    </button>
                                </div>
                            </div>
                        </Show>
                    </div>
                </aside>
            </div>
        </div>
    }.into_view()
}