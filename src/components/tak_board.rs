use crate::components::TakClock;
use crate::components::tak_board_state::TakBoardState;
use crate::components::tak_flats_counter::TakFlatsCounter;
use crate::components::tak_hand::TakHand;
use crate::components::tak_piece::TakPiece;
use crate::components::tak_tile::TakTile;
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::prelude::*;
use tak_core::{TakCoord, TakPieceVariant, TakPlayer};

#[component]
pub fn TakBoard() -> Element {
    let state = use_context::<TakBoardState>();
    let state_clone = state.clone();

    let data = use_memo(move || {
        let _ = state_clone.on_change.read();
        state_clone
            .with_game(|game| {
                let mut piece_ids = game.pieces.iter().map(|(id, _)| *id).collect::<Vec<_>>();
                piece_ids.sort_unstable();
                (
                    game.game().current_player,
                    game.game().board.size,
                    piece_ids,
                )
            })
            .expect("Game should exist to get board data")
    });

    let (player, size, piece_ids) = data.read().clone();

    let tile_coords = (0..size)
        .rev()
        .flat_map(|j| {
            (0..size).map(move |i| (format!("{},{}", i, j), TakCoord::new(i as i32, j as i32)))
        })
        .collect::<Vec<_>>();

    let state_clone = state.clone();
    let player_names = use_memo(move || {
        let player_info = state_clone.player_info.read();
        let white_player_name = player_info
            .get(&TakPlayer::White)
            .map_or(("Waiting...".to_string(), None), |info| {
                (info.name.clone(), info.rating.map(|x| x.round() as usize))
            });

        let black_player_name = player_info
            .get(&TakPlayer::Black)
            .map_or(("Waiting...".to_string(), None), |info| {
                (info.name.clone(), info.rating.map(|x| x.round() as usize))
            });

        (white_player_name, black_player_name)
    });

    let mut state_clone = state.clone();
    use_effect(move || {
        let _ = state_clone.on_change.read();
        state_clone.correct_selected_piece_type();
    });

    let state_clone = state.clone();
    let show_piece_selector = use_memo(move || !state_clone.is_spectator());

    rsx! {
        div { class: "tak-board-container",
            div { class: "tak-game-info",
                TakClock { player: TakPlayer::White }
                p {
                    class: "tak-player-info left",
                    class: if player == TakPlayer::White { "current-player" } else { "" },
                    "{player_names.read().0.0}"
                    if let Some(rating) = player_names.read().0.1 {
                        span { " ({rating})" }
                    }
                }
                p {
                    class: "tak-player-info right",
                    class: if player == TakPlayer::Black { "current-player" } else { "" },
                    "{player_names.read().1.0}"
                    if let Some(rating) = player_names.read().1.1 {
                        span { " ({rating})" }
                    }
                }
                TakClock { player: TakPlayer::Black }
            }
            div {
                class: "tak-board",
                style: "grid-template-columns: repeat({size}, 1fr); grid-template-rows: repeat({size}, 1fr);",
                for (key , pos) in tile_coords {
                    TakTile { key: "{key}", pos }
                }
                for id in piece_ids {
                    TakPiece { key: "{id}", id }
                }
            }
            div { class: "tak-piece-hand-container",
                TakHand { player: TakPlayer::White }
                TakFlatsCounter {}
                TakHand { player: TakPlayer::Black }
            }
            if *show_piece_selector.read() {
                div { class: "tak-piece-selector",
                    PieceTypeSelectorButton { piece_type: TakPieceVariant::Flat }
                    PieceTypeSelectorButton { piece_type: TakPieceVariant::Wall }
                    PieceTypeSelectorButton { piece_type: TakPieceVariant::Capstone }
                }
            }
        }
    }
}

#[component]
fn PieceTypeSelectorButton(piece_type: TakPieceVariant) -> Element {
    let state = use_context::<TakBoardState>();
    let mut selected_piece_type = state.selected_piece_type.clone();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        let player = state.get_active_local_player();
        state
            .with_game(|game| {
                (
                    game.available_piece_types[player.index()].contains(&piece_type),
                    piece_type == *state.selected_piece_type.read(),
                )
            })
            .expect("Game should exist to get piece selector data")
    });

    let (can_place, is_selected) = data.read().clone();

    let text = match piece_type {
        TakPieceVariant::Flat => "Flat",
        TakPieceVariant::Wall => "Wall",
        TakPieceVariant::Capstone => "Cap",
    };

    rsx! {
        button {
            class: "piece-selector",
            class: if is_selected { "piece-selector-current" } else { "" },
            class: if can_place { "" } else { "piece-selector-disabled" },
            disabled: !can_place,
            onclick: move |_| {
                selected_piece_type.set(piece_type);
            },
            {text}
        }
    }
}
