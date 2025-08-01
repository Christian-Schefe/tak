use dioxus::prelude::*;
use tak_core::TakTimeMode;

use crate::{
    Route,
    components::WS_CLIENT,
    server::{
        SeekUpdate, ServerError, UserId,
        api::{accept_seek, get_seeks},
    },
};

fn on_seeks_changed(update: SeekUpdate) {
    dioxus::logger::tracing::info!("Seeks updated: {:?}", update);
}

#[component]
pub fn Seeks() -> Element {
    let seeks = use_resource(|| get_seeks());

    use_effect(move || {
        let _ = seeks.read();
        WS_CLIENT.read().as_ref().map(|ws| {
            ws.subscribe("seeks", on_seeks_changed);
        });
    });

    let seek_list = use_memo(move || {
        seeks.read().as_ref().map(|s| match s {
            Ok(Ok(data)) => data.clone(),
            _ => vec![],
        })
    });
    let nav = use_navigator();

    use_effect(move || match &*seeks.read() {
        Some(Ok(Err(ServerError::Unauthorized))) => {
            nav.push(Route::Auth {});
        }
        _ => {}
    });

    let formatted_time_mode = |time_mode: &Option<TakTimeMode>| {
        let Some(time_mode) = time_mode.as_ref() else {
            return "No time limit".to_string();
        };
        let minutes = time_mode.time / 60;
        let seconds = time_mode.time % 60;
        if time_mode.increment > 0 {
            format!("{}:{:02} + {}s", minutes, seconds, time_mode.increment)
        } else {
            format!("{}:{:02}", minutes, seconds)
        }
    };

    let on_click_join = move |user_id: UserId| {
        spawn(async move {
            let res = accept_seek(user_id).await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(())) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(_) => {
                    dioxus::logger::tracing::error!("Cannot join seek.");
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to join seek: {}", e);
                }
            }
        });
    };

    let make_on_click_join =
        move |user_id: UserId| move |_: MouseEvent| on_click_join(user_id.clone());

    let on_click_create = move |_| {
        nav.push(Route::CreateRoomOnline {});
    };

    rsx! {
        div { class: "seeks-view",
            button { class: "primary-button", onclick: on_click_create, "Create Seek" }
            div { class: "seek-list",
                if let Some(data) = &*seek_list.read() {
                    for (opponent_info , seek_settings , can_join) in data.iter() {
                        div {
                            key: opponent_info.user_id.clone(),
                            class: "seek-item",
                            div { class: "seek-info",
                                p { class: "seek-owner", "{opponent_info.username}" }
                                div { class: "seek-details",
                                    p {
                                        "{seek_settings.game_settings.size}x{seek_settings.game_settings.size}"
                                    }
                                    p { "{formatted_time_mode(&seek_settings.game_settings.time_mode)}" }
                                }
                            }
                            div { class: "seek-actions",
                                if *can_join {
                                    button {
                                        class: "primary-button",
                                        onclick: make_on_click_join(opponent_info.user_id.clone()),
                                        "Join"
                                    }
                                }
                            }
                        }
                    }
                    if data.is_empty() {
                        p { "No seeks available." }
                    }
                } else {
                    p { "Loading seeks..." }
                }
            }
        }
    }
}
