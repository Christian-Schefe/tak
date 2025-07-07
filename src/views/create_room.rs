use crate::server::room::{create_room, CreateRoomResponse};
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{FaBolt, FaChessBoard, FaClock};
use dioxus_free_icons::Icon;

const CSS: Asset = asset!("/assets/styling/create_room.css");

#[component]
pub fn CreateRoom() -> Element {
    let blitz_modes = vec![(3, 0), (3, 2), (5, 0)];
    let rapid_modes = vec![(10, 0), (15, 10), (30, 0)];
    let categories: Vec<(_, Element, _)> = vec![
        ("Blitz", rsx! {Icon {icon: FaBolt}}, blitz_modes),
        ("Rapid", rsx! {Icon {icon: FaClock}}, rapid_modes),
    ];

    let nav = use_navigator();
    let mut board_size = use_signal(|| 5);
    let mut time_mode = use_signal(|| (10, 0));

    let on_click_create = move |_| {
        spawn(async move {
            let res = create_room().await;
            match res {
                Ok(CreateRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(CreateRoomResponse::Success(_)) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(CreateRoomResponse::AlreadyInRoom) => {
                    dioxus::logger::tracing::error!("Already in a room, cannot create a new one.");
                }
                Ok(CreateRoomResponse::FailedToGenerateId) => {
                    dioxus::logger::tracing::error!("Failed to generate id.");
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to create room: {}", e);
                }
            }
        });
    };

    rsx! {
        div {
            id: "create-room-view",
            h1 {
                "Create Room"
            }
            div {
                id: "board-size-chooser",
                div {
                    class: "category-header",
                    Icon { icon: FaChessBoard },
                    "Board Size"
                }
                div {
                    class: "category-container",
                    for size in 3..=9 {
                        button {
                            class: "board-size-button",
                            onclick: move |_| board_size.set(size),
                            class: if *board_size.read() == size { "current" } else { "" },
                            "{size}"
                        }
                    }
                }
            }
            div {
                id: "time-mode-chooser",
                for category in categories {
                    div {
                        class: "category-header",
                        {category.1}
                        "{category.0}"
                    }
                    div {
                        class: "category-container",
                        for mode in category.2 {
                            button {
                                class: "time-mode-button",
                                onclick: move |_| time_mode.set(mode),
                                class: if *time_mode.read() == mode { "current" } else { "" },
                                if mode.1 == 0 {"{mode.0} mins"} else {"{mode.0} | {mode.1}"},
                            }
                        }
                    }
                }
            }
            button {
                id: "create-room-button",
                onclick: on_click_create,
                "Create Room",
            }
        }
    }
}
