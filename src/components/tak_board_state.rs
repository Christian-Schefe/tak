use crate::server::room::{get_players, GetPlayersResponse};
use crate::tak::action::{TakAction, TakActionResult};
use crate::tak::ptn::Ptn;
use crate::tak::{
    Direction, TakCoord, TakFeedback, TakGame, TakGameState, TakPieceType, TakPlayer, TakSettings,
};
use crate::views::ClientGameMessage;
use dioxus::logger::tracing;
use dioxus::prelude::{Readable, Signal, Writable, WritableVecExt, Write};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

#[derive(Clone)]
pub struct TakBoardState {
    game: Arc<Mutex<TakGame>>,
    pub has_started: Signal<bool>,
    pub game_state: Signal<TakGameState>,
    pub remaining_stones: Signal<HashMap<TakPlayer, (usize, usize)>>,
    pub available_piece_types: Signal<Vec<TakPieceType>>,
    pub player: Signal<TakPlayer>,
    pub player_info: Signal<HashMap<TakPlayer, PlayerInfo>>,
    pub prev_move: Signal<Option<TakActionResult>>,
    pub move_selection: Signal<Option<MoveSelection>>,
    pub selected_piece_type: Signal<TakPieceType>,
    pub size: Signal<usize>,
    pub pieces: Signal<HashMap<usize, TakPieceState>>,
    pub message_queue: Signal<Vec<ClientGameMessage>>,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
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
    pub fn new(settings: TakSettings, player_info: HashMap<TakPlayer, PlayerInfo>) -> Self {
        let game = TakGame::new(settings);
        let remaining_stones = Self::get_remaining_stones(&game);
        let available_piece_types = vec![TakPieceType::Flat];
        TakBoardState {
            size: Signal::new(game.settings.size),
            game: Arc::new(Mutex::new(game)),
            has_started: Signal::new(false),
            game_state: Signal::new(TakGameState::Ongoing),
            remaining_stones: Signal::new(remaining_stones),
            player: Signal::new(TakPlayer::White),
            player_info: Signal::new(player_info),
            move_selection: Signal::new(None),
            available_piece_types: Signal::new(available_piece_types),
            prev_move: Signal::new(None),
            selected_piece_type: Signal::new(TakPieceType::Flat),
            pieces: Signal::new(HashMap::new()),
            message_queue: Signal::new(Vec::new()),
        }
    }

    pub fn replace_settings_if_not_started(&mut self, settings: &TakSettings) {
        if !*self.has_started.read() {
            let mut game_lock = self.game.lock().unwrap();
            *game_lock = TakGame::new(settings.clone());
            drop(game_lock);
            self.on_game_update();
        }
    }

    pub fn get_active_local_player(&self) -> TakPlayer {
        let current_player = *self.player.read();
        if let Some(info) = self.player_info.read().get(&current_player) {
            if info.player_type == PlayerType::Local {
                return current_player;
            }
        }
        self.player_info
            .read()
            .iter()
            .find(|(_, info)| info.player_type == PlayerType::Local)
            .map(|(p, _)| *p)
            .unwrap_or(current_player)
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: Duration) {
        let mut game_lock = self.game.lock().unwrap();
        game_lock.set_time_remaining(player, time_remaining);
        drop(game_lock);
        self.on_game_update();
    }

    pub fn get_winning_tiles(&self, winner: TakPlayer) -> Vec<TakCoord> {
        let game_lock = self.game.lock().unwrap();
        let mut winning_tiles = Vec::new();
        for y in 0..game_lock.size() {
            for x in 0..game_lock.size() {
                let pos = TakCoord::new(x, y);
                if let Some(tower) = game_lock.try_get_tower(pos) {
                    if tower.controlling_player() == winner {
                        winning_tiles.push(pos);
                    }
                }
            }
        }
        winning_tiles
    }

    pub fn count_flats(&self) -> HashMap<TakPlayer, usize> {
        let game_lock = self.game.lock().unwrap();
        let mut counts = HashMap::new();
        for y in 0..game_lock.size() {
            for x in 0..game_lock.size() {
                if let Some(tower) = game_lock.try_get_tower(TakCoord::new(x, y)) {
                    if tower.top_type == TakPieceType::Flat {
                        *counts.entry(tower.controlling_player()).or_insert(0) += 1;
                    }
                }
            }
        }
        counts
    }

    pub fn set_game_from_ptn(&mut self, ptn: String) -> Option<()> {
        tracing::info!("from ptn str: {:?}", ptn);
        let ptn = Ptn::from_str(&ptn)?;
        tracing::info!("ptn: {:?}", ptn);
        let mut game_lock = self.game.lock().unwrap();
        game_lock.update_from_ptn(ptn)?;
        tracing::info!("game updated");
        drop(game_lock);
        self.on_game_update();
        Some(())
    }

