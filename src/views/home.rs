use crate::{
    server::{
        api::{get_room, join_room, leave_room},
        ServerError, ROOM_ID_LEN,
    },
    Route,
};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let nav = use_navigator();

    let mut room = use_resource(|| get_room());

    let on_click_create = move |_| {
        nav.push(Route::CreateRoomOnline {});
    };

    let mut room_id_input = use_signal(|| String::new());

    let join_valid = use_memo(move || {
        let room_id = room_id_input.read().trim().to_ascii_uppercase();
        room_id.len() == ROOM_ID_LEN && room_id.chars().all(|c| c.is_ascii_alphanumeric())
    });

    let on_click_join = move |_| {
        let room_id = room_id_input.read().trim().to_ascii_uppercase();
        if !*join_valid.read() {
            return;
        }
        spawn(async move {
            let res = join_room(room_id, false).await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(())) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(Err(e)) => {
                    dioxus::logger::tracing::error!("Failed to join room: {}", e);
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to join room: {}", e);
                }
            }
        });
    };

    let on_click_play_computer = move |_| {
        nav.push(Route::CreateRoomComputer {});
    };
    let on_click_play_local = move |_| {
        nav.push(Route::CreateRoomLocal {});
    };

    let on_click_leave = move |_| {
        spawn(async move {
            let res = leave_room().await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(_)) => {
                    room.restart();
                }
                Ok(Err(e)) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
            }
        });
    };

    let is_logged_out =
        use_memo(move || matches!(&*room.read(), Some(Ok(Err(ServerError::Unauthorized)))));

    let is_loading = use_memo(move || room.read().is_none());

    rsx! {
        div { id: "home-view",
            div { class: "home-options",
                if !*is_loading.read() {
                    if !*is_logged_out.read() {
                        if let Some(Ok(Ok((_, _)))) = &*room.read() {
                            button {
                                onclick: move |_| {
                                    nav.push(Route::PlayOnline {});
                                },
                                "Rejoin Room"
                            }
                            button { onclick: on_click_leave, "Leave Room" }
                        } else {
                            div { id: "home-join-bar",
                                input {
                                    id: "home-room-id-input",
                                    r#type: "text",
                                    value: "{room_id_input}",
                                    maxlength: ROOM_ID_LEN,
                                    oninput: move |e| {
                                        let new_str = e.value().trim().to_ascii_uppercase();
                                        let truncated_str = new_str.chars().take(ROOM_ID_LEN).collect::<String>();
                                        room_id_input.set(truncated_str);
                                    },
                                }
                                button {
                                    class: "primary-button",
                                    onclick: on_click_join,
                                    disabled: !*join_valid.read(),
                                    "Join"
                                }
                            }
                            button { onclick: on_click_create, "Create Room" }
                        }
                    } else {
                        button {
                            class: "primary-button",
                            onclick: move |_| {
                                nav.push(Route::Auth {});
                            },
                            "Login"
                        }
                    }
                }
            }
            div { class: "home-options",
                button { onclick: on_click_play_computer, "Play Computer" }
                button { onclick: on_click_play_local, "Play Local" }
            }
        }
    }
}
