use crate::components::TakBoardState;
use crate::tak::{Player, TakPieceType};
use dioxus::dioxus_core::Element;
use dioxus::prelude::*;

#[component]
pub fn TakPiece(id: usize) -> Element {
    let board = use_context::<TakBoardState>();
    let mut piece = (board.pieces)()[id];
    let size = (board.size)();

    rsx! {
        div {
            class: "tak-piece",
            style: format!("width: {}%; height: {}%; transform: translate({}%, calc({}% - {}px)); z-index: {}", 100f32 / size as f32, 100f32 / size as f32, piece.position.0 * 100, piece.position.1 * 100, piece.stack_height * 5, piece.stack_height),
            div {
                class: format!("tak-piece-inner tak-piece-inner-{} tak-piece-inner-{}", match piece.piece_type {
                    TakPieceType::Flat => "flat",
                    TakPieceType::Wall => "wall",
                    TakPieceType::Capstone => "cap",
                }, match piece.player {
                    Player::White => "light",
                    Player::Black => "dark",
                }),
            }
        }
    }
}
