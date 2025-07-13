use crate::components::tak_board_state::TakBoardState;
use dioxus::prelude::*;
use tak_core::{TakPieceVariant, TakPlayer, TakUIPiece};

#[component]
pub fn TakPiece(id: usize) -> Element {
    let state = use_context::<TakBoardState>();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        state
            .with_game(|game| {
                (
                    game.game().board.size,
                    game.pieces.get(&id).expect("Piece should exist").clone(),
                )
            })
            .expect("Game should exist to get piece data")
    });

    let (
        size,
        TakUIPiece {
            height,
            variant,
            player,
            pos,
            is_floating,
            z_priority,
        },
    ) = data.read().clone();

    let height = if is_floating { height + 2 } else { height };
    let z_index = if let Some(z) = z_priority {
        z + 50
    } else {
        height
    };

    rsx! {
        div {
            class: "tak-piece tak-piece-height-{height}",
            style: format!(
                "width: {}%; height: {}%; transform: translate({}%, calc({}% - {}%)); z-index: {}",
                100f32 / size as f32,
                100f32 / size as f32,
                pos.x * 100,
                (size as i32 - 1 - pos.y) * 100,
                height * 7,
                z_index,
            ),
            div { class: "tak-piece-wrapper",
                div {
                    class: format!(
                        "tak-piece-inner tak-piece-inner-{} tak-piece-inner-{}",
                        match variant {
                            TakPieceVariant::Flat => "flat",
                            TakPieceVariant::Wall => "wall",
                            TakPieceVariant::Capstone => "cap",
                        },
                        match player {
                            TakPlayer::White => "light",
                            TakPlayer::Black => "dark",
                        },
                    ),
                }
            }
        }
    }
}
