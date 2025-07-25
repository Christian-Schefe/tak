use dioxus::prelude::*;
use tak_core::TakTimeMode;

use crate::{
    server::{
        api::{get_room_list, join_room},
        ServerError,
    },
    Route,
};

#[component]
pub fn Rooms() -> Element {
    let rooms = use_resource(|| get_room_list());

    let room_list = use_memo(move || {
        rooms.read().as_ref().map(|s| match s {
            Ok(Ok(data)) => data.clone(),
            _ => vec![],
        })
    });
    let nav = use_navigator();

    use_effect(move || match &*rooms.read() {
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

    let on_click_join = move |room_id: String, is_spectator: bool| {
        spawn(async move {
            let res = join_room(room_id, is_spectator).await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(())) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(_) => {
                    dioxus::logger::tracing::error!("Cannot join room.");
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to join room: {}", e);
                }
            }
        });
    };

    let make_on_click_join = move |room_id: String, is_spectator: bool| {
        move |_: MouseEvent| on_click_join(room_id.clone(), is_spectator)
    };

    rsx! {
        div { class: "rooms-view",
            div { class: "room-list",
                if let Some(data) = &*room_list.read() {
                    for room in data.iter() {
                        div { key: room.room_id.clone(), class: "room-item",
                            div { class: "room-info",
                                p { class: "room-code", "{room.room_id}" }
                                div { class: "room-details",
                                    p {
                                        "{room.settings.game_settings.size}x{room.settings.game_settings.size}"
                                    }
                                    p { "{formatted_time_mode(&room.settings.game_settings.time_mode)}" }
                                }
                                div { class: "room-players",
                                    p {
                                        {room.players.iter().map(|x| x.username.clone()).collect::<Vec<_>>().join(", ")}
                                    }
                                }
                            }
                            div { class: "room-actions",
                                if room.can_join {
                                    button {
                                        class: "primary-button",
                                        onclick: make_on_click_join(room.room_id.clone(), false),
                                        "Join"
                                    }
                                }
                                button {
                                    class: "secondary-button",
                                    onclick: make_on_click_join(room.room_id.clone(), true),
                                    "Spectate"
                                }
                            }
                        }
                    }
                    if data.is_empty() {
                        p { "No rooms available." }
                    }
                } else {
                    p { "Loading rooms..." }
                }
            }
        }
    }
}
