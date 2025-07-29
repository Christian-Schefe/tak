use dioxus::prelude::*;

pub const PLAYTAK_AUTH_USER_KEY: &str = "playtak_auth_user";
pub const PLAYTAK_AUTH_PASS_KEY: &str = "playtak_auth_pass";

#[component]
pub fn Settings() -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());

    use_effect(move || {
        if let Ok(user) = crate::storage::get::<String>(PLAYTAK_AUTH_USER_KEY) {
            username.set(user);
        }
        if let Ok(pass) = crate::storage::get::<String>(PLAYTAK_AUTH_PASS_KEY) {
            password.set(pass);
        }
    });

    use_effect(move || {
        if !username.read().is_empty() {
            if let Err(e) = crate::storage::set(PLAYTAK_AUTH_USER_KEY, username.read().clone()) {
                dioxus::logger::tracing::error!("Failed to set username in storage: {}", e);
            }
        }
        if !password.read().is_empty() {
            if let Err(e) = crate::storage::set(PLAYTAK_AUTH_PASS_KEY, password.read().clone()) {
                dioxus::logger::tracing::error!("Failed to set password in storage: {}", e);
            }
        }
    });

    rsx! {
        div { id: "settings-view",
            h1 { "Settings" }
            p { "PlayTak Username" }
            input {
                r#type: "text",
                placeholder: "Username",
                name: "username",
                oninput: move |e| {
                    username.set(e.value().to_string());
                },
                value: username.read().clone(),
            }
            p { "PlayTak Password" }
            input {
                r#type: "password",
                placeholder: "Password",
                name: "password",
                oninput: move |e| {
                    password.set(e.value().to_string());
                },
                value: password.read().clone(),
            }
        }
    }
}
