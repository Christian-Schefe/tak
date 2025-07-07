use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::components::tak_flats_counter::TakFlatsCounter;
use crate::components::tak_hand::TakHand;
use crate::components::tak_piece::TakPiece;
use crate::components::Clock;
use crate::tak::action::TakActionResult;
use crate::tak::{Direction, TakCoord, TakGameState, TakPieceType, TakPlayer};
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
            if !*cloned_state.has_started.read()
                || *state_clone.game_state.read() != TakGameState::Ongoing
            {
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
                if let Some(Err(e)) = cloned_state.try_do_local_place_move(pos, piece_type) {
                    tracing::error!("Failed to place piece: {:?}", e);
                }
            } else {
                if let Some(Err(e)) = cloned_state.try_do_local_move(pos) {
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
        if !*state_clone.has_started.read()
            || *state_clone.game_state.read() != TakGameState::Ongoing
        {
            return vec![];
        }
        let Some(PlayerInfo {
            name: _,
            player_type: PlayerType::Local,
        }) = state_clone.player_info.read().get(&player)
        else {
            return vec![];
        };
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
                    if game.get_current_move_index() < 2 {
                        return;
                    }
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
    let state_clone = state.clone();
    let highlighted_tiles = use_memo(move || {
        if let TakGameState::Win(winner, _) = *state_clone.game_state.read() {
            state_clone.get_winning_tiles(winner)
        } else if let Some(prev_move) = state_clone.prev_move.read().as_ref() {
            match prev_move {
                TakActionResult::PlacePiece {
                    position,
                    piece_type: _,
                } => {
                    vec![*position]
                }
                TakActionResult::MovePiece {
                    from,
                    direction,
                    drops,
                    take: _,
                    flattened: _,
                } => {
                    let mut positions = from
                        .try_get_positions(direction, drops.len(), *state_clone.size.read())
                        .unwrap_or_else(|| vec![]);
                    positions.push(*from);
                    positions
                }
            }
        } else {
            vec![]
        }
    });

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
                            class: if highlighted_tiles.read().contains(&TakCoord::new(i, j)) {
                                "tak-tile-highlight"
                            } else {
                                ""
                            },
                            class:if selected_tiles.read().contains(&TakCoord::new(i, j)) {
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
