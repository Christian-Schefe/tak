use chrono::{DateTime, Local};
use dioxus::prelude::*;

use crate::{
    server::{api::get_history, error::ServerError},
    Route,
};

#[component]
pub fn History() -> Element {
    let nav = use_navigator();
    let stats = use_resource(|| async { get_history().await });

    use_effect(move || {
        if let Some(Ok(Err(ServerError::Unauthorized))) = stats.read().as_ref() {
            nav.push(Route::Auth {});
        }
    });

    let data = use_memo(move || {
        stats.read().as_ref().map_or(None, |s| match s {
            Ok(Ok(data)) => Some(data.clone()),
            _ => None,
        })
    });

    rsx! {
        div { id: "history-view",
            if let Some(data) = &*data.read() {
                for game in data {
                    div { class: "history-game-entry",
                        p { {format!("{}", std::convert::Into::<DateTime<Local>>::into(game.timestamp).format("%Y-%m-%d %H:%M:%S"))} }
                        p {
                            "{game.white_player.username} ({game.white_player.rating})"
                        }
                        p {
                            "{game.black_player.username} ({game.black_player.rating})"
                        }
                        Link {
                            to: Route::ReviewBoard {
                                game_id: game.game_id.clone(),
                            },
                            "Review Game"
                        }
                    }
                }
            } else {
                p { "Loading..." }
            }
        }
    }
}
