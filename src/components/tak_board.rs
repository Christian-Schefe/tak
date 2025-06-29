use crate::components::tak_piece::TakPiece;
use crate::tak::{Direction, Player, TakAction, TakCoord, TakFeedback, TakGame, TakPieceType};
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TakBoardState {
    game: Arc<Mutex<TakGame>>,
    pub player: Signal<Player>,
    pub move_selection: Signal<Option<MoveSelection>>,
    pub selected_piece_type: Signal<TakPieceType>,
    pub size: Signal<usize>,
    pub pieces: Signal<HashMap<usize, TakPieceState>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveSelection {
    pub position: TakCoord,
    pub count: usize,
    pub drops: Vec<usize>,
    pub direction: Option<Direction>,
}

impl TakBoardState {
    pub fn new(size: usize) -> Self {
        TakBoardState {
            game: Arc::new(Mutex::new(TakGame::new(size))),
            player: Signal::new(Player::White),
            move_selection: Signal::new(None),
            selected_piece_type: Signal::new(TakPieceType::Flat),
            size: Signal::new(size),
            pieces: Signal::new(HashMap::new()),
        }
    }

    pub fn debug_board(&self) {
        let game_lock = self.game.lock().unwrap();
        tracing::info!("Current game state: {:?}", game_lock);
        for action in game_lock.actions.iter() {
            tracing::info!("Action: {:?}", action);
        }
    }

    fn on_game_update(&mut self) {
        let game_lock = self.game.lock().unwrap();
        let new_player = game_lock.current_player;
        self.player.set(new_player);
        let pieces = &mut self.pieces.write();
        for y in 0..game_lock.size {
            for x in 0..game_lock.size {
                let pos = TakCoord::new(x, y);
                if let Ok(tower) = game_lock.try_get_tower(&pos) {
                    let height = tower.height();
                    for i in 0..height {
                        let stone = tower.composition[i];
                        let new_piece_type = if i == height - 1 {
                            tower.top_type
                        } else {
                            TakPieceType::Flat
                        };
                        if let Some(piece) = pieces.get_mut(&stone.id) {
                            piece.position = pos;
                            piece.stack_height = i;
                            piece.piece_type = new_piece_type;
                            piece.player = stone.player;
                        } else {
                            pieces.insert(
                                stone.id,
                                TakPieceState {
                                    id: stone.id,
                                    player: stone.player,
                                    piece_type: new_piece_type,
                                    position: pos,
                                    stack_height: i,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn is_empty_tile(&self, pos: TakCoord) -> bool {
        let game_lock = self.game.lock().unwrap();
        game_lock.try_get_tile(&pos).is_ok_and(|x| x.is_none())
    }

    pub fn try_place_move(&mut self, pos: TakCoord, piece_type: TakPieceType) -> TakFeedback {
        let tak_move = TakAction::PlacePiece {
            position: pos,
            piece_type,
        };
        let mut game_lock = self.game.lock().unwrap();
        let res = game_lock.try_play_action(tak_move);
        drop(game_lock);
        if res.is_ok() {
            self.on_game_update();
        }
        res
    }

    pub fn try_do_move(&mut self, pos: TakCoord) -> Option<TakFeedback> {
        let _ = self.add_to_move_selection(pos);
        self.try_do_move_action()
    }

    fn try_do_move_action(&mut self) -> Option<TakFeedback> {
        let move_action = self.move_selection.read().clone()?;
        let drop_sum = move_action.drops.iter().sum::<usize>();
        if drop_sum < move_action.count {
            return None;
        } else if drop_sum > move_action.count || move_action.drops.len() < 2 {
            self.move_selection.write().take();
            return None;
        }
        let action_from = move_action.position;
        let action_take = move_action.count - move_action.drops.first().unwrap_or(&0);
        let action_direction = move_action.direction?;
        let action_drops = move_action
            .drops
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>();
        let action = TakAction::MovePiece {
            from: action_from,
            take: action_take,
            direction: action_direction,
            drops: action_drops.clone(),
        };
        let mut game_lock = self.game.lock().unwrap();
        let res = game_lock.try_play_action(action);
        drop(game_lock);
        self.move_selection.write().take();
        if res.is_ok() {
            self.on_game_update();
        }
        Some(res)
    }

    fn add_to_move_selection(&mut self, pos: TakCoord) -> Option<()> {
        let prev_selection = self.move_selection.read().clone();
        if let Some(selection) = prev_selection {
            if selection.position == pos && selection.drops.len() < 2 {
                let mut move_selection_lock = self.move_selection.write();
                let move_selection = move_selection_lock.as_mut().unwrap();
                move_selection.drops[0] += 1;
                return None;
            }
            if let Some(dir) = &selection.direction {
                let prev_drop_pos = selection
                    .position
                    .offset_by(dir, selection.drops.len() - 1)?;
                if let Some(dir2) = Direction::try_from_diff(&prev_drop_pos, &pos) {
                    if dir2 == *dir {
                        let mut move_selection_lock = self.move_selection.write();
                        let move_selection = move_selection_lock.as_mut().unwrap();
                        move_selection.drops.push(1);
                        return None;
                    }
                } else if prev_drop_pos == pos {
                    let mut move_selection_lock = self.move_selection.write();
                    let move_selection = move_selection_lock.as_mut().unwrap();
                    let last_index = move_selection.drops.len() - 1;
                    move_selection.drops[last_index] += 1;
                    return None;
                }
                self.move_selection.set(None);
                self.try_select_for_move(pos)?;
                return None;
            } else {
                if let Some(dir) = Direction::try_from_diff(&selection.position, &pos) {
                    let mut move_selection_lock = self.move_selection.write();
                    let move_selection = move_selection_lock.as_mut().unwrap();
                    move_selection.direction = Some(dir);
                    move_selection.drops.push(1);
                } else {
                    self.move_selection.set(None);
                    self.try_select_for_move(pos)?;
                }
            }
            return None;
        }
        self.try_select_for_move(pos)?;
        None
    }

    fn try_select_for_move(&mut self, pos: TakCoord) -> Option<()> {
        let game = self.game.lock().unwrap();
        let tower = game.try_get_tower(&pos).ok()?;
        if tower.controlling_player() != *self.player.read() {
            return None;
        }
        self.move_selection.set(Some(MoveSelection {
            position: pos,
            count: tower.height().min(game.size),
            drops: vec![0],
            direction: None,
        }));
        Some(())
    }
}

#[derive(Clone)]
pub struct TakPieceState {
    pub id: usize,
    pub player: Player,
    pub piece_type: TakPieceType,
    pub position: TakCoord,
    pub stack_height: usize,
}

#[component]
pub fn TakBoard() -> Element {
    let mut state = use_context_provider(|| TakBoardState::new(5));
    let state_clone = state.clone();

    let make_on_tile_click = move |i: usize, j: usize| {
        let pos = TakCoord::new(i, j);
        let mut cloned_state = state_clone.clone();
        move |_| {
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

    let selected_tiles = use_memo(move || {
        state
            .move_selection
            .read()
            .as_ref()
            .map(|x| {
                let mut positions = vec![x.position];
                if let Some(dir) = x.direction {
                    for i in 1..x.drops.len() {
                        positions.push(x.position.offset_by(&dir, i).unwrap());
                    }
                }
                positions
            })
            .unwrap_or(vec![])
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
                                    {format!("{}", ('a' as u8 + i as u8) as char)}
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
            {format!("{:?} to move", state.player.read())}
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
        }
    }
}
