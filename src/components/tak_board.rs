use crate::components::tak_piece::TakPiece;
use crate::tak::{Player, TakPieceType};
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::html::completions::CompleteWithBraces::div;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct TakBoardState {
    pub size: Signal<i32>,
    pub pieces: Signal<Vec<TakPieceState>>,
}

#[derive(Clone, Copy)]
pub struct TakPieceState {
    pub id: usize,
    pub player: Player,
    pub piece_type: TakPieceType,
    pub position: (i32, i32),
    pub stack_height: i32,
}

#[component]
pub fn TakBoard() -> Element {
    let state = use_context_provider(|| TakBoardState {
        size: Signal::new(5),
        pieces: Signal::new(vec![
            TakPieceState {
                id: 0,
                player: Player::White,
                piece_type: TakPieceType::Flat,
                position: (0, 0),
                stack_height: 0,
            },
            TakPieceState {
                id: 1,
                player: Player::Black,
                piece_type: TakPieceType::Wall,
                position: (2, 3),
                stack_height: 0,
            },
            TakPieceState {
                id: 3,
                player: Player::Black,
                piece_type: TakPieceType::Flat,
                position: (4, 4),
                stack_height: 0,
            },
            TakPieceState {
                id: 4,
                player: Player::White,
                piece_type: TakPieceType::Flat,
                position: (4, 4),
                stack_height: 1,
            },
            TakPieceState {
                id: 2,
                player: Player::White,
                piece_type: TakPieceType::Capstone,
                position: (4, 4),
                stack_height: 2,
            },
        ]),
    });

    rsx! {
        div {
            class: "tak-board",
            style: "grid-template-columns: repeat({state.size}, 1fr); grid-template-rows: repeat({state.size}, 1fr);",
            for i in 0..(state.size)() {
                for j in 0..(state.size)() {
                    div {
                        class: if (i + j) % 2 == 0 {
                            "tak-tile tak-tile-light"
                        } else {
                            "tak-tile tak-tile-dark"
                        },
                        if i + 1 == (state.size)() {
                            div {
                                class: "tak-tile-label tak-tile-label-rank",
                                {format!("{}", ('a' as u8 + j as u8) as char)}
                            }
                        }
                        if j == 0 {
                            div {
                                class: "tak-tile-label tak-tile-label-file",
                                {format!("{}", i + 1)}
                            }
                        }
                    }
                }
            }
            for piece in (state.pieces)().iter() {
                TakPiece {
                    id: piece.id,
                }
            }
        }
    }
}
