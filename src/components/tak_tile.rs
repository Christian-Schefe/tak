use crate::components::tak_board_state::TakBoardState;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use tak_core::{TakCoord, TakDir, TakPlayer};

#[component]
pub fn TakTile(pos: TakCoord) -> Element {
    let state = use_context::<TakBoardState>();
    let state_clone = state.clone();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        state
            .with_game(|game| game.tiles.get(&pos).expect("Tile should exist").clone())
            .expect("Game should exist to get tile data")
    });

    let tile = data.read().clone();

    let make_on_tile_click = move |pos: TakCoord| {
        let mut cloned_state = state_clone.clone();
        move |_| {
            if !cloned_state.check_ongoing_game() || !cloned_state.is_local_player_turn() {
                return;
            }
            tracing::info!("Clicked on tile: {:?}", pos);
            if cloned_state.is_place_action(pos) {
                let piece_type = *cloned_state.selected_piece_type.read();
                cloned_state.try_do_local_place(pos, piece_type);
            } else {
                cloned_state.try_do_local_move(pos);
            }
        }
    };

    let dir_to_class = |dir: TakDir| match dir {
        TakDir::Up => "up",
        TakDir::Down => "down",
        TakDir::Left => "left",
        TakDir::Right => "right",
    };

    let center_corner_classes = tile
        .get_center_corners()
        .into_iter()
        .map(|(first, second)| {
            format!(
                "tak-bridge-corner-{}-{}",
                dir_to_class(first),
                dir_to_class(second)
            )
        })
        .collect::<Vec<_>>();

    let player_class = match tile.owner {
        Some(TakPlayer::White) => "tak-bridge-light",
        Some(TakPlayer::Black) => "tak-bridge-dark",
        None => "tak-bridge-none",
    };

    let mut rendered_bridges = tile
        .bridges
        .iter()
        .map(|&(dir, has_bridge)| {
            let direction_class = dir_to_class(dir);
            let player_class = if has_bridge {
                player_class
            } else {
                "tak-bridge-none"
            };
            rsx! {
                div { class: "tak-bridge {player_class} tak-bridge-{direction_class}" }
            }
        })
        .collect::<Vec<_>>();

    rendered_bridges.push({
        let center_corner_classes = center_corner_classes.join(" ");
        rsx! {
            div { class: "tak-bridge tak-bridge-center {player_class} {center_corner_classes}" }
        }
    });

    rsx! {
        div {
            onclick: make_on_tile_click(pos),
            class: if (pos.x + pos.y) % 2 == 1 { "tak-tile tak-tile-light" } else { "tak-tile tak-tile-dark" },
            class: if tile.highlighted || tile.last_action { "tak-tile-highlight" } else { "" },
            class: if tile.selectable { "tak-tile-selected" } else { "" },
            if pos.y == 0 {
                div { class: "tak-tile-label tak-tile-label-rank",
                    {format!("{}", ('A' as u8 + pos.x as u8) as char)}
                }
            }
            if pos.x == 0 {
                div { class: "tak-tile-label tak-tile-label-file",
                    {format!("{}", pos.y + 1)}
                }
            }
            {rendered_bridges.iter()}
        }
    }
}
