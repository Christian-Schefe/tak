use chrono::{DateTime, Local};
use dioxus::prelude::*;

use crate::{
    Route,
    server::{GameInformation, UserId, api::get_history, error::ServerError},
};

#[component]
pub fn History() -> Element {
    let nav = use_navigator();
    let stats = use_resource(|| async { get_history(Some((0, 100))).await });

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

    let get_opponent_info = |game: &GameInformation, user_id: &UserId| {
        if game.white_player.user_id == *user_id {
            format!(
                "{} ({})",
                game.black_player.username, game.black_player.rating
            )
        } else {
            format!(
                "{} ({})",
                game.white_player.username, game.white_player.rating
            )
        }
    };

    rsx! {
        div { id: "history-view",
            if let Some((data, user_id)) = &*data.read() {
                for game in data {
                    div { class: "history-game-entry",
                        p {
                            {
                                format!(
                                    "{}",
                                    std::convert::Into::<DateTime<Local>>::into(game.timestamp)
                                        .format("%Y-%m-%d %H:%M:%S"),
                                )
                            }
                        }
                        p { {get_opponent_info(game, user_id)} }
                        Link {
                            to: Route::ReviewBoard {
                                game_id: game.game_id.clone(),
                            },
                            "Review Game"
                        }
                    }
                }
                if data.len() == 0 {
                    p { "No games found." }
                }
            } else {
                p { "Loading..." }
            }
        }
    }
}
