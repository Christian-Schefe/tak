use dioxus::prelude::*;

use crate::components::tak_board_state::TakBoardState;

#[component]
pub fn TakFlatsCounter() -> Element {
    let state = use_context::<TakBoardState>();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        state.with_game(|game| {
            let komi = &game.game().settings.komi;
            (
                game.flat_counts[0],
                game.flat_counts[1],
                if komi.tiebreak {
                    format!("{}.5", komi.amount)
                } else {
                    komi.amount.to_string()
                },
            )
        })
    });

    let (white_flats, black_flats, komi_flats) = data.read().clone();

    rsx! {
        div {
            class: "flats-counter",
            div {
                class: "flats-bar flats-bar-light",
                style: "flex-grow: {white_flats + 1};",
                p {
                    "{white_flats}"
                }
            }
            div {
                class: "flats-bar flats-bar-dark",
                style: "flex-grow: {black_flats + 1};",
                p {
                    "{black_flats}"
                }
            }
            if komi_flats != "0" {
                div {
                    class: "flats-bar flats-bar-komi",
                    style: "flex-grow: {komi_flats};",
                    p {
                        "+{komi_flats}"
                    }
                }
            }
        }
    }
}
