use crate::components::tak_board_state::TakBoardState;
use crate::tak::TakPlayer;
use dioxus::prelude::*;

#[component]
pub fn TakFlatsCounter() -> Element {
    let board = use_context::<TakBoardState>();

    let flats = use_memo(move || {
        let _ = board.pieces.read();
        let flats = board.count_flats();
        (
            *flats.get(&TakPlayer::White).unwrap_or(&0),
            *flats.get(&TakPlayer::Black).unwrap_or(&0),
        )
    });

    let (white_flats, black_flats) = *flats.read();
    let komi_flats = 2;

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
            if komi_flats > 0 {
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
