use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::tak::{Direction, TakCoord, TakGameState, TakPlayer};
use dioxus::logger::tracing;
use dioxus::prelude::*;

#[component]
pub fn TakTile(
    pos: TakCoord,
    is_selected: bool,
    is_highlighted: bool,
    bridges: Option<(TakPlayer, Vec<Direction>)>,
) -> Element {
    let state = use_context::<TakBoardState>();

    let state_clone = state.clone();
    let make_on_tile_click = move |i: usize, j: usize| {
        let pos = TakCoord::new(i, j);
        let mut cloned_state = state_clone.clone();
        move |_| {
            if !*cloned_state.has_started.read()
                || *state.game_state.read() != TakGameState::Ongoing
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

    let rendered_bridges = Direction::all()
        .into_iter()
        .map(|dir| Some(dir))
        .chain([None])
        .map(|dir| {
            let player_class = match bridges
                .iter()
                .find(|(_, d)| dir.is_none() || d.contains(&dir.unwrap()))
                .map(|(p, _)| p)
            {
                Some(TakPlayer::White) => "tak-bridge-light",
                Some(TakPlayer::Black) => "tak-bridge-dark",
                None => "tak-bridge-none",
            }
            .to_string();
            let corner_pairs = [
                (Direction::Up, Direction::Left, "tak-bridge-corner-up-left"),
                (
                    Direction::Up,
                    Direction::Right,
                    "tak-bridge-corner-up-right",
                ),
                (
                    Direction::Down,
                    Direction::Left,
                    "tak-bridge-corner-down-left",
                ),
                (
                    Direction::Down,
                    Direction::Right,
                    "tak-bridge-corner-down-right",
                ),
            ];
            let direction_class = match dir {
                Some(Direction::Up) => "tak-bridge-up",
                Some(Direction::Down) => "tak-bridge-down",
                Some(Direction::Left) => "tak-bridge-left",
                Some(Direction::Right) => "tak-bridge-right",
                None => &corner_pairs
                    .into_iter()
                    .filter_map(|(d1, d2, class_str)| {
                        let has_corner = bridges
                            .as_ref()
                            .map(|(_, directions)| {
                                !directions.contains(&d1) && !directions.contains(&d2)
                            })
                            .unwrap_or(false);
                        if has_corner {
                            Some(class_str)
                        } else {
                            None
                        }
                    })
                    .chain(["tak-bridge-center"])
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            rsx! {
                div {
                    class: "tak-bridge {player_class} {direction_class}",
                }
            }
        });

    rsx! {
        div {
            onclick: make_on_tile_click(pos.x, pos.y),
            class: if (pos.x + pos.y) % 2 == 0 {
                "tak-tile tak-tile-light"
            } else {
                "tak-tile tak-tile-dark"
            },
            class: if is_highlighted {
                "tak-tile-highlight"
            } else {
                ""
            },
            class: if is_selected {
                "tak-tile-selected"
            } else {
                ""
            },
            if pos.y == 0 {
                div {
                    class: "tak-tile-label tak-tile-label-rank",
                    {format!("{}", ('A' as u8 + pos.x as u8) as char)}
                }
            }
            if pos.x == 0 {
                div {
                    class: "tak-tile-label tak-tile-label-file",
                    {format!("{}", *state.size.read() - pos.y)}
                }
            }
            {rendered_bridges}
        }
    }
}
