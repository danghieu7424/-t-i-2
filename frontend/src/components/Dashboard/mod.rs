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

#[derive(Clone, Debug)]
pub struct EditItemSignal {
    pub id: usize,
    pub name: RwSignal<String>,
    pub qty: RwSignal<f64>,
    pub price: RwSignal<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChartSeries {
    pub label: String,
    pub data: Vec<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DashboardData {
    pub month_labels: Vec<String>,
    pub chart_series: Vec<ChartSeries>, 
    pub stats: Vec<StatItem>,
    pub recent_expenses: Vec<RecentExpense>,
    pub current_page: u32, 
    pub total_pages: u32,  
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatItem {
    pub title: String,
    pub amount: f64,
    pub percent: f64,
    pub budget: f64,
    pub slug: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Subscription {
    pub id: i32,
    pub merchant: String,
    pub amount: f64,
    pub category_name: String,
    pub start_date: String,
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

// 🛡️ CẢNH SÁT CHÌM: Quét lỗi Token Hết Hạn
fn check_auth_error(status: u16) -> bool {
    if status == 401 {
        let window = web_sys::window().unwrap();
        window.local_storage().unwrap().unwrap().clear();
        let _ = window.alert_with_message("Phiên đăng nhập đã hết hạn! Vui lòng đăng nhập lại để bảo mật.");
        let _ = window.location().set_href("/login");
        true
    } else {
        false
    }
}

fn get_color_from_string(s: &str) -> String {
    let palette = [
        "#f1c40f", "#3498db", "#2ecc71", "#e74c3c", "#9b59b6", 
        "#1abc9c", "#e67e22", "#e84393", "#ff7f50", "#00cec9"
    ];
    let mut hash = 0;
    for b in s.chars() {
        hash = (hash + (b as usize)) % palette.len();
    }
    palette[hash].to_string()
}

fn remove_accents(s: &str) -> String {
    s.to_lowercase()
     .replace(['á','à','ả','ã','ạ','ă','ắ','ằ','ẳ','ẵ','ặ','â','ấ','ầ','ẩ','ẫ','ậ'], "a")
     .replace('đ', "d")
     .replace(['é','è','ẻ','ẽ','ẹ','ê','ế','ề','ể','ễ','ệ'], "e")
     .replace(['í','ì','ỉ','ĩ','ị'], "i")
     .replace(['ó','ò','ỏ','õ','ọ','ô','ố','ồ','ổ','ỗ','ộ','ơ','ớ','ờ','ở','ỡ','ợ'], "o")
     .replace(['ú','ù','ủ','ũ','ụ','ư','ứ','ừ','ử','ữ','ự'], "u")
     .replace(['ý','ỳ','ỷ','ỹ','ỵ'], "y")
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let _state = use_context::<GlobalState>().expect("Phải có GlobalState");
    let domain = _state.domain.clone(); 
    let api_domain = store_value(domain.clone());

    let (is_dragging, set_is_dragging) = create_signal(false);
    let (is_uploading, set_is_uploading) = create_signal(false);
    let (search_query, set_search_query) = create_signal(String::new());

    let (input_mode, set_input_mode) = create_signal("ai"); 
    let (manual_merchant, set_manual_merchant) = create_signal(String::new());
    let (manual_amount, set_manual_amount) = create_signal(String::new());
    let (manual_category, set_manual_category) = create_signal(String::new());
    let (manual_date, set_manual_date) = create_signal({
        let date = js_sys::Date::new_0();
        let month = date.get_month() + 1;
        let day = date.get_date();
        let year = date.get_full_year();
        format!("{}-{:02}-{:02}", year, month, day) 
    });

    let (edit_modal_open, set_edit_modal_open) = create_signal(false);
    let (edit_id, set_edit_id) = create_signal(0);
    let (edit_merchant, set_edit_merchant) = create_signal(String::new());
    let (edit_amount, set_edit_amount) = create_signal(String::new());
    let (edit_category, set_edit_category) = create_signal(String::new());
    let (edit_date, set_edit_date) = create_signal(String::new());
    let (edit_items, set_edit_items) = create_signal(Vec::<EditItemSignal>::new());
    let (edit_next_id, set_edit_next_id) = create_signal(0);
    let (active_subs, set_active_subs) = create_signal(Vec::<Subscription>::new());

    let (selected_month, set_selected_month) = create_signal({
        let date = js_sys::Date::new_0();
        let month = date.get_month() + 1;
        let year = date.get_full_year();
        format!("{}-{:02}", year, month) 
    });

    let (current_page, set_current_page) = create_signal(1u32);

    create_effect(move |_| {
        let _ = selected_month.get();
        set_current_page.set(1);
    });
    
    let (is_camera_open, set_is_camera_open) = create_signal(false);
    let (camera_error, set_camera_error) = create_signal(String::new());
    let video_ref = create_node_ref::<html::Video>();
    let canvas_ref = create_node_ref::<html::Canvas>();

    // API 1: LOAD SUBSCRIPTIONS
    let load_subs = move || {
        let dom = api_domain.get_value();
        spawn_local(async move {
            let storage = window().local_storage().unwrap().unwrap();
            let uid = storage.get_item("user_id").ok().flatten().unwrap_or_else(|| "1".into());
            let token = storage.get_item("auth_token").ok().flatten().unwrap_or_default();
            let url = format!("{}/api/expenses/subscriptions?user_id={}", dom, uid);
            
            let res = gloo_net::http::Request::get(&url)
                .header("Authorization", &format!("Bearer {}", token))
                .send().await;
            
            // Fix chuẩn:
            if let Ok(response) = res {
                if check_auth_error(response.status()) { return; }
                if let Ok(data) = response.json::<Vec<Subscription>>().await {
                    set_active_subs.set(data);
                }
            }
        });
    };

    create_effect(move |_| {
        if input_mode.get() == "sub" {
            load_subs();
        }
    });

    // API 2: LOAD DASHBOARD DATA
    let dash_resource = create_resource(
        move || (selected_month.get(), current_page.get()), 
        move |(month, page)| {
            let domain = domain.clone();
            async move {
                let storage = window().local_storage().unwrap().unwrap();
                let token = storage.get_item("auth_token").ok().flatten().unwrap_or_default();
                let url = format!("{}/api/expenses?month={}&page={}&limit=20", domain, month, page);   
                
                let res = gloo_net::http::Request::get(&url)
                    .header("Authorization", &format!("Bearer {}", token))
                    .send().await;
                
                // Fix chuẩn:
                if let Ok(response) = res {
                    if check_auth_error(response.status()) { return DashboardData::default(); }
                    response.json::<DashboardData>().await.unwrap_or_default()
                } else {
                    DashboardData::default()
                }
            }
        }
    );

    // API 3: UPLOAD INVOICE
    // API 3: UPLOAD INVOICE (LONG POLLING BACKGROUND JOB)
    let upload_action = move |file: web_sys::File| {
        if is_uploading.get() { return; } 
        set_is_uploading.set(true); 
        
        let domain_url = api_domain.get_value();
        let url = format!("{}/api/expenses/upload", domain_url);
        
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            let token = storage.get_item("auth_token").ok().flatten().unwrap_or_default();
            
            let form_data = web_sys::FormData::new().unwrap();
            form_data.append_with_blob("file", &file).unwrap();

            // 1. Gửi file lên và lấy Job ID ngay lập tức
            let res = gloo_net::http::Request::post(&url)
                .header("Authorization", &format!("Bearer {}", token))
                .body(form_data).unwrap().send().await;
            
            if let Ok(response) = res {
                if check_auth_error(response.status()) { set_is_uploading.set(false); return; }
                if response.status() == 200 {
                    let data: serde_json::Value = response.json().await.unwrap();
                    if let Some(job_id) = data["job_id"].as_str() {
                        let job_id = job_id.to_string();
                        
                        // 2. VÒNG LẶP POLLING TÌM KẾT QUẢ
                        loop {
                            // Nghỉ 2 giây để không spam Server
                            let _ = gloo_timers::future::TimeoutFuture::new(2000).await;
                            
                            let status_url = format!("{}/api/expenses/upload/status/{}", domain_url, job_id);
                            if let Ok(status_res) = gloo_net::http::Request::get(&status_url)
                                .header("Authorization", &format!("Bearer {}", token))
                                .send().await 
                            {
                                if let Ok(status_data) = status_res.json::<serde_json::Value>().await {
                                    let state_str = status_data["state"].as_str().unwrap_or("");
                                    
                                    if state_str == "Completed" {
                                        dash_resource.refetch();
                                        set_is_uploading.set(false);
                                        break; // Thoát vòng lặp
                                    } else if state_str == "Failed" {
                                        set_is_uploading.set(false);
                                        let err = status_data["error"].as_str().unwrap_or("Lỗi không xác định");
                                        let _ = window.alert_with_message(&format!("❌ Lỗi AI: {}", err));
                                        break; // Thoát vòng lặp
                                    }
                                    // Nếu là Pending hoặc Processing -> im lặng lặp tiếp
                                }
                            }
                        }
                    }
                } else {
                    set_is_uploading.set(false);
                    let _ = window.alert_with_message("Lỗi đẩy hóa đơn lên Server!");
                }
            } else {
                set_is_uploading.set(false);
            }
        });
    };

    let on_file_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Some(files) = target.files() {
            if let Some(file) = files.get(0) { upload_action(file); }
        }
    };

    let on_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
        if is_uploading.get() { return; } 
        let web_ev: &web_sys::DragEvent = ev.as_ref();
        if let Some(dt) = web_ev.data_transfer() {
            if let Some(files) = dt.files() {
                let file_list: web_sys::FileList = files;
                if let Some(file) = file_list.get(0) { upload_action(file); }
            }
        }
    };

    let start_camera = move |_| {
        set_is_camera_open.set(true);
        set_camera_error.set(String::new());
        spawn_local(async move {
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

    let switch_lens = move |_| {
        spawn_local(async move {
            let js_code = r#"
                (async function() {
                    if (!window.vi_list || window.vi_list.length < 2) return null;
                    window.vi_idx = (window.vi_idx + 1) % window.vi_list.length; 
                    let targetCam = window.vi_list[window.vi_idx];
                    return await navigator.mediaDevices.getUserMedia({ video: { deviceId: { exact: targetCam.deviceId } } });
                })()
            "#;

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

    let close_camera = move |_| { stop_camera(); };

    let capture_photo = move |_| {
        if is_uploading.get() { return; } 
        if let (Some(video), Some(canvas)) = (video_ref.get(), canvas_ref.get()) {
            let context = canvas.get_context("2d").unwrap().unwrap().unchecked_into::<web_sys::CanvasRenderingContext2d>();
            let width = video.video_width() as u32;
            let height = video.video_height() as u32;
            if width == 0 || height == 0 { return; }

            canvas.set_width(width);
            canvas.set_height(height);
            let _ = context.draw_image_with_html_video_element(&video, 0.0, 0.0);
            
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |blob: web_sys::Blob| {
                let array = js_sys::Array::new(); array.push(&blob);
                let options = web_sys::FilePropertyBag::new(); options.set_type("image/jpeg");
                let file = web_sys::File::new_with_blob_sequence_and_options(&array, "webcam_capture.jpg", &options).unwrap();
                upload_action(file); stop_camera();
            }) as Box<dyn FnMut(web_sys::Blob)>);
            
            let _ = canvas.to_blob_with_type(closure.as_ref().unchecked_ref(), "image/jpeg");
            closure.forget();
        }
    };

    create_effect(move |_| {
        if let Some(d) = dash_resource.get() {
            let series_json = serde_json::to_string(&d.chart_series).unwrap_or_else(|_| "[]".to_string());
            let stats_json = serde_json::to_string(&d.stats).unwrap_or_else(|_| "[]".to_string());

            let js_code = format!(r##"
                let lineCanvas = document.getElementById('payment-chart-canvas');
                if (lineCanvas) {{
                    let oldLine = Chart.getChart(lineCanvas); if(oldLine) oldLine.destroy();
                    let rawSeries = JSON.parse('{}');
                    let colorPalette = ['#f1c40f', '#3498db', '#2ecc71', '#e74c3c', '#9b59b6', '#1abc9c', '#e67e22', '#e84393', '#ff7f50', '#00cec9'];

                    let dynamicDatasets = rawSeries.map((s) => {{
                        let hash = 0;
                        for (let i = 0; i < s.label.length; i++) {{
                            hash = (hash + s.label.charCodeAt(i)) % colorPalette.length;
                        }}
                        let color = colorPalette[hash];

                        return {{
                            label: s.label,
                            data: s.data,
                            borderColor: color,
                            tension: 0.3,
                            fill: true,
                            backgroundColor: color + '1a'
                        }};
                    }});

                    new Chart(lineCanvas, {{
                        type: 'line',
                        data: {{
                            labels: {:?},
                            datasets: dynamicDatasets
                        }},
                        options: {{ maintainAspectRatio: false, plugins: {{ legend: {{ labels: {{ color: '#fff' }} }} }} }}
                    }});
                }}

                let pieCanvas = document.getElementById('proportion-chart-canvas');
                if (pieCanvas) {{
                    let oldPie = Chart.getChart(pieCanvas); if(oldPie) oldPie.destroy();
                    let rawStats = JSON.parse('{}');
                    let colorPalette = ['#f1c40f', '#3498db', '#2ecc71', '#e74c3c', '#9b59b6', '#1abc9c', '#e67e22', '#e84393', '#ff7f50', '#00cec9'];

                    let totalAmount = rawStats.reduce((sum, item) => sum + item.amount, 0);

                    let pieLabels = [];
                    let pieData = [];
                    let pieColors = [];
                    let otherAmount = 0;

                    if (totalAmount === 0) {{
                        pieLabels = ["Chưa có dữ liệu"];
                        pieData = [1];
                        pieColors = ['#333333']; 
                    }} else {{
                        rawStats.forEach(item => {{
                            if (item.amount > 0) {{
                                let percentage = (item.amount / totalAmount) * 100;
                                if (percentage < 3) {{ 
                                    otherAmount += item.amount;
                                }} else {{
                                    pieLabels.push(item.title);
                                    pieData.push(item.amount);
                                    let hash = 0;
                                    for (let i = 0; i < item.title.length; i++) {{
                                        hash = (hash + item.title.charCodeAt(i)) % colorPalette.length;
                                    }}
                                    pieColors.push(colorPalette[hash]);
                                }}
                            }}
                        }});

                        if (otherAmount > 0) {{
                            pieLabels.push("Khác (Các mục <3%)");
                            pieData.push(otherAmount);
                            pieColors.push("#7f8c8d");
                        }}
                    }}

                    new Chart(pieCanvas, {{
                        type: 'doughnut',
                        data: {{
                            labels: pieLabels,
                            datasets: [{{
                                data: pieData,
                                backgroundColor: pieColors,
                                borderWidth: 2,
                                borderColor: '#1a1a1a'
                            }}]
                        }},
                        options: {{
                            maintainAspectRatio: false,
                            cutout: '70%', 
                            plugins: {{
                                legend: {{ 
                                    position: 'right', 
                                    labels: {{ color: '#fff', boxWidth: 12, font: {{size: 11}} }} 
                                }},
                                tooltip: {{
                                    callbacks: {{
                                        label: function(context) {{
                                            let val = context.raw;
                                            let pct = totalAmount > 0 ? ((val / totalAmount) * 100).toFixed(1) : 0;
                                            if (totalAmount === 0) return " 0 VNĐ";
                                            return ' ' + context.label + ': ' + val.toLocaleString('vi-VN') + ' đ (' + pct + '%)';
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }});
                }}
            "##, series_json, d.month_labels, stats_json); 
            let _ = js_sys::eval(&js_code);
        }
    });

    view! {
        <div class="dashboard-wrapper">
            <Show when=move || is_uploading.get()>
                <div class="global-loading-overlay">
                    <div class="spinner"></div>
                    <h2>"Hệ thống đang xử lý..."</h2>
                </div>
            </Show>

            // MODAL CHỈNH SỬA GIAO DỊCH
            <Show when=move || edit_modal_open.get()>
                <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.85); display: flex; justify-content: center; align-items: center; z-index: 9999;">
                    <div style="background: #1a1a1a; padding: 25px; border-radius: 12px; border: 1px solid #444; width: 95%; max-width: 500px; max-height: 90vh; overflow-y: auto;">
                        <h3 style="color: #fff; margin-top: 0; margin-bottom: 20px;">
                            "Chỉnh sửa giao dịch"
                        </h3>
                        <div style="display: flex; flex-direction: column; gap: 12px;">
                            <div style="display: flex; gap: 10px;">
                                <div style="flex: 1;">
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Tên cửa hàng"
                                    </label>
                                    <input
                                        type="text"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || edit_merchant.get()
                                        on:input=move |ev| {
                                            set_edit_merchant.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div style="flex: 1;">
                                    <label style="color: #f1c40f; font-weight: bold; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Tổng tiền (Tự động tính)"
                                    </label>
                                    <input
                                        type="number"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #f1c40f; background: #2a2200; color: #f1c40f; font-weight: bold; box-sizing: border-box;"
                                        prop:value=move || edit_amount.get()
                                        on:input=move |ev| {
                                            set_edit_amount.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                            </div>

                            <div style="display: flex; gap: 10px;">
                                <div style="flex: 1;">
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Danh mục"
                                    </label>
                                    <input
                                        type="text"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || edit_category.get()
                                        on:input=move |ev| {
                                            set_edit_category.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div style="flex: 1;">
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Ngày giao dịch"
                                    </label>
                                    <input
                                        type="date"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || edit_date.get()
                                        on:input=move |ev| {
                                            set_edit_date.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                            </div>

                            <div style="margin-top: 10px; background: #111; padding: 15px; border-radius: 8px; border: 1px solid #333;">
                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                                    <label style="color: #fff; font-size: 0.9rem; font-weight: bold; margin: 0;">
                                        "Chi tiết mặt hàng"
                                    </label>
                                    <button
                                        style="background: #2ecc71; color: #000; border: none; border-radius: 4px; padding: 4px 10px; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                        on:click=move |_| {
                                            let id = edit_next_id.get();
                                            set_edit_next_id.set(id + 1);
                                            set_edit_items
                                                .update(|items| {
                                                    items
                                                        .push(EditItemSignal {
                                                            id,
                                                            name: create_rw_signal(String::new()),
                                                            qty: create_rw_signal(1.0),
                                                            price: create_rw_signal(0.0),
                                                        });
                                                });
                                        }
                                    >
                                        "+ Thêm món"
                                    </button>
                                </div>

                                <div style="max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; padding-right: 5px;">
                                    {move || {
                                        edit_items
                                            .get()
                                            .into_iter()
                                            .map(|item| {
                                                let name_sig = item.name;
                                                let qty_sig = item.qty;
                                                let price_sig = item.price;
                                                let item_id = item.id;

                                                view! {
                                                    <div style="display: flex; gap: 5px; align-items: center;">
                                                        <input
                                                            type="text"
                                                            placeholder="Tên món"
                                                            style="flex: 2; padding: 8px; border-radius: 4px; border: 1px solid #444; background: #222; color: #fff; font-size: 0.8rem;"
                                                            prop:value=move || name_sig.get()
                                                            on:input=move |ev| {
                                                                name_sig.set(event_target_value(&ev));
                                                            }
                                                        />
                                                        <input
                                                            type="number"
                                                            placeholder="SL"
                                                            style="flex: 0.6; padding: 8px; border-radius: 4px; border: 1px solid #444; background: #222; color: #fff; font-size: 0.8rem;"
                                                            prop:value=move || qty_sig.get()
                                                            on:input=move |ev| {
                                                                qty_sig
                                                                    .set(event_target_value(&ev).parse::<f64>().unwrap_or(1.0));
                                                                let new_total: f64 = edit_items
                                                                    .get_untracked()
                                                                    .into_iter()
                                                                    .map(|i| i.qty.get() * i.price.get())
                                                                    .sum();
                                                                set_edit_amount.set(new_total.to_string());
                                                            }
                                                        />
                                                        <input
                                                            type="number"
                                                            placeholder="Đơn giá"
                                                            style="flex: 1.2; padding: 8px; border-radius: 4px; border: 1px solid #444; background: #222; color: #fff; font-size: 0.8rem;"
                                                            prop:value=move || price_sig.get()
                                                            on:input=move |ev| {
                                                                price_sig
                                                                    .set(event_target_value(&ev).parse::<f64>().unwrap_or(0.0));
                                                                let new_total: f64 = edit_items
                                                                    .get_untracked()
                                                                    .into_iter()
                                                                    .map(|i| i.qty.get() * i.price.get())
                                                                    .sum();
                                                                set_edit_amount.set(new_total.to_string());
                                                            }
                                                        />
                                                        <button
                                                            style="background: rgba(255, 77, 77, 0.1); border: 1px solid rgba(255, 77, 77, 0.3); color: #ff4d4d; border-radius: 4px; padding: 7px 10px; cursor: pointer; font-weight: bold; flex-shrink: 0;"
                                                            on:click=move |_| {
                                                                set_edit_items
                                                                    .update(|items| {
                                                                        items.retain(|i| i.id != item_id);
                                                                    });
                                                                let new_total: f64 = edit_items
                                                                    .get_untracked()
                                                                    .into_iter()
                                                                    .map(|i| i.qty.get() * i.price.get())
                                                                    .sum();
                                                                set_edit_amount.set(new_total.to_string());
                                                            }
                                                        >
                                                            "✕"
                                                        </button>
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </div>
                            </div>

                            <div style="display: flex; gap: 10px; margin-top: 15px;">
                                <button
                                    style="flex: 1; padding: 12px; border-radius: 6px; background: #3498db; color: #fff; border: none; font-weight: bold; cursor: pointer; font-size: 1rem;"
                                    disabled=move || is_uploading.get()
                                    on:click=move |_| {
                                        set_is_uploading.set(true);
                                        let domain_url = api_domain.get_value();
                                        let id = edit_id.get();
                                        let amount = edit_amount
                                            .get()
                                            .parse::<f64>()
                                            .unwrap_or(0.0);
                                        let merchant = edit_merchant.get();
                                        let category_name = edit_category.get();
                                        let date = edit_date.get();
                                        let items_payload = serde_json::json!(
                                            edit_items.get().into_iter().map(|item| {
                                                serde_json::json!({
                                                    "name": item.name.get(),
                                                    "quantity": item.qty.get(),
                                                    "price": item.price.get(),
                                                    "total": item.qty.get() * item.price.get()
                                                })
                                            }).collect::<Vec<_>>()
                                        );
                                        spawn_local(async move {
                                            let window = web_sys::window().unwrap();
                                            let storage = window.local_storage().unwrap().unwrap();
                                            let user_id = storage
                                                .get_item("user_id")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_else(|| "1".to_string());
                                            let token = storage
                                                .get_item("auth_token")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_default();
                                            let req = serde_json::json!(
                                                {
                                                "id": id,
                                                "user_id": user_id,
                                                "amount": amount,
                                                "merchant": merchant,
                                                "category_name": category_name,
                                                "bill_date": date,
                                                "items": items_payload
                                            }
                                            );
                                            let res = gloo_net::http::Request::put(
                                                    &format!("{}/api/expenses/edit", domain_url),
                                                )
                                                .header("Authorization", &format!("Bearer {}", token))
                                                .json(&req)
                                                .unwrap()
                                                .send()
                                                .await;
                                            set_is_uploading.set(false);
                                            set_edit_modal_open.set(false);
                                            if let Ok(response) = res {
                                                if check_auth_error(response.status()) {
                                                    return;
                                                }
                                                if response.status() == 200 {
                                                    dash_resource.refetch();
                                                } else {
                                                    let _ = window
                                                        .alert_with_message("Lỗi khi cập nhật giao dịch!");
                                                }
                                            }
                                        });
                                    }
                                >
                                    {move || {
                                        if is_uploading.get() {
                                            "Đang lưu..."
                                        } else {
                                            "Lưu thay đổi"
                                        }
                                    }}
                                </button>
                                <button
                                    style="flex: 1; padding: 12px; border-radius: 6px; background: #444; color: #fff; border: none; font-weight: bold; cursor: pointer; font-size: 1rem;"
                                    disabled=move || is_uploading.get()
                                    on:click=move |_| set_edit_modal_open.set(false)
                                >
                                    "Hủy"
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>

            <Nav />

            <div class="dashboard-container">
                <div class="main-content">
                    <div
                        class="month-filter-container"
                        style="display: flex; justify-content: flex-end; align-items: center; gap: 8px; margin-bottom: 10px;"
                    >
                        <button
                            style="padding: 8px 12px; border-radius: 8px; background: #333; color: #fff; border: none; cursor: pointer; font-weight: bold;"
                            on:click=move |_| {
                                let current = selected_month.get();
                                let parts: Vec<&str> = current.split('-').collect();
                                if parts.len() == 2 {
                                    let year: i32 = parts[0].parse().unwrap_or(2026);
                                    let month: i32 = parts[1].parse().unwrap_or(1);
                                    let mut prev_month = month - 1;
                                    let mut prev_year = year;
                                    if prev_month < 1 {
                                        prev_month = 12;
                                        prev_year -= 1;
                                    }
                                    set_selected_month
                                        .set(format!("{}-{:02}", prev_year, prev_month));
                                }
                            }
                        >
                            "◀"
                        </button>

                        <input
                            type="month"
                            class="month-picker"
                            style="padding: 8px 15px; border-radius: 8px; background: #222; color: #fff; border: 1px solid #444; font-weight: bold; cursor: pointer; text-align: center;"
                            prop:value=move || selected_month.get()
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_selected_month.set(val);
                            }
                        />

                        <button
                            style="padding: 8px 12px; border-radius: 8px; background: #333; color: #fff; border: none; cursor: pointer; font-weight: bold;"
                            on:click=move |_| {
                                let current = selected_month.get();
                                let parts: Vec<&str> = current.split('-').collect();
                                if parts.len() == 2 {
                                    let year: i32 = parts[0].parse().unwrap_or(2026);
                                    let month: i32 = parts[1].parse().unwrap_or(1);
                                    let mut next_month = month + 1;
                                    let mut next_year = year;
                                    if next_month > 12 {
                                        next_month = 1;
                                        next_year += 1;
                                    }
                                    set_selected_month
                                        .set(format!("{}-{:02}", next_year, next_month));
                                }
                            }
                        >
                            "▶"
                        </button>
                    </div>

                    <Suspense fallback=|| {
                        view! { <div>"Đang tải dữ liệu tổng quan..."</div> }
                    }>
                        {move || {
                            dash_resource
                                .get()
                                .map(|d| {
                                    let current_total: f64 = d.stats.iter().map(|s| s.amount).sum();
                                    let last_month_total: f64 = d
                                        .chart_series
                                        .iter()
                                        .map(|series| {
                                            let len = series.data.len();
                                            if len >= 2 { series.data[len - 2] } else { 0.0 }
                                        })
                                        .sum();
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

                                    view! {
                                        <div class="month-summary-header">
                                            <div class="summary-item">
                                                <span class="label">"Tổng chi tháng này"</span>
                                                <div class="value-row" style="margin-bottom: 6px;">
                                                    <h2 class="total-value">
                                                        {format_thousands(current_total)} " VNĐ"
                                                    </h2>
                                                </div>

                                                <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                                                    <span
                                                        class=if is_total_up {
                                                            "pct-badge up"
                                                        } else {
                                                            "pct-badge down"
                                                        }
                                                        style="white-space: nowrap; padding: 4px 10px; font-size: 0.8rem;"
                                                    >
                                                        {if is_total_up { "↑ " } else { "↓ " }}
                                                        {format!("{:.1}%", total_diff_pct.abs())}
                                                    </span>
                                                    <p class="prev-label" style="margin: 0; color: #888;">
                                                        "Tháng trước: "
                                                        {format_thousands(last_month_total)}
                                                        " VNĐ"
                                                    </p>
                                                </div>
                                            </div>

                                            <div class="summary-item divider"></div>

                                            <div class="summary-item">
                                                <span class="label">"Ngân sách kế hoạch"</span>
                                                <div class="value-row" style="margin-bottom: 6px;">
                                                    <h2 class="budget-value">
                                                        {format_thousands(total_budget)} " VNĐ"
                                                    </h2>
                                                </div>

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

                    <div
                        class="charts-wrapper"
                        style="display: flex; gap: 20px; margin-bottom: 20px; flex-wrap: wrap;"
                    >
                        <div
                            class="chart-container"
                            style="flex: 2; min-width: 300px; background: #1a1a1a; padding: 20px; border-radius: 12px; border: 1px solid #333;"
                        >
                            <h3 style="color: #fff; margin-top: 0; font-size: 1rem; margin-bottom: 15px;">
                                "Biến động chi tiêu 12 tháng"
                            </h3>
                            <div style="height: 300px;">
                                <canvas id="payment-chart-canvas"></canvas>
                            </div>
                        </div>

                        <div
                            class="chart-container"
                            style="flex: 1; min-width: 250px; background: #1a1a1a; padding: 20px; border-radius: 12px; border: 1px solid #333;"
                        >
                            <h3 style="color: #fff; margin-top: 0; font-size: 1rem; margin-bottom: 15px;">
                                "Cơ cấu tháng này"
                            </h3>
                            <div style="height: 300px; position: relative;">
                                <canvas id="proportion-chart-canvas"></canvas>
                            </div>
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
                                                {
                                                    let mut sorted_stats = d.stats.clone();
                                                    sorted_stats
                                                        .sort_by(|a, b| {
                                                            b.amount
                                                                .partial_cmp(&a.amount)
                                                                .unwrap_or(std::cmp::Ordering::Equal)
                                                        });
                                                    sorted_stats
                                                        .into_iter()
                                                        .map(|s| {
                                                            let progress = if s.budget > 0.0 {
                                                                (s.amount / s.budget) * 100.0
                                                            } else {
                                                                0.0
                                                            };
                                                            let card_color = get_color_from_string(&s.title);

                                                            view! {
                                                                <StatCard
                                                                    title=s.title.clone()
                                                                    amount=format_thousands(s.amount)
                                                                    percent=s.percent
                                                                    budget=format_thousands(s.budget)
                                                                    progress=progress
                                                                    category_slug=s.slug.clone()
                                                                    color=card_color
                                                                />
                                                            }
                                                        })
                                                        .collect_view()
                                                }
                                            </div>

                                            <div
                                                class="recent-transactions-section"
                                                style="margin-top: 40px;"
                                            >
                                                <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px; margin-bottom: 20px;">
                                                    <h3 style="color: #fff; margin: 0; font-size: 1.2rem; display: flex; align-items: center; gap: 10px;">
                                                        <span style="background: #f1c40f; width: 6px; height: 20px; border-radius: 3px;"></span>
                                                        "Giao dịch chi tiết ("
                                                        {move || selected_month.get()}
                                                        ")"
                                                    </h3>

                                                    <div style="display: flex; gap: 10px; align-items: center; flex-wrap: wrap;">
                                                        <input
                                                            type="text"
                                                            placeholder="🔍 Tìm tên quán, danh mục..."
                                                            style="padding: 8px 15px; border-radius: 8px; background: #222; color: #fff; border: 1px solid #444; width: 200px; font-size: 0.9rem;"
                                                            prop:value=move || search_query.get()
                                                            on:input=move |ev| {
                                                                set_search_query.set(event_target_value(&ev))
                                                            }
                                                        />

                                                        {
                                                            let expenses_for_csv = d.recent_expenses.clone();

                                                            view! {
                                                                <button
                                                                    style="background: #2ecc71; color: #000; font-weight: bold; border: none; padding: 8px 15px; border-radius: 8px; cursor: pointer; font-size: 0.9rem;"
                                                                    on:click=move |_| {
                                                                        let mut csv_content = String::from(
                                                                            "\u{FEFF}Ngày,Danh mục,Cửa hàng,Số tiền,Chi tiết mặt hàng\n",
                                                                        );
                                                                        for exp in &expenses_for_csv {
                                                                            let items_str = exp
                                                                                .items
                                                                                .as_array()
                                                                                .map(|arr| {
                                                                                    arr.iter()
                                                                                        .map(|i| {
                                                                                            format!(
                                                                                                "{} ({} x {})",
                                                                                                i["name"].as_str().unwrap_or(""),
                                                                                                i["quantity"].as_f64().unwrap_or(1.0),
                                                                                                i["price"].as_f64().unwrap_or(0.0),
                                                                                            )
                                                                                        })
                                                                                        .collect::<Vec<_>>()
                                                                                        .join("; ")
                                                                                })
                                                                                .unwrap_or_default();
                                                                            csv_content
                                                                                .push_str(
                                                                                    &format!(
                                                                                        "{},\"{}\",\"{}\",{},\"{}\"\n",
                                                                                        exp.bill_date,
                                                                                        exp.category_name.replace("\"", "\"\""),
                                                                                        exp.merchant.replace("\"", "\"\""),
                                                                                        exp.amount,
                                                                                        items_str.replace("\"", "\"\""),
                                                                                    ),
                                                                                );
                                                                        }
                                                                        let blob = web_sys::Blob::new_with_str_sequence(
                                                                                &js_sys::Array::of1(
                                                                                    &wasm_bindgen::JsValue::from_str(&csv_content),
                                                                                ),
                                                                            )
                                                                            .unwrap();
                                                                        let url = web_sys::Url::create_object_url_with_blob(&blob)
                                                                            .unwrap();
                                                                        let document = web_sys::window()
                                                                            .unwrap()
                                                                            .document()
                                                                            .unwrap();
                                                                        let a = document
                                                                            .create_element("a")
                                                                            .unwrap()
                                                                            .dyn_into::<web_sys::HtmlAnchorElement>()
                                                                            .unwrap();
                                                                        a.set_href(&url);
                                                                        a.set_download(
                                                                            &format!("ThuChi_{}.csv", selected_month.get()),
                                                                        );
                                                                        a.click();
                                                                        web_sys::Url::revoke_object_url(&url).unwrap();
                                                                    }
                                                                >
                                                                    "📥 Xuất CSV"
                                                                </button>
                                                            }
                                                        }
                                                    </div>
                                                </div>

                                                {
                                                    let list_for_filter = d.recent_expenses.clone();
                                                    move || {
                                                        let filtered_expenses = list_for_filter
                                                            .clone()
                                                            .into_iter()
                                                            .filter(|exp| {
                                                                let query = remove_accents(&search_query.get());
                                                                if query.is_empty() {
                                                                    return true;
                                                                }
                                                                let norm_merchant = remove_accents(&exp.merchant);
                                                                let norm_category = remove_accents(&exp.category_name);
                                                                norm_merchant.contains(&query)
                                                                    || norm_category.contains(&query)
                                                            })
                                                            .collect::<Vec<_>>();
                                                        if filtered_expenses.is_empty() {

                                                            view! {
                                                                <div style="background: #1a1a1a; border: 1px dashed #444; border-radius: 12px; padding: 40px; text-align: center;">
                                                                    <span style="font-size: 2rem; opacity: 0.5;">"📭"</span>
                                                                    <p style="color: #888; margin-top: 10px;">
                                                                        {if search_query.get().is_empty() {
                                                                            "Không có giao dịch nào trong tháng này."
                                                                        } else {
                                                                            "Không tìm thấy kết quả phù hợp."
                                                                        }}
                                                                    </p>
                                                                </div>
                                                            }
                                                                .into_view()
                                                        } else {
                                                            view! {
                                                                <div class="transactions-list">
                                                                    {filtered_expenses
                                                                        .into_iter()
                                                                        .map(|exp| {
                                                                            let items_array = exp
                                                                                .items
                                                                                .as_array()
                                                                                .cloned()
                                                                                .unwrap_or_default();
                                                                            let has_items = !items_array.is_empty();
                                                                            let exp_id = exp.id;
                                                                            let exp_merchant = exp.merchant.clone();
                                                                            let exp_amount = exp.amount;
                                                                            let exp_date = exp.bill_date.clone();
                                                                            let exp_cat_name = exp.category_name.clone();
                                                                            let items_for_edit = items_array.clone();

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

                                                                                                <button
                                                                                                    class="btn-edit-tx"
                                                                                                    disabled=move || is_uploading.get()
                                                                                                    style=move || {
                                                                                                        format!(
                                                                                                            "background: #3498db; color: #fff; border: none; padding: 2px 8px; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: bold; {}",
                                                                                                            if is_uploading.get() { "opacity: 0.5" } else { "" },
                                                                                                        )
                                                                                                    }
                                                                                                    on:click=move |_| {
                                                                                                        set_edit_id.set(exp_id);
                                                                                                        set_edit_merchant.set(exp_merchant.clone());
                                                                                                        set_edit_amount.set(exp_amount.to_string());
                                                                                                        set_edit_date.set(exp_date.clone());
                                                                                                        set_edit_category.set(exp_cat_name.clone());
                                                                                                        let mut current_id = 0;
                                                                                                        let mapped_items = items_for_edit
                                                                                                            .clone()
                                                                                                            .into_iter()
                                                                                                            .map(|val| {
                                                                                                                let id = current_id;
                                                                                                                current_id += 1;
                                                                                                                EditItemSignal {
                                                                                                                    id,
                                                                                                                    name: create_rw_signal(
                                                                                                                        val["name"].as_str().unwrap_or("").to_string(),
                                                                                                                    ),
                                                                                                                    qty: create_rw_signal(
                                                                                                                        val["quantity"].as_f64().unwrap_or(1.0),
                                                                                                                    ),
                                                                                                                    price: create_rw_signal(
                                                                                                                        val["price"].as_f64().unwrap_or(0.0),
                                                                                                                    ),
                                                                                                                }
                                                                                                            })
                                                                                                            .collect::<Vec<_>>();
                                                                                                        set_edit_next_id.set(current_id);
                                                                                                        set_edit_items.set(mapped_items);
                                                                                                        set_edit_modal_open.set(true);
                                                                                                    }
                                                                                                >
                                                                                                    "Sửa"
                                                                                                </button>

                                                                                                <button
                                                                                                    class="btn-delete-tx"
                                                                                                    disabled=move || is_uploading.get()
                                                                                                    style=move || {
                                                                                                        format!(
                                                                                                            "background: #ff4d4d; color: #fff; border: none; padding: 2px 8px; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: bold; {}",
                                                                                                            if is_uploading.get() { "opacity: 0.5" } else { "" },
                                                                                                        )
                                                                                                    }
                                                                                                    on:click=move |_| {
                                                                                                        set_is_uploading.set(true);
                                                                                                        let dom = api_domain.get_value();
                                                                                                        spawn_local(async move {
                                                                                                            let window = web_sys::window().unwrap();
                                                                                                            let confirm = window
                                                                                                                .confirm_with_message(
                                                                                                                    "Bạn có chắc chắn muốn xóa giao dịch này? Dữ liệu sẽ không thể khôi phục!",
                                                                                                                )
                                                                                                                .unwrap_or(false);
                                                                                                            if !confirm {
                                                                                                                set_is_uploading.set(false);
                                                                                                                return;
                                                                                                            }
                                                                                                            let storage = window.local_storage().unwrap().unwrap();
                                                                                                            let user_id = storage
                                                                                                                .get_item("user_id")
                                                                                                                .ok()
                                                                                                                .flatten()
                                                                                                                .unwrap_or_else(|| "1".to_string());
                                                                                                            let token = storage
                                                                                                                .get_item("auth_token")
                                                                                                                .ok()
                                                                                                                .flatten()
                                                                                                                .unwrap_or_default();
                                                                                                            let url = format!(
                                                                                                                "{}/api/expenses/{}?user_id={}",
                                                                                                                dom,
                                                                                                                exp_id,
                                                                                                                user_id,
                                                                                                            );
                                                                                                            let res = gloo_net::http::Request::delete(&url)
                                                                                                                .header("Authorization", &format!("Bearer {}", token))
                                                                                                                .send()
                                                                                                                .await;
                                                                                                            set_is_uploading.set(false);
                                                                                                            if let Ok(response) = res {
                                                                                                                if check_auth_error(response.status()) {
                                                                                                                    return;
                                                                                                                }
                                                                                                                if response.ok() {
                                                                                                                    dash_resource.refetch();
                                                                                                                } else {
                                                                                                                    let _ = window
                                                                                                                        .alert_with_message("Lỗi mạng khi xóa giao dịch!");
                                                                                                                }
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

                                                                <div
                                                                    class="pagination-controls"
                                                                    style="display: flex; justify-content: center; align-items: center; gap: 15px; margin-top: 20px;"
                                                                >
                                                                    <button
                                                                        disabled=move || current_page.get() <= 1
                                                                        style=move || {
                                                                            format!(
                                                                                "padding: 8px 16px; border-radius: 6px; background: #333; color: #fff; border: none; font-weight: bold; {}",
                                                                                if current_page.get() <= 1 {
                                                                                    "opacity: 0.5; cursor: not-allowed;"
                                                                                } else {
                                                                                    "cursor: pointer;"
                                                                                },
                                                                            )
                                                                        }
                                                                        on:click=move |_| set_current_page.update(|p| *p -= 1)
                                                                    >
                                                                        "◀ Trang trước"
                                                                    </button>

                                                                    <span style="color: #fff; font-weight: bold; font-size: 0.9rem;">
                                                                        {move || {
                                                                            format!(
                                                                                "Trang {} / {}",
                                                                                d.current_page,
                                                                                d.total_pages.max(1),
                                                                            )
                                                                        }}
                                                                    </span>

                                                                    <button
                                                                        disabled=move || (current_page.get() >= d.total_pages)
                                                                        style=move || {
                                                                            format!(
                                                                                "padding: 8px 16px; border-radius: 6px; background: #333; color: #fff; border: none; font-weight: bold; {}",
                                                                                if current_page.get() >= d.total_pages {
                                                                                    "opacity: 0.5; cursor: not-allowed;"
                                                                                } else {
                                                                                    "cursor: pointer;"
                                                                                },
                                                                            )
                                                                        }
                                                                        on:click=move |_| set_current_page.update(|p| *p += 1)
                                                                    >
                                                                        "Trang sau ▶"
                                                                    </button>
                                                                </div>
                                                            }
                                                                .into_view()
                                                        }
                                                    }
                                                }
                                            </div>
                                        </div>
                                    }
                                })
                        }}
                    </Suspense>
                </div>

                <aside class="side-panel">
                    <div class="upload-widget">

                        <div style="display: flex; background: #1a1a1a; border-radius: 8px; padding: 4px; margin-bottom: 20px; border: 1px solid #333;">
                            <button
                                style=move || {
                                    if input_mode.get() == "ai" {
                                        "flex: 1; padding: 8px; background: #333; color: #fff; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    } else {
                                        "flex: 1; padding: 8px; background: transparent; color: #888; border: none; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    }
                                }
                                on:click=move |_| set_input_mode.set("ai")
                            >
                                "📸 Quét AI"
                            </button>
                            <button
                                style=move || {
                                    if input_mode.get() == "manual" {
                                        "flex: 1; padding: 8px; background: #333; color: #fff; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    } else {
                                        "flex: 1; padding: 8px; background: transparent; color: #888; border: none; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    }
                                }
                                on:click=move |_| set_input_mode.set("manual")
                            >
                                "✍️ Ghi tay"
                            </button>
                            <button
                                style=move || {
                                    if input_mode.get() == "sub" {
                                        "flex: 1; padding: 8px; background: #333; color: #fff; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    } else {
                                        "flex: 1; padding: 8px; background: transparent; color: #888; border: none; cursor: pointer; font-weight: bold; font-size: 0.85rem;"
                                    }
                                }
                                on:click=move |_| set_input_mode.set("sub")
                            >
                                "🔄 Định kỳ"
                            </button>
                        </div>

                        // TAB 1: NHẬP THỦ CÔNG
                        <Show when=move || input_mode.get() == "manual">
                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Số tiền (VNĐ)"
                                    </label>
                                    <input
                                        type="number"
                                        placeholder="Ví dụ: 50000"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_amount.get()
                                        on:input=move |ev| {
                                            set_manual_amount.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Tên cửa hàng/mục đích"
                                    </label>
                                    <input
                                        type="text"
                                        placeholder="Ví dụ: Đổ xăng"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_merchant.get()
                                        on:input=move |ev| {
                                            set_manual_merchant.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Danh mục (Ăn uống, Đi lại...)"
                                    </label>
                                    <input
                                        type="text"
                                        placeholder="Ví dụ: Ăn uống"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_category.get()
                                        on:input=move |ev| {
                                            set_manual_category.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Ngày giao dịch"
                                    </label>
                                    <input
                                        type="date"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_date.get()
                                        on:input=move |ev| {
                                            set_manual_date.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <button
                                    style=move || {
                                        format!(
                                            "width: 100%; padding: 12px; border-radius: 6px; background: #f1c40f; color: #000; border: none; font-weight: bold; cursor: pointer; margin-top: 10px; {}",
                                            if is_uploading.get() { "opacity: 0.5" } else { "" },
                                        )
                                    }
                                    disabled=move || is_uploading.get()
                                    on:click=move |_| {
                                        set_is_uploading.set(true);
                                        let domain_url = api_domain.get_value();
                                        let amount = manual_amount
                                            .get()
                                            .parse::<f64>()
                                            .unwrap_or(0.0);
                                        let merchant = manual_merchant.get();
                                        let category_name = manual_category.get();
                                        let date = manual_date.get();
                                        spawn_local(async move {
                                            let window = web_sys::window().unwrap();
                                            let storage = window.local_storage().unwrap().unwrap();
                                            let user_id = storage
                                                .get_item("user_id")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_else(|| "1".to_string());
                                            let token = storage
                                                .get_item("auth_token")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_default();
                                            let req = serde_json::json!(
                                                {
                                                "user_id": user_id, "amount": amount, "merchant": merchant, "category_name": category_name, "bill_date": date
                                            }
                                            );
                                            let res = gloo_net::http::Request::post(
                                                    &format!("{}/api/expenses/manual", domain_url),
                                                )
                                                .header("Authorization", &format!("Bearer {}", token))
                                                .json(&req)
                                                .unwrap()
                                                .send()
                                                .await;
                                            set_is_uploading.set(false);
                                            if let Ok(response) = res {
                                                if check_auth_error(response.status()) {
                                                    return;
                                                }
                                                if response.status() == 200 {
                                                    set_manual_amount.set("".to_string());
                                                    set_manual_merchant.set("".to_string());
                                                    dash_resource.refetch();
                                                } else {
                                                    let _ = window
                                                        .alert_with_message("Lỗi nhập dữ liệu!");
                                                }
                                            }
                                        });
                                    }
                                >
                                    {move || {
                                        if is_uploading.get() {
                                            "Đang lưu..."
                                        } else {
                                            "Thêm giao dịch"
                                        }
                                    }}
                                </button>
                            </div>
                        </Show>

                        // TAB 2: ĐĂNG KÝ HÓA ĐƠN ĐỊNH KỲ
                        <Show when=move || input_mode.get() == "sub">
                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                <div style="background: rgba(46, 204, 113, 0.1); border: 1px solid rgba(46, 204, 113, 0.3); padding: 10px; border-radius: 6px; margin-bottom: 10px;">
                                    <p style="color: #2ecc71; font-size: 0.8rem; margin: 0; line-height: 1.4;">
                                        "💡 Hóa đơn sẽ tự động được thêm vào ngày mùng 1 mỗi tháng. Chuyên dùng cho: Tiền nhà, Internet, Netflix..."
                                    </p>
                                </div>
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Số tiền cố định (VNĐ)"
                                    </label>
                                    <input
                                        type="number"
                                        placeholder="Ví dụ: 99000"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_amount.get()
                                        on:input=move |ev| {
                                            set_manual_amount.set(event_target_value(&ev))
                                        }
                                    />
                                </div>
                                <div>
                                    <label style="color: #aaa; font-size: 0.8rem; margin-bottom: 4px; display: block;">
                                        "Tên dịch vụ định kỳ"
                                    </label>
                                    <input
                                        type="text"
                                        placeholder="Ví dụ: Tiền mạng FPT"
                                        style="width: 100%; padding: 10px; border-radius: 6px; border: 1px solid #444; background: #222; color: #fff; box-sizing: border-box;"
                                        prop:value=move || manual_merchant.get()
                                        on:input=move |ev| {
                                            set_manual_merchant.set(event_target_value(&ev))
                                        }
                                    />
                                </div>

                                <button
                                    style=move || {
                                        format!(
                                            "width: 100%; padding: 12px; border-radius: 6px; background: #2ecc71; color: #000; border: none; font-weight: bold; cursor: pointer; margin-top: 10px; {}",
                                            if is_uploading.get() { "opacity: 0.5" } else { "" },
                                        )
                                    }
                                    disabled=move || is_uploading.get()
                                    on:click=move |_| {
                                        set_is_uploading.set(true);
                                        let domain_url = api_domain.get_value();
                                        let amount = manual_amount
                                            .get()
                                            .parse::<f64>()
                                            .unwrap_or(0.0);
                                        let merchant = manual_merchant.get();
                                        spawn_local(async move {
                                            let window = web_sys::window().unwrap();
                                            let storage = window.local_storage().unwrap().unwrap();
                                            let user_id = storage
                                                .get_item("user_id")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_else(|| "1".to_string());
                                            let token = storage
                                                .get_item("auth_token")
                                                .ok()
                                                .flatten()
                                                .unwrap_or_default();
                                            let req = serde_json::json!(
                                                {
                                                "user_id": user_id, 
                                                "amount": amount, 
                                                "merchant": merchant, 
                                                "category_name": "Định kỳ" 
                                            }
                                            );
                                            let res = gloo_net::http::Request::post(
                                                    &format!("{}/api/expenses/subscription", domain_url),
                                                )
                                                .header("Authorization", &format!("Bearer {}", token))
                                                .json(&req)
                                                .unwrap()
                                                .send()
                                                .await;
                                            set_is_uploading.set(false);
                                            if let Ok(response) = res {
                                                if check_auth_error(response.status()) {
                                                    return;
                                                }
                                                if response.status() == 200 {
                                                    set_manual_amount.set("".to_string());
                                                    set_manual_merchant.set("".to_string());
                                                    dash_resource.refetch();
                                                    load_subs();
                                                    let _ = window
                                                        .alert_with_message("Đăng ký thành công!");
                                                } else {
                                                    let _ = window
                                                        .alert_with_message(
                                                            "Lỗi! Hãy kiểm tra lại số tiền.",
                                                        );
                                                }
                                            }
                                        });
                                    }
                                >
                                    {move || {
                                        if is_uploading.get() {
                                            "Đang lưu..."
                                        } else {
                                            "Lưu gói định kỳ"
                                        }
                                    }}
                                </button>

                                <div style="margin-top: 20px; border-top: 1px solid #333; padding-top: 15px;">
                                    <p style="color: #888; font-size: 0.8rem; font-weight: bold; margin-bottom: 10px;">
                                        "CÁC GÓI ĐANG ĐĂNG KÝ"
                                    </p>
                                    <div style="display: flex; flex-direction: column; gap: 8px;">
                                        {move || {
                                            active_subs
                                                .get()
                                                .into_iter()
                                                .map(|sub| {
                                                    let sub_id = sub.id;

                                                    view! {
                                                        <div style="background: #111; padding: 10px; border-radius: 6px; display: flex; justify-content: space-between; align-items: center; border: 1px solid #222;">
                                                            <div>
                                                                <div style="color: #fff; font-size: 0.85rem; font-weight: bold;">
                                                                    {sub.merchant}
                                                                </div>
                                                                <div style="color: #2ecc71; font-size: 0.8rem;">
                                                                    {format_thousands(sub.amount)} "đ"
                                                                </div>
                                                                <div style="color: #555; font-size: 0.7rem;">
                                                                    "Từ: " {sub.start_date}
                                                                </div>
                                                            </div>
                                                            <button
                                                                disabled=move || is_uploading.get()
                                                                style=move || {
                                                                    format!(
                                                                        "background: none; border: 1px solid #444; color: #ff4d4d; padding: 4px 8px; border-radius: 4px; font-size: 0.7rem; {}",
                                                                        if is_uploading.get() {
                                                                            "opacity: 0.5"
                                                                        } else {
                                                                            "cursor: pointer;"
                                                                        },
                                                                    )
                                                                }
                                                                on:click=move |_| {
                                                                    set_is_uploading.set(true);
                                                                    let dom = api_domain.get_value();
                                                                    spawn_local(async move {
                                                                        let window = web_sys::window().unwrap();
                                                                        let confirm = window
                                                                            .confirm_with_message(
                                                                                "Hủy gói này?\n(Hóa đơn của tháng này sẽ giữ nguyên bảo toàn lịch sử. Việc hủy chỉ có tác dụng từ tháng sau!)",
                                                                            )
                                                                            .unwrap_or(false);
                                                                        if confirm {
                                                                            let storage = window.local_storage().unwrap().unwrap();
                                                                            let token = storage
                                                                                .get_item("auth_token")
                                                                                .ok()
                                                                                .flatten()
                                                                                .unwrap_or_default();
                                                                            let res = gloo_net::http::Request::delete(
                                                                                    &format!("{}/api/expenses/subscription/{}", dom, sub_id),
                                                                                )
                                                                                .header("Authorization", &format!("Bearer {}", token))
                                                                                .send()
                                                                                .await;
                                                                            if let Ok(response) = res {
                                                                                if check_auth_error(response.status()) {
                                                                                    return;
                                                                                }
                                                                                load_subs();
                                                                            }
                                                                        }
                                                                        set_is_uploading.set(false);
                                                                    });
                                                                }
                                                            >
                                                                "Hủy"
                                                            </button>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()
                                        }}
                                    </div>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || input_mode.get() == "ai">
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
                                                if !is_uploading.get() {
                                                    set_is_dragging.set(true);
                                                }
                                            }
                                            on:dragleave=move |ev| {
                                                ev.prevent_default();
                                                set_is_dragging.set(false);
                                            }
                                            on:drop=on_drop.clone()
                                        >
                                            <span class="icon">"📄"</span>
                                            <span class="text">"Kéo thả hóa đơn vào đây"</span>
                                            <div
                                                class="action-buttons"
                                                style=move || {
                                                    if is_uploading.get() {
                                                        "pointer-events: none; opacity: 0.5"
                                                    } else {
                                                        ""
                                                    }
                                                }
                                            >
                                                <button class="btn-action" on:click=start_camera.clone()>
                                                    "📸 Chụp ảnh"
                                                </button>
                                                <label class="btn-action">
                                                    <input
                                                        type="file"
                                                        accept="image/*"
                                                        class="hidden"
                                                        disabled=move || is_uploading.get()
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
                        </Show>
                    </div>
                </aside>
            </div>
        </div>
    }.into_view()
}