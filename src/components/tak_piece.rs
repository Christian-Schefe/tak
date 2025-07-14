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
            can_be_picked,
            buried_piece_count,
        },
    ) = data.read().clone();

    let height = if is_floating { height + 2 } else { height };
    let z_index = if let Some(z) = z_priority {
        z as i32 + 100
    } else if can_be_picked {
        height as i32 + 50
    } else {
        height as i32
    };

    let buried_limit = 12;

    let style = if can_be_picked {
        format!(
            "width: {}%; height: {}%; transform: translate({}%, calc({}% - {}%)); z-index: {}",
            100f32 / size as f32,
            100f32 / size as f32,
            pos.x * 100,
            (size as i32 - 1 - pos.y) * 100,
            height * 7,
            z_index,
        )
    } else {
        let buried_height_offset = buried_piece_count.saturating_sub(buried_limit - 1);
        format!(
            "width: {}%; height: {}%; transform: translate({}%, calc({}% - {}%)); z-index: {}",
            100f32 / size as f32,
            100f32 / size as f32,
            pos.x * 100 + 35,
            (size as i32 - 1 - pos.y) * 100 + 35,
            height.saturating_sub(buried_height_offset) * 7,
            z_index,
        )
    };

    let class = format!(
        "tak-piece-inner tak-piece-inner-{} tak-piece-inner-{}",
        if can_be_picked {
            match variant {
                TakPieceVariant::Flat => "flat",
                TakPieceVariant::Wall => "wall",
                TakPieceVariant::Capstone => "cap",
            }
        } else if buried_piece_count - height < buried_limit {
            "buried"
        } else {
            "hidden"
        },
        match player {
            TakPlayer::White => "light",
            TakPlayer::Black => "dark",
        },
    );

    rsx! {
        div { class: "tak-piece tak-piece-height-{height}", style,
            div { class: "tak-piece-wrapper",
                div { class }
            }
        }
    }
}
