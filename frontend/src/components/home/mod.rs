use leptos::*;

#[component]
pub fn Home() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    view! {
        <div class="home-page">
            <h1>"Đây là Module Home"</h1>
            <p>"Style được load từ file _home.scss riêng biệt"</p>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                "Đếm: " {count}
            </button>
        </div>
    }
}