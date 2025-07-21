use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Route;

#[component]
pub fn Stats() -> Element {
    let nav = use_navigator();
    let stats = use_resource(|| async { get_stats().await });

    use_effect(move || {
        if let Some(stats) = stats.read().as_ref() {
            if let Ok(StatsResponse::Unauthorized) = stats {
                nav.push(Route::Auth {});
            }
        }
    });

    let data = use_memo(move || {
        stats.read().as_ref().map_or(None, |s| match s {
            Ok(StatsResponse::Success(data)) => Some(data.clone()),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsData {
    pub rating: f64,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatsResponse {
    Success(StatsData),
    Unauthorized,
}

#[server]
async fn get_stats() -> Result<StatsResponse, ServerFnError> {
    use crate::server::player::get_or_insert_player;
    let Some(user): Option<crate::server::auth::AuthenticatedUser> = extract().await.ok() else {
        return Ok(StatsResponse::Unauthorized);
    };
    let player = get_or_insert_player(&user.0).await?;
    Ok(StatsResponse::Success(StatsData {
        rating: player.rating,
        wins: player.wins,
        losses: player.losses,
        draws: player.draws,
    }))
}
