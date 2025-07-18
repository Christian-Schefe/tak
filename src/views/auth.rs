use crate::server::auth::{LoginResult, RegisterResult};
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
    callback: impl FnOnce(Result<LoginResult, ServerFnError>) + Send + 'static,
) {
    spawn(async move {
        let res = try_login(username, password).await;
        callback(res);
    });
}

#[server]
async fn try_login(username: String, password: String) -> Result<LoginResult, ServerFnError> {
    use crate::server::auth::handle_try_login;
    let user = handle_try_login(username, password).await?;

    if let LoginResult::Success(user_id) = &user {
        add_session(user_id.to_string()).await?;
    }

    println!("User login: {:?}", user);
    Ok(user)
}

#[cfg(feature = "server")]
async fn add_session(user_id: String) -> Result<String, ServerFnError> {
    use axum::http::StatusCode;
    use tower_cookies::Cookie;
    use uuid::Uuid;

    let store: axum::Extension<crate::server::auth::SessionStore> = extract().await?;
    let cookies: tower_cookies::Cookies = extract()
        .await
        .map_err(|e: (StatusCode, &str)| ServerFnError::new(e.1))?;

    let session_id = Uuid::new_v4().to_string();
    store
        .lock()
        .await
        .insert(session_id.clone(), user_id.to_string());
    let mut cookie = Cookie::new("session_id", session_id.clone());
    cookie.set_http_only(Some(true));
    cookies.add(cookie);
    Ok(session_id)
}

pub fn do_logout(callback: impl FnOnce(Result<(), ServerFnError>) + Send + 'static) {
    spawn(async move {
        let res = logout().await;
        callback(res);
    });
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use axum::http::StatusCode;
    use tower_cookies::Cookie;
    use uuid::Uuid;

    let Some(user): Option<crate::server::auth::AuthenticatedUser> = extract().await.ok() else {
        return Ok(());
    };

    let store: axum::Extension<crate::server::auth::SessionStore> = extract().await?;
    let cookies: tower_cookies::Cookies = extract()
        .await
        .map_err(|e: (StatusCode, &str)| ServerFnError::new(e.1))?;

    let session_id = Uuid::new_v4().to_string();
    store.lock().await.remove(&session_id);

    cookies.remove(Cookie::from("session_id"));

    println!("User logged out: {}", user.0);
    Ok(())
}

fn do_register(
    username: String,
    password: String,
    callback: impl FnOnce(Result<RegisterResult, ServerFnError>) + Send + 'static,
) {
    spawn(async move {
        let res = register(username, password).await;
        callback(res);
    });
}

#[server]
async fn register(username: String, password: String) -> Result<RegisterResult, ServerFnError> {
    use crate::server::auth::handle_register;

    let res = handle_register(username, password).await?;
    println!("User registration: {:?}", res);

    if let RegisterResult::Success(user_id) = &res {
        add_session(user_id.to_string()).await?;
    }

    Ok(res)
}

fn do_check_login(callback: impl FnOnce(Result<Option<String>, ServerFnError>) + Send + 'static) {
    spawn(async move {
        let res = check_login().await;
        callback(res);
    });
}

#[server]
pub async fn check_login() -> Result<Option<String>, ServerFnError> {
    let user: Option<crate::server::auth::AuthenticatedUser> = extract().await.ok();
    Ok(user.map(|u| u.0))
}

#[server]
pub async fn get_session_id() -> Result<Option<String>, ServerFnError> {
    use axum::extract::Extension;

    let Ok(crate::server::auth::AuthenticatedUser(user)) = extract().await else {
        return Ok(None);
    };
    let Extension(session_store): Extension<crate::server::auth::SessionStore> = extract().await?;
    let lock = session_store.lock().await;
    if let Some(session_id) = lock.get(&user) {
        Ok(Some(session_id.clone()))
    } else {
        drop(lock);
        Ok(Some(add_session(user).await?))
    }
}
