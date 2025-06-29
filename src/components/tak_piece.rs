use crate::components::TakBoardState;
use crate::tak::{Player, TakPieceType};
use dioxus::dioxus_core::Element;
use dioxus::logger::tracing;
use dioxus::prelude::*;

#[component]
pub fn TakPiece(id: usize) -> Element {
    let board = use_context::<TakBoardState>();
    let pieces = board.pieces.read();
    let piece = pieces.get(&id).unwrap();
    let size = *board.size.read();

    let actual_data = use_memo(move || {
        let pieces = board.pieces.read();
        let piece = pieces.get(&id).unwrap();
        let mut position = piece.position;
        let mut height = piece.stack_height;
        if let Some(move_selection) = &*board.move_selection.read() {
            if move_selection.position == piece.position {
                let pieces_at_pos = pieces
                    .iter()
                    .filter(|(_, p)| p.position == piece.position)
                    .collect::<Vec<_>>();
                let this_stack_height = pieces_at_pos
                    .iter()
                    .map(|(_, p)| p.stack_height)
                    .max()
                    .unwrap();
                tracing::info!(
                    "Calculating actual position for piece {} at position {:?} with {} piece tower with stack height {}, move count {}",
                    id, piece.position, pieces_at_pos.len(), this_stack_height, move_selection.count
                );
                let first_dropped_height = this_stack_height + 1 - move_selection.count;
                if first_dropped_height <= piece.stack_height {
                    let drops_needed = piece.stack_height + 1 - first_dropped_height;
                    let is_dropped = move_selection.drops.iter().sum::<usize>() >= drops_needed;
                    if !is_dropped {
                        height += 2;
                    } else if let Some(dir) = move_selection.direction {
                        let mut drop_count = 0;
                        for i in 0..move_selection.drops.len() {
                            drop_count += move_selection.drops[i];
                            if drop_count >= drops_needed {
                                let height_offset =
                                    move_selection.drops[i] - (drop_count - drops_needed);
                                position = position.offset_by(&dir, i).unwrap();
                                height = pieces
                                    .iter()
                                    .filter(|(_, p)| p.position == position)
                                    .map(|(_, p)| p.stack_height + 1)
                                    .max()
                                    .unwrap_or(0)
                                    + height_offset;
                                break;
                            }
                        }
                    }
                }
            }
        }
        (position, height)
    });

    let (actual_pos, actual_stack_height) = *actual_data.read();

    rsx! {
        div {
            class: "tak-piece tak-piece-height-{actual_stack_height}",
            style: format!("width: {}%; height: {}%; transform: translate({}%, calc({}% - {}px)); z-index: {}", 100f32 / size as f32, 100f32 / size as f32, actual_pos.x * 100, (size - actual_pos.y - 1) * 100, actual_stack_height * 10, actual_stack_height),
            div {
                class: "tak-piece-wrapper",
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
}
