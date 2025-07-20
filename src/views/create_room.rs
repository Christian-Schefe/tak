use crate::server::room::{create_room, CreateRoomResponse, RoomSettings};
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{
    FaBolt, FaChessBoard, FaClock, FaPalette, FaPlusMinus,
};
use dioxus_free_icons::Icon;
use tak_core::{TakGameSettings, TakKomi, TakPlayer, TakTimeMode};

pub static LOCAL_SETTINGS: GlobalSignal<LocalSettings> = GlobalSignal::new(|| LocalSettings {
    game_settings: TakGameSettings::new(6, None, TakKomi::new(2, false), None),
    first_player_mode: None,
});

#[derive(Debug, Clone, PartialEq)]
pub struct LocalSettings {
    pub game_settings: TakGameSettings,
    pub first_player_mode: Option<TakPlayer>,
}

#[component]
pub fn CreateRoomOnline() -> Element {
    rsx! {
        CreateRoomView { is_local: None }
    }
}

#[component]
pub fn CreateRoomLocal() -> Element {
    rsx! {
        CreateRoomView { is_local: Some(false) }
    }
}

#[component]
pub fn CreateRoomComputer() -> Element {
    rsx! {
        CreateRoomView { is_local: Some(true) }
    }
}

#[component]
pub fn CreateRoomView(is_local: Option<bool>) -> Element {
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
    let mut first_player_mode = use_signal(|| None);

    let on_click_create = move |_| {
        let time_mode = time_mode.read().clone();
        let time_mode = TakTimeMode::new(time_mode.0 * 60, time_mode.1);
        let board_size = *board_size.read();
        let komi = komi.read().clone();
        let first_player_mode = first_player_mode.read().clone();
        let create_room_params = RoomSettings {
            game_settings: TakGameSettings::new(board_size, None, komi, Some(time_mode)),
            first_player_mode,
        };
        if let Some(is_computer) = is_local {
            let mut local_settings = LOCAL_SETTINGS.write();
            *local_settings = LocalSettings {
                game_settings: create_room_params.game_settings,
                first_player_mode,
            };
            if is_computer {
                nav.push(Route::PlayComputer {});
            } else {
                nav.push(Route::PlayLocal {});
            }
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
                    for size in 4..=8 {
                        button {
                            class: "choice-button",
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
                                class: "choice-button",
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
            div { id: "color-chooser",
                div { class: "category-header",
                    Icon { icon: FaPalette }
                    "Color"
                }
                div { class: "category-container",
                    button {
                        class: "choice-button",
                        class: if first_player_mode.read().is_none() { "current" } else { "" },
                        onclick: move |_| {
                            first_player_mode.set(None);
                        },
                        "Random"
                    }
                    button {
                        class: "choice-button",
                        class: if *first_player_mode.read() == Some(TakPlayer::White) { "current" } else { "" },
                        onclick: move |_| {
                            first_player_mode.set(Some(TakPlayer::White));
                        },
                        "White"
                    }
                    button {
                        class: "choice-button",
                        class: if *first_player_mode.read() == Some(TakPlayer::Black) { "current" } else { "" },
                        onclick: move |_| {
                            first_player_mode.set(Some(TakPlayer::Black));
                        },
                        "Black"
                    }
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
