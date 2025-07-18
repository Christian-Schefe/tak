use crate::server::room::{create_room, CreateRoomResponse, RoomSettings};
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{FaBolt, FaChessBoard, FaClock, FaPlusMinus};
use dioxus_free_icons::Icon;
use tak_core::{TakGameSettings, TakKomi, TakTimeMode};

pub static LOCAL_SETTINGS: GlobalSignal<TakGameSettings> =
    GlobalSignal::new(|| TakGameSettings::new(6, None, TakKomi::new(2, false), None));

#[component]
pub fn CreateRoomOnline() -> Element {
    rsx! {
        CreateRoomView { is_local: false }
    }
}

#[component]
pub fn CreateRoomLocal() -> Element {
    rsx! {
        CreateRoomView { is_local: true }
    }
}

#[component]
pub fn CreateRoomComputer() -> Element {
    rsx! {
        CreateRoomView { is_local: true }
    }
}

#[component]
pub fn CreateRoomView(is_local: bool) -> Element {
    let blitz_modes = vec![(3, 0), (3, 2), (5, 0)];
    let rapid_modes = vec![(10, 0), (15, 10), (30, 0)];
    let categories: Vec<(_, Element, _)> = vec![
        ("Blitz", rsx! {Icon {icon: FaBolt}}, blitz_modes),
        ("Rapid", rsx! {Icon {icon: FaClock}}, rapid_modes),
    ];

    let nav = use_navigator();
    let mut board_size = use_signal(|| 5);
    let mut time_mode = use_signal(|| (10, 0));
    let mut komi = use_signal(|| TakKomi::new(2, false));

    let on_click_create = move |_| {
        let time_mode = time_mode.read().clone();
        let time_mode = TakTimeMode::new(time_mode.0 * 60, time_mode.1);
        let board_size = *board_size.read();
        let komi = komi.read().clone();
        let create_room_params = RoomSettings {
            game_settings: TakGameSettings::new(board_size, None, komi, Some(time_mode)),
            first_player_mode: None,
        };
        if is_local {
            let mut local_settings = LOCAL_SETTINGS.write();
            *local_settings = create_room_params.game_settings;
            nav.push(Route::PlayComputer {});
            return;
        }
        spawn(async move {
            let res = create_room(create_room_params).await;
            match res {
                Ok(CreateRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(CreateRoomResponse::Success(_)) => {
                    nav.push(Route::PlayOnline {});
                }
                Ok(CreateRoomResponse::InvalidSettings) => {
                    dioxus::logger::tracing::error!("Invalid settings provided.");
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

    let formatted_komi = use_memo(move || {
        let value = &*komi.read();
        if value.tiebreak {
            format!("{}.5", value.amount)
        } else {
            format!("{}", value.amount)
        }
    });

    rsx! {
        div { id: "create-room-view",
            h1 { "Create Room" }
            div { id: "board-size-chooser",
                div { class: "category-header",
                    Icon { icon: FaChessBoard }
                    "Board Size"
                }
                div { class: "category-container",
                    for size in 3..=8 {
                        button {
                            class: "board-size-button",
                            onclick: move |_| board_size.set(size),
                            class: if *board_size.read() == size { "current" } else { "" },
                            "{size}"
                        }
                    }
                }
            }
            div { id: "time-mode-chooser",
                for category in categories {
                    div { class: "category-header",
                        {category.1}
                        "{category.0}"
                    }
                    div { class: "category-container",
                        for mode in category.2 {
                            button {
                                class: "time-mode-button",
                                onclick: move |_| time_mode.set(mode),
                                class: if *time_mode.read() == mode { "current" } else { "" },
                                if mode.1 == 0 {
                                    "{mode.0} mins"
                                } else {
                                    "{mode.0} | {mode.1}"
                                }
                            }
                        }
                    }
                }
            }
            div { id: "komi-chooser",
                div { class: "category-header",
                    Icon { icon: FaPlusMinus }
                    "Komi"
                }
                div { class: "category-container",
                    input {
                        class: "komi-slider",
                        r#type: "range",
                        min: "0",
                        max: "10",
                        value: "4",
                        oninput: move |e| {
                            let value = e.value().parse::<usize>().unwrap_or(4);
                            let amount = value / 2;
                            let tiebreak = value % 2 == 1;
                            komi.set(TakKomi::new(amount, tiebreak));
                        },
                    }
                    p { class: "komi-value", "{formatted_komi}" }
                }
            }
            button {
                id: "create-room-button",
                class: "primary-button",
                onclick: on_click_create,
                "Create Room"
            }
        }
    }
}
