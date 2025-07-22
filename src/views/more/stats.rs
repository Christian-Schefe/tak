use dioxus::prelude::*;

use crate::{
    server::{
        api::{get_stats, ApiResponse},
        error::ServerError,
    },
    Route,
};

#[component]
pub fn Stats() -> Element {
    let nav = use_navigator();
    let stats = use_resource(|| async { get_stats().await });

    use_effect(move || {
        if let Some(Ok(ApiResponse::Error(ServerError::Unauthorized))) = stats.read().as_ref() {
            nav.push(Route::Auth {});
        }
    });

    let data = use_memo(move || {
        stats.read().as_ref().map_or(None, |s| match s {
            Ok(ApiResponse::Success(data)) => Some(data.clone()),
            _ => None,
        })
    });

    rsx! {
        div { id: "stats-view",
            if let Some(data) = &*data.read() {
                h2 { "Rating" }
                p { "{data.rating.round() as usize}" }
                h2 { "Games" }
                div { id: "games-grid",
                    p { "Wins" }
                    p { "Losses" }
                    p { "Draws" }
                    p { "{data.wins}" }
                    p { "{data.losses}" }
                    p { "{data.draws}" }
                }
            } else {
                p { "Loading..." }
            }
        }
    }
}
