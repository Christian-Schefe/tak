use crate::server::room::{
    create_room, get_room, join_room, leave_room, CreateRoomResponse, GetRoomResponse,
    JoinRoomResponse, LeaveRoomResponse,
};
use crate::Route;
use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/styling/home.css");

#[component]
pub fn Home() -> Element {
    let nav = use_navigator();

    let mut room = use_server_future(|| get_room())?;

    use_effect(move || {
        match &*room.read() {
            Some(Ok(GetRoomResponse::Unauthorized)) => {
                dioxus::logger::tracing::error!("Unauthorized access to room.");
                nav.push(Route::Auth {});
            }
            _ => {}
        };
    });

    let on_click_create = move |_| {
        nav.push(Route::CreateRoom {});
    };

    let mut room_id_input = use_signal(|| String::new());

    let join_valid = use_memo(move || {
        let room_id = room_id_input.read().trim().to_ascii_uppercase();
        room_id.len() == 6 && room_id.chars().all(|c| c.is_ascii_alphanumeric())
    });

    let on_click_join = move |_| {
        let room_id = room_id_input.read().trim().to_ascii_uppercase();
        if !*join_valid.read() {
            return;
        }
        spawn(async move {
            let res = join_room(room_id, false).await;
            match res {
                Ok(JoinRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(JoinRoomResponse::Success) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(JoinRoomResponse::AlreadyInRoom) => {
                    dioxus::logger::tracing::error!("Already in a room, cannot join another one.");
                }
                Ok(JoinRoomResponse::RoomFull) => {
                    dioxus::logger::tracing::error!("Room is full.");
                }
                Ok(JoinRoomResponse::RoomNotFound) => {
                    dioxus::logger::tracing::error!("Room not found.");
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to join room: {}", e);
                }
            }
        });
    };

    let on_click_play_computer = move |_| {
        nav.push(Route::PlayComputer {});
    };

    let on_click_leave = move |_| {
        spawn(async move {
            let res = leave_room().await;
            match res {
                Ok(LeaveRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(_) => {
                    room.restart();
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-options",
            if let Some(Ok(GetRoomResponse::Success(_))) = &*room.read() {
                button {
                    onclick: move |_| {
                        nav.push(Route::PlayOnline {});
                    },
                    "Rejoin Room"
                }
                button {
                    onclick: on_click_leave,
                    "Leave Room"
                }
            } else {
                button {
                    onclick: on_click_create,
                    "Create Room"
                }
                button {
                    onclick: on_click_join,
                    disabled: !*join_valid.read(),
                    "Join Room"
                }
                input {
                    type: "text",
                    placeholder: "Enter room ID",
                    id: "room-id-input",
                    oninput: move |e| {
                        room_id_input.set(e.value())
                    }
                }
            }
            button {
                onclick: on_click_play_computer,
                "Play Computer"
            }
        }
    }
}
