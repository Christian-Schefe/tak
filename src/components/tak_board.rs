use crate::components::tak_piece::TakPiece;
use crate::components::Clock;
use crate::tak::{Direction, TakCoord, TakGameAPI, TakPieceType};
use crate::views::{PlayerInfo, PlayerType, TakBoardState};
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::logger::tracing;
use dioxus::prelude::*;

#[component]
pub fn TakBoard() -> Element {
    let mut state = use_context::<TakBoardState>();
    let state_clone = state.clone();

    let make_on_tile_click = move |i: usize, j: usize| {
        let pos = TakCoord::new(i, j);
        let mut cloned_state = state_clone.clone();
        move |_| {
            if !*cloned_state.has_started.read() {
                return;
            }
            let Some(PlayerInfo {
                name: _,
                player_type: PlayerType::Local,
            }) = cloned_state
                .player_info
                .read()
                .get(&*cloned_state.player.read())
            else {
                return;
            };
            tracing::info!("Clicked on tile: {:?}", pos);
            if cloned_state.is_empty_tile(pos) && cloned_state.move_selection.read().is_none() {
                let piece_type = *cloned_state.selected_piece_type.read();
                if let Err(e) = cloned_state.try_place_move(pos, piece_type) {
                    tracing::error!("Failed to place piece: {:?}", e);
                }
            } else {
                if let Some(Err(e)) = cloned_state.try_do_move(pos) {
                    tracing::error!("Failed to do move: {:?}", e);
                }
            }
        }
    };

    let pieces_lock = state.pieces.read();
    let pieces_rendered = (0..pieces_lock.len()).map(|id| {
        rsx! {
            TakPiece {
                key: "{id}",
                id: id,
            }
        }
    });

    let size = state.size.read();

    let state_clone = state.clone();

    let selected_tiles = use_memo(move || {
        let player = *state.player.read();
        let size = *state.size.read();
        state_clone
            .move_selection
            .read()
            .as_ref()
            .map(|m| {
                let mut positions = vec![];
                if let Some(dir) = m.direction {
                    let offset_pos = m.position.offset_by(&dir, m.drops.len()).unwrap();
                    positions.push(offset_pos);
                    if let Some(pos) = offset_pos.offset_by(&dir, 1) {
                        if state_clone.can_drop_at(m, pos) {
                            positions.push(pos);
                        }
                    }
                } else {
                    for dir in Direction::all() {
                        if let Some(pos) = m.position.offset_by(&dir, 1) {
                            if state_clone.can_drop_at(m, pos) {
                                positions.push(pos);
                            }
                        }
                    }
                }
                positions
            })
            .unwrap_or_else(|| {
                let mut place_positions = vec![];
                state_clone.with_game_readonly(|game| {
                    for y in 0..size {
                        for x in 0..size {
                            let pos = TakCoord::new(x, y);
                            if game
                                .try_get_tower(pos)
                                .is_some_and(|t| t.controlling_player() == player)
                            {
                                place_positions.push(pos);
                            }
                        }
                    }
                });
                place_positions
            })
    });

    rsx! {
        div {
            class: "tak-board-container",
            div {
                class: "tak-board",
                style: "grid-template-columns: repeat({state.size}, 1fr); grid-template-rows: repeat({state.size}, 1fr);",
                for j in (0..*size).rev() {
                    for i in 0..*size {
                        div {
                            onclick: make_on_tile_click(i, j),
                            class: if (i + j) % 2 == 0 {
                                "tak-tile tak-tile-light"
                            } else {
                                "tak-tile tak-tile-dark"
                            },
                            class: if selected_tiles.read().contains(&TakCoord::new(i, j)) {
                                "tak-tile-selected"
                            } else {
                                ""
                            },
                            if j == 0 {
                                div {
                                    class: "tak-tile-label tak-tile-label-rank",
                                    {format!("{}", ('A' as u8 + i as u8) as char)}
                                }
                            }
                            if i == 0 {
                                div {
                                    class: "tak-tile-label tak-tile-label-file",
                                    {format!("{}", *size - j)}
                                }
                            }
                        }
                    }
                }
                {pieces_rendered}
            }
            Clock {

            }
            div {
                class: "tak-piece-selector",
                button {
                    class: "piece-selector",
                    class: if *state.selected_piece_type.read() == TakPieceType::Flat {
                        "piece-selector-current"
                    } else {
                        ""
                    },
                    onclick: move |_| {
                        state.selected_piece_type.set(TakPieceType::Flat);
                    },
                    "F"
                }
                button {
                    class: "piece-selector",
                    class: if *state.selected_piece_type.read() == TakPieceType::Wall {
                        "piece-selector-current"
                    } else {
                        ""
                    },
                    onclick: move |_| {
                        state.selected_piece_type.set(TakPieceType::Wall);
                    },
                    "W"
                }
                button {
                    class: "piece-selector",
                    class: if *state.selected_piece_type.read() == TakPieceType::Capstone {
                        "piece-selector-current"
                    } else {
                        ""
                    },
                    onclick: move |_| {
                        state.selected_piece_type.set(TakPieceType::Capstone);
                    },
                    "C"
                }
            }
            {format!("{:?} to move", state.player.read())}
        }
    }
}
