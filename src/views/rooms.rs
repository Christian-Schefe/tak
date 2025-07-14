use dioxus::prelude::*;
use tak_core::TakTimeMode;

use crate::{
    server::room::{get_room_list, join_room, GetRoomListResponse, JoinRoomResponse},
    Route,
};

#[component]
pub fn Rooms() -> Element {
    let rooms = use_resource(|| get_room_list());

    let mut room_list = use_signal(|| Vec::new());
    let nav = use_navigator();

    use_effect(move || match &*rooms.read() {
        Some(Ok(GetRoomListResponse::Success(list))) => {
            room_list.set(list.clone());
        }
        Some(Ok(GetRoomListResponse::Unauthorized)) => {
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
                Ok(JoinRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(JoinRoomResponse::Success) => {
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
            h1 { "Rooms" }
            div { class: "room-list",
                for room in room_list.read().iter() {
                    div { key: room.0.clone(), class: "room-item",
                        div { class: "room-info",
                            p { class: "room-code", "{room.0}" }
                            div { class: "room-details",
                                p { "{room.1.game_settings.size}x{room.1.game_settings.size}" }
                                p { "{formatted_time_mode(&room.1.game_settings.time_mode)}" }
                            }
                            div { class: "room-players",
                                p { {room.2.join(", ")} }
                            }
                        }
                        div { class: "room-actions",
                            button {
                                class: "primary-button",
                                onclick: make_on_click_join(room.0.clone(), false),
                                "Join"
                            }
                            button {
                                class: "secondary-button",
                                onclick: make_on_click_join(room.0.clone(), true),
                                "Spectate"
                            }
                        }
                    }
                }
            }
        }
    }
}
