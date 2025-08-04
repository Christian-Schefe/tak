use crate::{
    Route,
    server::{
        ServerError,
        api::{cancel_seek, get_match_id, get_seek},
    },
};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let nav = use_navigator();

    let player_match = use_resource(|| get_match_id());
    let mut seek = use_resource(|| get_seek());

    let on_click_create = move |_| {
        nav.push(Route::CreateRoomOnline {});
    };

    let on_click_cancel = move |_| {
        spawn(async move {
            match cancel_seek().await {
                Ok(Ok(())) => {
                    dioxus::logger::tracing::info!("Seek cancelled successfully");
                }
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.replace(Route::Auth {});
                }
                Ok(Err(e)) => {
                    dioxus::logger::tracing::warn!("Failed to cancel seek: {:?}", e);
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to cancel seek: {:?}", e);
                }
            }
            seek.restart();
        });
    };

    let on_click_play_computer = move |_| {
        nav.push(Route::CreateRoomComputer {});
    };
    let on_click_play_local = move |_| {
        nav.push(Route::CreateRoomLocal {});
    };

    let is_logged_out = use_memo(move || {
        matches!(
            &*player_match.read(),
            Some(Ok(Err(ServerError::Unauthorized)))
        )
    });

    let is_loading = use_memo(move || player_match.read().is_none());

    rsx! {
        div { id: "home-view",
            div { class: "home-options",
                if !*is_loading.read() {
                    if !*is_logged_out.read() {
                        if let Some(Ok(Ok(match_id))) = player_match.read().clone() {
                            button {
                                class: "primary-button",
                                onclick: move |_| {
                                    nav.push(Route::PlayOnline {
                                        match_id: match_id.clone(),
                                    });
                                },
                                "Rejoin Match"
                            }
                        } else if let Some(Ok(Ok(_))) = seek.read().clone() {
                            button {
                                class: "primary-button",
                                onclick: on_click_cancel,
                                "Cancel Seek"
                            }
                        } else {
                            button {
                                class: "primary-button",
                                onclick: on_click_create,
                                "Create Seek"
                            }
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
                } else {
                    button { class: "primary-button", disabled: true, "Loading..." }
                }
                button { onclick: on_click_play_computer, "Play Computer" }
                button { onclick: on_click_play_local, "Play Local" }
            }
        }
    }
}
