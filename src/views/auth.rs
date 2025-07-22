use crate::server::api::{get_auth, post_login, post_logout, post_register, ApiResponse};
use crate::server::UserId;
use crate::Route;
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::prelude::*;

#[derive(Clone, Debug, Copy, PartialEq)]
enum AuthState {
    NotAttempted,
    TakenUsername,
    InvalidCredentials,
    Success,
    UnknownError,
}

#[derive(Clone, Debug, Copy, PartialEq)]
struct FormState {
    invalid_username: bool,
    invalid_password: bool,
}

impl FormState {
    fn new() -> Self {
        FormState {
            invalid_username: true,
            invalid_password: true,
        }
    }
    fn is_valid(&self) -> bool {
        !self.invalid_username && !self.invalid_password
    }
}

fn validate_username(username: &str) -> bool {
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    username.len() >= 3 && username.len() <= 100 && username.chars().all(is_valid_char)
}

fn validate_password(password: &str) -> bool {
    password.len() >= 8 && password.len() <= 100
}

#[component]
pub fn Auth() -> Element {
    let mut is_login = use_signal(|| true);
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());

    let show_login = *is_login.read();

    let nav = use_navigator();
    let mut auth_state = use_signal_sync(|| AuthState::NotAttempted);
    let mut form_state = use_signal(|| FormState::new());

    use_effect(move || {
        if let AuthState::Success = *auth_state.read() {
            nav.push(Route::Home {});
        }
    });

    use_effect(move || {
        let mut form_state_write = form_state.write();
        let is_registering = !*is_login.read();
        form_state_write.invalid_username = is_registering && !validate_username(&*username.read());
        form_state_write.invalid_password = is_registering && !validate_password(&*password.read());
    });

    use_effect(move || {
        do_check_login(move |res| {
            if let Ok(Some(user_id)) = res {
                dioxus::logger::tracing::info!("User is logged in: {}", user_id);
                auth_state.set(AuthState::Success);
            }
        })
    });

    let on_login = move |username: String, password: String| {
        let callback = move |res| match res {
            Ok(LoginResult::Success(user_id)) => {
                dioxus::logger::tracing::info!("Login successful: {}", user_id);
                auth_state.set(AuthState::Success);
            }
            Ok(LoginResult::InvalidCredentials) => {
                dioxus::logger::tracing::error!("Login failed: Invalid credentials");
                auth_state.set(AuthState::InvalidCredentials);
            }
            Err(e) => {
                dioxus::logger::tracing::error!("Login error: {}", e);
                auth_state.set(AuthState::UnknownError);
            }
        };

        do_login(username, password, callback);
    };

    let on_register = move |username: String, password: String| {
        if !form_state.read().is_valid() {
            return;
        }
        let callback = move |res| match res {
            Ok(RegisterResult::Success(message)) => {
                dioxus::logger::tracing::info!("Registration successful: {}", message);
                auth_state.set(AuthState::Success);
            }
            Ok(RegisterResult::UserExists) => {
                dioxus::logger::tracing::warn!("Registration failed: Username already taken");
                auth_state.set(AuthState::TakenUsername);
            }
            Ok(RegisterResult::ValidationError) => {
                dioxus::logger::tracing::error!("Registration failed: Validation error");
                auth_state.set(AuthState::UnknownError);
            }
            Err(e) => dioxus::logger::tracing::error!("Registration failed: {}", e),
        };

        do_register(username, password, callback);
    };

    rsx! {
        div {
            class: "auth-container",
            div {
                class: "auth-form",
                h2 {
                    "Tak"
                }
                p {
                    class: "auth-instruction",
                    "Username"
                }
                input {
                    type: "text",
                    class: "auth-input",
                    class: if !show_login && form_state.read().invalid_username { "auth-invalid" },
                    placeholder: "Username",
                    name: "username",
                    oninput: move |e| {
                        username.set(e.value().to_string());
                    }
                }
                p {
                    class: "auth-instruction",
                    "Password"
                }
                input {
                    type: "password",
                    class: "auth-input",
                    class: if !show_login && form_state.read().invalid_password { "auth-invalid" },
                    placeholder: "Password",
                    name: "password",
                    oninput: move |e| {
                        password.set(e.value().to_string());
                    }
                }
                div {
                    class: "auth-validation",
                    {if !show_login && form_state.read().invalid_username {
                        rsx! { p { class: "auth-error", "Username must be between 3 and 100 characters and all characters must be from [a-zA-Z0-9_-]" } }
                    } else {
                        rsx! {}
                    }}
                    {if !show_login && form_state.read().invalid_password {
                        rsx! { p { class: "auth-error", "Password must be between 8 and 100 characters" } }
                    } else {
                        rsx! {}
                    }}
                    {match *auth_state.read() {
                        AuthState::TakenUsername => rsx! { p { class: "auth-error", "Username already taken" } },
                        AuthState::InvalidCredentials => rsx! { p { class: "auth-error", "Invalid credentials" } },
                        _ => rsx! { }
                    }}
                }
                button {
                    type: "submit",
                    class: "auth-button",
                    onclick: move |_| {
                        if show_login {
                            on_login(username.read().clone(), password.read().clone());
                        } else {
                            on_register(username.read().clone(), password.read().clone());
                        }
                    },
                    disabled: !show_login && !form_state.read().is_valid(),
                    {if show_login { "Login" } else { "Register" }}
                }
                button {
                    class: "auth-toggle",
                    onclick: move |_| {
                        is_login.set(!show_login);
                        auth_state.set(AuthState::NotAttempted);
                    },
                    {if show_login { "Register instead" } else { "Login instead" }}
                }
                Link {
                    class: "auth-skip",
                    to: Route::Home {  },
                    "Continue without logging in"
                }
            }
        }
    }
}

fn do_login(
    username: String,
    password: String,
    callback: impl FnOnce(Result<ApiResponse<UserId>, ServerFnError>) + Send + 'static,
) {
    spawn(async move {
        let res = post_login(username, password).await;
        callback(res);
    });
}

pub fn do_logout(callback: impl FnOnce(Result<ApiResponse<()>, ServerFnError>) + Send + 'static) {
    spawn(async move {
        let res = post_logout().await;
        callback(res);
    });
}

fn do_register(
    username: String,
    password: String,
    callback: impl FnOnce(Result<ApiResponse<UserId>, ServerFnError>) + Send + 'static,
) {
    spawn(async move {
        let res = post_register(username, password).await;
        callback(res);
    });
}

fn do_check_login(
    callback: impl FnOnce(Result<ApiResponse<String>, ServerFnError>) + Send + 'static,
) {
    spawn(async move {
        let res = get_auth().await;
        callback(res);
    });
}
