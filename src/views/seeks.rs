use dioxus::prelude::*;
use tak_core::TakTimeMode;
use ws_pubsub::{use_ws_topic_receive, use_ws_topic_send};

use crate::{
    Route,
    server::{
        SeekUpdate, ServerError, UserId,
        api::{MyServerFunctions, accept_seek, get_seeks},
    },
};

#[component]
pub fn Seeks() -> Element {
    let seeks = use_resource(|| get_seeks());

    let seek_list = use_memo(move || {
        seeks.read().as_ref().map(|s| match s {
            Ok(Ok(data)) => data.clone(),
            _ => vec![],
        })
    });
    let nav = use_navigator();
    use_ws_topic_receive::<SeekUpdate, MyServerFunctions, _>("seeks", |x| {
        dioxus::logger::tracing::info!("Received seek update: {:?}", x);
        async move { () }
    });

    let seek_service = use_ws_topic_send::<String>("seeks");
    use_future(move || {
        let seek_service = seek_service.clone();
        async move {
            loop {
                let res = seek_service.send("get_seeks".to_string()).await;
                dioxus::logger::tracing::info!("send: {:?}", res);
                crate::future::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });

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