    pub fn reset_game(&mut self) {
        let mut game_lock = self.game.lock().unwrap();
        let settings = game_lock.settings.clone();
        *game_lock = TakGame::new(settings);
        drop(game_lock);
        self.has_started.set(false);
        self.player.set(TakPlayer::White);
        self.move_selection.set(None);
        self.selected_piece_type.set(TakPieceType::Flat);
        self.pieces.set(HashMap::new());
        self.on_game_update();
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

    pub fn get_time_remaining(&self, player: TakPlayer) -> Duration {
        let game_lock = self.game.lock().unwrap();
        game_lock.get_time_remaining(player)
    }

    fn on_finish_move(&mut self, is_local: bool, action: TakActionResult) {
        self.on_game_update();
        if is_local {
            println!("local move: {:?}", action);
            self.message_queue
                .push(ClientGameMessage::Move(action.to_ptn()));
        }
    }

    fn get_remaining_stones(game: &TakGame) -> HashMap<TakPlayer, (usize, usize)> {
        let mut remaining_stones = HashMap::new();
        for player in TakPlayer::all() {
            let hand = game.get_hand(player);
            remaining_stones.insert(player, (hand.stones, hand.capstones));
        }
        remaining_stones
    }

    fn on_game_update(&mut self) {
        let game_lock = self.game.lock().unwrap();
        self.size.set(game_lock.size());
        self.prev_move.set(game_lock.actions.last().cloned());
        let new_player = game_lock.current_player;
        self.player.set(new_player);
        self.game_state.set(game_lock.game_state);
        let mut pieces = self.pieces.write();
        let mut unused_pieces = pieces.keys().cloned().collect::<HashSet<_>>();
        let size = game_lock.size();
        for y in 0..size {
            for x in 0..size {
                Self::on_update_square(&game_lock, &mut unused_pieces, &mut pieces, y, x);
            }
        }
        for id in unused_pieces {
            pieces.remove(&id);
        }
        drop(pieces);
        let remaining_stones = Self::get_remaining_stones(&game_lock);
        self.remaining_stones.set(remaining_stones);

        let active_local_player = self.get_active_local_player();
        let available_piece_types = game_lock.get_valid_place_options(active_local_player);
        let current_selected = self.selected_piece_type.peek().clone();
        match current_selected {
            TakPieceType::Capstone
                if !available_piece_types.contains(&TakPieceType::Capstone)
                    && available_piece_types.contains(&TakPieceType::Flat) =>
            {
                self.selected_piece_type.set(TakPieceType::Flat);
            }
            TakPieceType::Flat
                if !available_piece_types.contains(&TakPieceType::Flat)
                    && available_piece_types.contains(&TakPieceType::Capstone) =>
            {
                self.selected_piece_type.set(TakPieceType::Capstone);
            }
            TakPieceType::Wall
                if !available_piece_types.contains(&TakPieceType::Wall)
                    && available_piece_types.contains(&TakPieceType::Capstone) =>
            {
                self.selected_piece_type.set(TakPieceType::Capstone);
            }
            TakPieceType::Wall
                if !available_piece_types.contains(&TakPieceType::Wall)
                    && available_piece_types.contains(&TakPieceType::Flat) =>
            {
                self.selected_piece_type.set(TakPieceType::Flat);
            }
            _ => {}
        }
        self.available_piece_types.set(available_piece_types);
    }

    fn on_update_square(
        game_lock: &MutexGuard<TakGame>,
        unused_pieces: &mut HashSet<usize>,
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
                    unused_pieces.remove(&stone.id);
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

    pub fn maybe_try_do_remote_action(
        &mut self,
        move_index: usize,
        action: &TakAction,
    ) -> Option<TakFeedback> {
        let mut game_lock = self.game.lock().unwrap();
        if move_index < game_lock.get_current_move_index() {
            return Some(Ok(()));
        } else if move_index > game_lock.get_current_move_index() {
            return None;
        }
        let res = match game_lock.try_do_action(action.clone()) {
            Ok(res) => res,
            Err(e) => return Some(Err(e)),
        };
        drop(game_lock);
        self.on_finish_move(false, res);
        Some(Ok(()))
    }

    pub fn try_do_local_place_move(
        &mut self,
        pos: TakCoord,
        piece_type: TakPieceType,
    ) -> Option<TakFeedback> {
        let tak_move = TakAction::PlacePiece {
            position: pos,
            piece_type,
        };
        let mut game_lock = self.game.lock().unwrap();
        if !self.is_current_player_local(&game_lock) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        let res = game_lock.try_do_action(tak_move.clone());
        drop(game_lock);
        match res {
            Ok(res) => {
                self.on_finish_move(true, res);
                Some(Ok(()))
            }
            Err(e) => Some(Err(e)),
        }
    }

    pub fn try_do_local_move(&mut self, pos: TakCoord) -> Option<TakFeedback> {
        let _ = self.add_to_move_selection(pos);
        self.try_do_local_move_action()
    }

    fn is_current_player_local(&self, game: &TakGame) -> bool {
        let player = game.current_player;
        if let Some(info) = self.player_info.read().get(&player) {
            info.player_type == PlayerType::Local
        } else {
            false
        }
    }

    fn try_do_local_move_action(&mut self) -> Option<TakFeedback> {
        let move_action = self.move_selection.read().clone()?;
        let drop_sum = move_action.drops.iter().sum::<usize>();
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
        if !self.is_current_player_local(&game_lock) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        let res = game_lock.try_do_action(action.clone());
        drop(game_lock);
        self.move_selection.write().take();
        match res {
            Ok(res) => {
                self.on_finish_move(true, res);
                Some(Ok(()))
            }
            Err(e) => Some(Err(e)),
        }
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
        let game_lock = self.game.lock().unwrap();
        if game_lock.get_current_move_index() < 2 {
            self.move_selection.set(None);
            return None;
        }
        drop(game_lock);

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

    pub fn get_bridges(&self) -> HashMap<TakCoord, (TakPlayer, Vec<Direction>)> {
        let _ = self.player.read();
        let game_lock = self.game.lock().unwrap();
        let mut bridges = Vec::new();
        let size = game_lock.size();
        for y in 0..size {
            for x in 0..size {
                let pos = TakCoord::new(x, y);
                let Some(tower) = game_lock.try_get_tower(pos) else {
                    continue;
                };
                let player = tower.controlling_player();
                if tower.top_type == TakPieceType::Wall {
                    continue;
                }
                let mut check_positions = vec![
                    (TakCoord::new(x + 1, y), Direction::Right),
                    (TakCoord::new(x, y + 1), Direction::Up),
                ];
                if x > 0 {
                    check_positions.push((TakCoord::new(x - 1, y), Direction::Left));
                }
                if y > 0 {
                    check_positions.push((TakCoord::new(x, y - 1), Direction::Down));
                }
                for (other_pos, direction) in check_positions {
                    if let Some(other_tower) = game_lock.try_get_tower(other_pos) {
                        if other_tower.controlling_player() == player
                            && other_tower.top_type != TakPieceType::Wall
                        {
                            bridges.push((pos, player, direction));
                        }
                    }
                }
                if x + 1 == size {
                    bridges.push((pos, player, Direction::Right));
                }
                if x == 0 {
                    bridges.push((pos, player, Direction::Left));
                }
                if y + 1 == size {
                    bridges.push((pos, player, Direction::Up));
                }
                if y == 0 {
                    bridges.push((pos, player, Direction::Down));
                }
            }
        }
        let mut bridge_map = HashMap::new();
        for (pos, player, direction) in bridges {
            bridge_map
                .entry(pos)
                .or_insert_with(|| (player, vec![]))
                .1
                .push(direction);
        }
        bridge_map
    }

    pub fn get_selected_tiles(&self) -> Vec<TakCoord> {
        let player = *self.player.read();
        if !*self.has_started.read() || *self.game_state.read() != TakGameState::Ongoing {
            return vec![];
        }
        let Some(PlayerInfo {
            name: _,
            player_type: PlayerType::Local,
        }) = self.player_info.read().get(&player)
        else {
            return vec![];
        };
        self.move_selection
            .read()
            .as_ref()
            .map(|m| {
                let mut positions = vec![];
                if let Some(dir) = m.direction {
                    let offset_pos = m.position.offset_by(&dir, m.drops.len()).unwrap();
                    positions.push(offset_pos);
                    if let Some(pos) = offset_pos.offset_by(&dir, 1) {
                        if self.can_drop_at(m, pos) {
                            positions.push(pos);
                        }
                    }
                } else {
                    for dir in Direction::all() {
                        if let Some(pos) = m.position.offset_by(&dir, 1) {
                            if self.can_drop_at(m, pos) {
                                positions.push(pos);
                            }
                        }
                    }
                }
                positions
            })
            .unwrap_or_else(|| vec![])
    }

    pub fn get_highlighted_tiles(&self) -> Vec<TakCoord> {
        if let TakGameState::Win(winner, _) = *self.game_state.read() {
            self.get_winning_tiles(winner)
        } else if let Some(prev_move) = self.prev_move.read().as_ref() {
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
                        .try_get_positions(direction, drops.len(), *self.size.read())
                        .unwrap_or_else(|| vec![]);
                    positions.push(*from);
                    positions
                }
            }
        } else {
            vec![]
        }
    }
}

#[derive(Clone)]
pub struct TakPieceState {
    pub player: TakPlayer,
    pub piece_type: TakPieceType,
    pub position: TakCoord,
    pub stack_height: usize,
}
