use crate::components::{TakBoard, TakWebSocket};
use crate::server::room::{get_players, get_room, GetPlayersResponse, GetRoomResponse};
use crate::tak::{
    Direction, TakAction, TakCoord, TakFeedback, TakGameAPI, TakPieceType, TakPlayer, TimeMode,
    TimedTakGame,
};
use crate::Route;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientGameMessage {
    Move(String),
}

#[derive(Clone)]
pub struct TakBoardState {
    game: Arc<Mutex<TimedTakGame>>,
    pub has_started: Signal<bool>,
    pub player: Signal<TakPlayer>,
    pub player_info: Signal<HashMap<TakPlayer, PlayerInfo>>,
    pub move_selection: Signal<Option<MoveSelection>>,
    pub selected_piece_type: Signal<TakPieceType>,
    pub size: Signal<usize>,
    pub pieces: Signal<HashMap<usize, TakPieceState>>,
    pub message_queue: Signal<Vec<ClientGameMessage>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerType {
    Local,
    Remote,
    Computer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerInfo {
    pub name: String,
    pub player_type: PlayerType,
}

impl PlayerInfo {
    pub fn new(name: String, player_type: PlayerType) -> Self {
        PlayerInfo { name, player_type }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveSelection {
    pub position: TakCoord,
    pub count: usize,
    pub drops: Vec<usize>,
    pub direction: Option<Direction>,
}

impl TakBoardState {
    pub fn new(size: usize, player_info: HashMap<TakPlayer, PlayerInfo>) -> Self {
        let time_mode = TimeMode::new(Duration::from_secs(300), Duration::from_secs(10));
        TakBoardState {
            game: Arc::new(Mutex::new(TimedTakGame::new_game(size, time_mode))),
            has_started: Signal::new(false),
            player: Signal::new(TakPlayer::White),
            player_info: Signal::new(player_info),
            move_selection: Signal::new(None),
            selected_piece_type: Signal::new(TakPieceType::Flat),
            size: Signal::new(size),
            pieces: Signal::new(HashMap::new()),
            message_queue: Signal::new(Vec::new()),
        }
    }

    pub fn start_game(&mut self) {
        self.has_started.set(true);
    }

    pub async fn update_player_info(&mut self) {
        let Ok(res) = get_players().await else {
            tracing::error!("Failed to fetch player info");
            return;
        };
        match res {
            GetPlayersResponse::Success(players) => {
                let mut map = self.player_info.write();
                for (player, info) in players {
                    map.insert(
                        player,
                        PlayerInfo {
                            name: info.username,
                            player_type: if info.is_local {
                                PlayerType::Local
                            } else {
                                PlayerType::Remote
                            },
                        },
                    );
                }
            }
            _ => {}
        };
    }

    pub fn with_game_readonly<R, F: FnOnce(&TimedTakGame) -> R>(&self, func: F) -> R {
        let lock = self.game.lock().unwrap();
        let res = func(lock.deref());
        drop(lock);
        res
    }

    pub fn get_time_remaining(&self, player: TakPlayer) -> Duration {
        let game_lock = self.game.lock().unwrap();
        game_lock.get_time_remaining(player)
    }

    pub fn debug_board(&self) {
        let game_lock = self.game.lock().unwrap();
        tracing::info!("Current game state: {:?}", game_lock);
        for action in game_lock.get_actions().iter() {
            tracing::info!("Action: {:?}", action);
        }
    }

    fn on_finish_move(&mut self, action: &TakAction) {
        self.on_game_update();
        self.message_queue
            .push(ClientGameMessage::Move(action.to_ptn()));
    }

    fn on_game_update(&mut self) {
        let game_lock = self.game.lock().unwrap();
        let new_player = game_lock.current_player();
        self.player.set(new_player);
        let mut pieces = self.pieces.write();
        let size = game_lock.size();
        for y in 0..size {
            for x in 0..size {
                Self::on_update_square(&game_lock, &mut pieces, y, x);
            }
        }
    }

    fn on_update_square(
        game_lock: &MutexGuard<TimedTakGame>,
        pieces: &mut Write<HashMap<usize, TakPieceState>>,
        y: usize,
        x: usize,
    ) {
        let pos = TakCoord::new(x, y);
        if let Some(tower) = game_lock.try_get_tower(pos) {
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
                    let new_stone = TakPieceState {
                        player: stone.player,
                        piece_type: new_piece_type,
                        position: pos,
                        stack_height: i,
                    };
                    pieces.insert(stone.id, new_stone);
                }
            }
        }
    }

    pub fn is_empty_tile(&self, pos: TakCoord) -> bool {
        let game_lock = self.game.lock().unwrap();
        game_lock.try_get_tower(pos).is_none()
    }

    pub fn try_do_action(&mut self, action: &TakAction) -> TakFeedback {
        let mut game_lock = self.game.lock().unwrap();
        game_lock.try_do_action(action.clone())?;
        drop(game_lock);
        self.on_finish_move(action);
        Ok(())
    }

    pub fn try_place_move(&mut self, pos: TakCoord, piece_type: TakPieceType) -> TakFeedback {
        let tak_move = TakAction::PlacePiece {
            position: pos,
            piece_type,
        };
        let mut game_lock = self.game.lock().unwrap();
        let res = game_lock.try_do_action(tak_move.clone());
        drop(game_lock);
        if res.is_ok() {
            self.on_finish_move(&tak_move);
        }
        res.map(|_| ())
    }

    pub fn try_do_move(&mut self, pos: TakCoord) -> Option<TakFeedback> {
        let _ = self.add_to_move_selection(pos);
        self.try_do_move_action()
    }

    fn try_do_move_action(&mut self) -> Option<TakFeedback> {
        let move_action = self.move_selection.read().clone()?;
        let drop_sum = move_action.drops.iter().sum::<usize>();
        tracing::info!("selection: {:?}", move_action);
        if drop_sum < move_action.count {
            return None;
        } else if drop_sum > move_action.count || move_action.count == 0 {
            self.move_selection.write().take();
            return None;
        }
        let action = TakAction::MovePiece {
            from: move_action.position,
            take: move_action.count,
            direction: move_action.direction?,
            drops: move_action.drops,
        };
        let mut game_lock = self.game.lock().unwrap();
        let res = game_lock.try_do_action(action.clone());
        drop(game_lock);
        self.move_selection.write().take();
        if res.is_ok() {
            self.on_finish_move(&action);
        }
        Some(res.map(|_| ()))
    }

    pub fn can_drop_at(&self, prev_selection: &MoveSelection, pos: TakCoord) -> bool {
        let game_lock = self.game.lock().unwrap();
        let top = game_lock
            .try_get_tower(prev_selection.position)
            .unwrap()
            .top_type;
        let left = prev_selection.count - prev_selection.drops.iter().sum::<usize>();
        if let Some(target_tower) = game_lock.try_get_tower(pos) {
            if target_tower.top_type == TakPieceType::Flat {
                true
            } else if target_tower.top_type == TakPieceType::Wall
                && top == TakPieceType::Capstone
                && left == 1
            {
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    fn add_to_move_selection(&mut self, pos: TakCoord) -> Option<()> {
        let prev_selection = self.move_selection.read().clone();
        let Some(selection) = prev_selection.as_ref() else {
            return self.try_select_for_move(pos);
        };

        if selection.position == pos && selection.drops.len() == 0 && selection.count > 1 {
            self.move_selection.with_mut(|m| {
                m.as_mut().unwrap().count -= 1;
            });
            return None;
        }

        if let Some(dir) = &selection.direction {
            let prev_drop_pos = selection.position.offset_by(dir, selection.drops.len())?;
            if let Some(dir2) = Direction::try_from_diff(&prev_drop_pos, &pos) {
                if dir2 == *dir && self.can_drop_at(selection, pos) {
                    self.move_selection.with_mut(|m| {
                        m.as_mut().unwrap().drops.push(1);
                    });
                    return Some(());
                }
            } else if prev_drop_pos == pos {
                self.move_selection.with_mut(|m| {
                    *m.as_mut().unwrap().drops.last_mut().unwrap() += 1;
                });
                return Some(());
            }
        } else if let Some(dir) = Direction::try_from_diff(&selection.position, &pos) {
            if self.can_drop_at(selection, pos) {
                self.move_selection.with_mut(|m| {
                    let m = m.as_mut().unwrap();
                    m.drops.push(1);
                    m.direction = Some(dir);
                });
                return Some(());
            }
        }

        self.move_selection.set(None);
        None
    }

    fn try_select_for_move(&mut self, pos: TakCoord) -> Option<()> {
        let game = self.game.lock().unwrap();
        let tower = game
            .try_get_tower(pos)
            .filter(|t| t.controlling_player() == *self.player.read())?;

        self.move_selection.set(Some(MoveSelection {
            position: pos,
            count: tower.height().min(game.size()),
            drops: vec![],
            direction: None,
        }));
        Some(())
    }
}

#[derive(Clone)]
pub struct TakPieceState {
    pub player: TakPlayer,
    pub piece_type: TakPieceType,
    pub position: TakCoord,
    pub stack_height: usize,
}

const CSS: Asset = asset!("/assets/styling/computer.css");

#[component]
pub fn PlayComputer() -> Element {
    let mut player_info = HashMap::new();
    player_info.insert(
        TakPlayer::White,
        PlayerInfo::new("You".to_string(), PlayerType::Local),
    );
    player_info.insert(
        TakPlayer::Black,
        PlayerInfo::new("Computer".to_string(), PlayerType::Computer),
    );
    use_context_provider(|| TakBoardState::new(5, player_info));
    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-computer",
            TakBoard {}
        }
    }
}

#[component]
pub fn PlayOnline() -> Element {
    let room = use_server_future(|| get_room())?;
    let nav = use_navigator();

    let player_info = HashMap::new();
    let board = use_context_provider(|| TakBoardState::new(5, player_info));

    use_effect(move || {
        let mut board = board.clone();
        spawn(async move {
            board.update_player_info().await;
        });
    });

    let room_id = use_memo(move || {
        if let Some(Ok(GetRoomResponse::Success(id))) = room.read().as_ref() {
            Some(id.clone())
        } else {
            None
        }
    });

    use_effect(move || match room.read().as_ref() {
        Some(Ok(GetRoomResponse::Unauthorized)) => {
            nav.replace(Route::Auth {});
        }
        Some(Ok(GetRoomResponse::NotInARoom)) => {
            nav.replace(Route::Home {});
        }
        _ => {}
    });

    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-computer",
            if let Some(room) = room_id.read().as_ref() {
                h2 {
                    "Room ID: {room}"
                }
                TakBoard {
                }
                TakWebSocket {}
            } else {
                h1 { "No room found or not connected." }
            }
        }
    }
}
