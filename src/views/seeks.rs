use dioxus::prelude::*;
use tak_core::TakTimeMode;
use ws_pubsub::use_ws_topic_receive;

use crate::{
    Route,
    server::{
        MatchId, MatchInstance, MatchUpdate, PlayerInformation, SeekSettings, SeekUpdate,
        ServerError, UserId,
        api::{
            MATCHES_TOPIC, MyServerFunctions, SEEK_TOPIC, accept_seek, get_matches, get_seeks,
            get_user_id,
        },
    },
};

#[component]
pub fn Seeks() -> Element {
    let seeks = use_resource(|| get_seeks());
    let matches = use_resource(|| get_matches());
    let user_id = use_resource(|| get_user_id());

    let mut seek_list = use_signal(|| Vec::new());
    let mut matches_list = use_signal(|| Vec::new());

    let mut show_seeks = use_signal(|| true);

    use_effect(move || {
        let seeks = seeks.read();
        let Some(fetched_seeks) = seeks.as_ref() else {
            return;
        };
        match fetched_seeks {
            Ok(Ok(data)) => seek_list.set(data.clone()),
            _ => {}
        };
    });

    use_effect(move || {
        let matches = matches.read();
        let Some(fetched_matches) = matches.as_ref() else {
            return;
        };
        match fetched_matches {
            Ok(Ok(data)) => matches_list.set(data.clone()),
            _ => {}
        };
    });

    let nav = use_navigator();
    use_ws_topic_receive::<SeekUpdate, MyServerFunctions, _>(SEEK_TOPIC, move |x| {
        dioxus::logger::tracing::info!("Received seek update: {:?}", x);
        let mut seek_list = seek_list.clone();
        match x {
            SeekUpdate::Created {
                player_info,
                settings,
            } => {
                seek_list.write().push((player_info, settings));
            }
            SeekUpdate::Removed { player_id } => {
                seek_list
                    .write()
                    .retain(|(info, _)| info.user_id != player_id);
            }
        }
        async move { () }
    });

    use_ws_topic_receive::<MatchUpdate, MyServerFunctions, _>(MATCHES_TOPIC, move |x| {
        dioxus::logger::tracing::info!("Received match update: {:?}", x);
        let mut matches_list = matches_list.clone();
        match x {
            MatchUpdate::Created {
                player_info,
                opponent_info,
                match_id,
                settings,
            } => {
                matches_list
                    .write()
                    .push((match_id, player_info, opponent_info, settings));
            }
            MatchUpdate::Removed { match_id } => {
                matches_list.write().retain(|(id, _, _, _)| id != &match_id);
            }
        }
        async move { () }
    });

    use_effect(move || match &*seeks.read() {
        Some(Ok(Err(ServerError::Unauthorized))) => {
            nav.push(Route::Auth {});
        }
        _ => {}
    });

    let on_click_join = move |user_id: UserId| {
        spawn(async move {
            let res = accept_seek(user_id).await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(match_id)) => {
                    nav.push(Route::PlayOnline { match_id });
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

    let make_on_click_spectate = move |match_id: MatchId| {
        move |_: MouseEvent| {
            dioxus::logger::tracing::info!("Spectating match: {:?}", match_id);
        }
    };

    let on_click_create = move |_| {
        nav.push(Route::CreateRoomOnline {});
    };

    rsx! {
        div { class: "seeks-view",
            button {
                class: "primary-button",
                onclick: move |_| {
                    show_seeks.set(true);
                },
                "Seeks"
            }
            button {
                class: "primary-button",
                onclick: move |_| {
                    show_seeks.set(false);
                },
                "Matches"
            }
            if *show_seeks.read() {
                button { class: "primary-button", onclick: on_click_create, "Create Seek" }
                div { class: "seek-list",
                    if let Some(Ok(Ok(user_id))) = user_id.read().as_ref() && seeks.read().is_some() {
                        for (opponent_info , seek_settings) in seek_list.read().iter() {
                            SeekItem {
                                key: "{opponent_info.user_id.clone()}",
                                opponent_info: opponent_info.clone(),
                                seek_settings: seek_settings.clone(),
                                user_id: user_id.clone(),
                                on_click_join: make_on_click_join(opponent_info.user_id.clone()),
                            }
                        }
                        if seek_list.read().is_empty() {
                            p { "No seeks available." }
                        }
                    } else {
                        p { "Loading seeks..." }
                    }
                }
            } else {
                div { class: "match-list",
                    if let Some(Ok(Ok(_))) = user_id.read().as_ref() && matches.read().is_some() {
                        for data in matches_list.iter() {
                            MatchItem {
                                key: "{data.0.clone()}",
                                player_info: data.1.clone(),
                                opponent_info: data.2.clone(),
                                settings: data.3.clone(),
                                on_click_spectate: make_on_click_spectate(data.0.clone()),
                            }
                        }
                        if matches_list.read().is_empty() {
                            p { "No matches available." }
                        }
                    } else {
                        p { "Loading matches..." }
                    }
                }
            }
        }
    }
}

fn formatted_time_mode(time_mode: &Option<TakTimeMode>) -> String {
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
}

#[component]
fn SeekItem(
    opponent_info: PlayerInformation,
    seek_settings: SeekSettings,
    user_id: UserId,
    on_click_join: Callback<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "seek-item",
            div { class: "seek-info",
                p { class: "seek-owner", "{opponent_info.username}" }
                div { class: "seek-details",
                    p { "{seek_settings.game_settings.size}x{seek_settings.game_settings.size}" }
                    p { "{formatted_time_mode(&seek_settings.game_settings.time_mode)}" }
                }
            }
            div { class: "seek-actions",
                if opponent_info.user_id != user_id {
                    button { class: "primary-button", onclick: on_click_join, "Join" }
                }
            }
        }
    }
}

#[component]
fn MatchItem(
    player_info: PlayerInformation,
    opponent_info: PlayerInformation,
    settings: MatchInstance,
    on_click_spectate: Callback<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "match-item",
            div { class: "match-info",
                p { class: "match-players", "{player_info.username} vs {opponent_info.username}" }
                div { class: "match-details",
                    p { "{settings.game_settings.size}x{settings.game_settings.size}" }
                    p { "{formatted_time_mode(&settings.game_settings.time_mode)}" }
                }
            }
            div { class: "match-actions",
                button { class: "primary-button", onclick: on_click_spectate, "Spectate" }
            }
        }
    }
}
