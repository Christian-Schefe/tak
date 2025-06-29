use crate::components::tak_piece::TakPiece;
use crate::tak::{Direction, Player, TakAction, TakCoord, TakFeedback, TakGame, TakPieceType};
use dioxus::core_macro::{component, rsx};
use dioxus::dioxus_core::Element;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TakBoardState {
    game: Arc<Mutex<TakGame>>,
    pub player: Signal<Player>,
    pub move_selection: Signal<Option<MoveSelection>>,
    pub selected_piece_type: Signal<TakPieceType>,
    pub size: Signal<usize>,
    pub pieces: Signal<Vec<TakPieceState>>,
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
            pieces: Signal::new(vec![]),
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
        {
            let mut game_lock = self.game.lock().unwrap();
            game_lock.try_play_action(tak_move)?;
            let placed_piece = game_lock.try_get_tower(&pos)?;
            let pieces = &mut self.pieces.write();
            let new_piece = TakPieceState {
                id: pieces.len(),
                player: placed_piece.controlling_player(),
                piece_type,
                position: pos,
                stack_height: 0,
            };
            pieces.push(new_piece);
        }
        self.on_game_update();
        Ok(())
    }

    pub fn try_do_move(&mut self, pos: TakCoord) -> Option<TakFeedback> {
        let _ = self.add_to_move_selection(pos);
        tracing::info!(
            "Move selection after click: {:?}",
            self.move_selection.read()
        );
        self.try_do_move_action()
    }

    fn try_do_move_action(&mut self) -> Option<TakFeedback> {
        let move_action = self.move_selection.read().clone()?;
        tracing::info!("Checking move action: {:?}", move_action);
        let drop_sum = move_action.drops.iter().sum::<usize>();
        if drop_sum < move_action.count {
            return None;
        } else if drop_sum > move_action.count || move_action.drops.len() < 2 {
            self.move_selection.write().take();
            return None;
        }
        tracing::info!("Trying to do move action: {:?}", move_action);
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
        if res.is_ok() {
            self.update_moved_pieces(action_from, action_direction, action_drops);
            self.on_game_update();
        }
        self.move_selection.write().take();
        Some(res)
    }

    fn update_moved_pieces(&mut self, from: TakCoord, direction: Direction, drops: Vec<usize>) {
        let mut pieces = self.pieces.write();
        let positions = (0..drops.len())
            .rev()
            .map(|x| {
                let pos = from.offset_by(&direction, x + 1).unwrap();
                let stack_height = pieces
                    .iter()
                    .filter(|piece| piece.position == pos)
                    .map(|piece| piece.stack_height + 1)
                    .max()
                    .unwrap_or(0);
                tracing::info!(
                    "Position for drop {}: {:?}, stack height: {}",
                    x,
                    pos,
                    stack_height
                );
                (x, pos, stack_height)
            })
            .collect::<Vec<_>>();
        let mut moved_pieces = pieces
            .iter_mut()
            .filter(|piece| piece.position == from)
            .collect::<Vec<_>>();
        moved_pieces.sort_by_key(|x| x.stack_height);
        for (i, pos, stack_height) in positions {
            for j in (0..drops[i]).rev() {
                let piece = moved_pieces.pop().unwrap();
                piece.position = pos;
                piece.stack_height = stack_height + j;
            }
        }
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
    let state = use_context_provider(|| TakBoardState::new(5));
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
    let pieces_rendered = pieces_lock.iter().map(|piece| {
        rsx! {
            TakPiece {
                id: piece.id,
            }
        }
    });

    let size = state.size.read();

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
        }
    }
}
