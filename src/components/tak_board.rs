use crate::components::tak_board_state::TakBoardState;
use crate::components::tak_flats_counter::TakFlatsCounter;
use crate::components::tak_hand::TakHand;
use crate::components::tak_piece::TakPiece;
use crate::components::tak_tile::TakTile;
use crate::components::Clock;
use crate::tak::{TakCoord, TakPieceType, TakPlayer};
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::prelude::*;

#[component]
pub fn TakBoard() -> Element {
    let state = use_context::<TakBoardState>();

    let pieces_lock = state.pieces.read();
    let pieces_rendered = (0..pieces_lock.len()).map(|id| {
        rsx! {
            TakPiece {
                key: "{id}",
                id: id,
            }
        }
    });

    let state_clone = state.clone();
    let highlighted_tiles = use_memo(move || state_clone.get_highlighted_tiles());
    let state_clone = state.clone();
    let selected_tiles = use_memo(move || state_clone.get_selected_tiles());
    let state_clone = state.clone();
    let bridges = use_memo(move || state_clone.get_bridges());

    let size = state.size.read();
    let tile_coords = (0..*size)
        .rev()
        .flat_map(|j| (0..*size).map(move |i| (format!("{},{}", i, j), TakCoord::new(i, j))))
        .collect::<Vec<_>>();

    let player_names = use_memo(move || {
        let player_info = state.player_info.read();
        let white_player_name = player_info
            .get(&TakPlayer::White)
            .map_or("Waiting...".to_string(), |info| info.name.clone());

        let black_player_name = player_info
            .get(&TakPlayer::Black)
            .map_or("Waiting...".to_string(), |info| info.name.clone());

        (white_player_name, black_player_name)
    });

    rsx! {
        div {
            class: "tak-board-container",
            div {
                class: "tak-game-info",
                Clock {
                    player: TakPlayer::White,
                }
                p {
                    class: "tak-player-info left",
                    class: if *state.player.read() == TakPlayer::White {"current-player"} else {""},
                    "{player_names.read().0}"
                }
                p {
                    class: "tak-player-info right",
                    class: if *state.player.read() == TakPlayer::Black {"current-player"} else {""},
                    "{player_names.read().1}"
                }
                Clock {
                    player: TakPlayer::Black,
                }
            }
            div {
                class: "tak-board",
                style: "grid-template-columns: repeat({size}, 1fr); grid-template-rows: repeat({size}, 1fr);",
                for (key, pos) in tile_coords {
                    TakTile {
                        key: "{key}",
                        pos,
                        is_selected: selected_tiles.read().contains(&pos),
                        is_highlighted: highlighted_tiles.read().contains(&pos),
                        bridges: bridges.read().get(&pos).cloned(),
                    }
                }
                {pieces_rendered}
            }
            div {
                class: "tak-piece-hand-container",
                TakHand {
                    player: TakPlayer::White
                }
                TakFlatsCounter {}
                TakHand {
                    player: TakPlayer::Black
                }
            }
            div {
                class: "tak-piece-selector",
                PieceTypeSelectorButton {
                    piece_type: TakPieceType::Flat
                },
                PieceTypeSelectorButton {
                    piece_type: TakPieceType::Wall
                },
                PieceTypeSelectorButton {
                    piece_type: TakPieceType::Capstone
                }
            }
        }
    }
}

#[component]
fn PieceTypeSelectorButton(piece_type: TakPieceType) -> Element {
    let mut state = use_context::<TakBoardState>();
    let available_piece_types = state.available_piece_types.read();
    let can_place = available_piece_types.contains(&piece_type);
    let text = match piece_type {
        TakPieceType::Flat => "Flat",
        TakPieceType::Wall => "Wall",
        TakPieceType::Capstone => "Cap",
    };
    rsx! {
        button {
            class: "piece-selector",
            class: if *state.selected_piece_type.read() == piece_type {
                "piece-selector-current"
            } else {
                ""
            },
            class: if can_place {
                ""
            } else {
                "piece-selector-disabled"
            },
            disabled: !can_place,
            onclick: move |_| {
                state.selected_piece_type.set(piece_type);
            },
            {text}
        }
    }
}
