use crate::server::room::{get_players, GetPlayersResponse};
use crate::views::ClientGameMessage;
use dioxus::logger::tracing;
use dioxus::prelude::{Readable, Signal, Writable, WritableVecExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tak_core::{
    TakAction, TakActionRecord, TakCoord, TakGame, TakGameSettings, TakGameState, TakPieceVariant,
    TakPlayer, TakPtn, TakUIState,
};

#[derive(Clone)]
pub struct TakBoardState {
    game: Option<Arc<Mutex<TakUIState>>>,
    pub has_started: Signal<bool>,
    pub on_change: Signal<bool>,

    pub selected_piece_type: Signal<TakPieceVariant>,
    pub player_info: Signal<HashMap<TakPlayer, PlayerInfo>>,
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

impl TakBoardState {
    pub fn new(player_info: HashMap<TakPlayer, PlayerInfo>) -> Self {
        TakBoardState {
            game: None,
            has_started: Signal::new(false),
            on_change: Signal::new(false),
            player_info: Signal::new(player_info),
            selected_piece_type: Signal::new(TakPieceVariant::Flat),
            message_queue: Signal::new(Vec::new()),
        }
    }

    pub fn try_set_from_settings(
        &mut self,
        settings: TakGameSettings,
        cause_update: bool,
    ) -> Option<()> {
        let mut game = TakUIState::new(TakGame::new(settings)?);
        let mut on_change = self.on_change.clone();
        game.add_listener(move || {
            on_change.toggle();
        });
        self.game = Some(Arc::new(Mutex::new(game)));
        if cause_update {
            on_change.toggle();
        }
        Some(())
    }

    pub fn try_set_from_ptn(&mut self, ptn: String) -> Option<()> {
        tracing::info!("from ptn str: {:?}", ptn);
        let ptn = TakPtn::try_from_str(&ptn)?;
        let mut game = TakUIState::new(TakGame::try_from_ptn(ptn)?);
        let mut on_change = self.on_change.clone();
        game.add_listener(move || {
            on_change.toggle();
        });
        self.game = Some(Arc::new(Mutex::new(game)));
        on_change.toggle();
        Some(())
    }

    pub fn with_game<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&TakUIState) -> R,
    {
        if let Some(game) = &self.game {
            let game_lock = game.lock().unwrap();
            f(&game_lock)
        } else {
            panic!("Game not initialized");
        }
    }

    pub fn with_game_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut TakUIState) -> R,
    {
        if let Some(game) = &self.game {
            let mut game_lock = game.lock().unwrap();
            f(&mut game_lock)
        } else {
            panic!("Game not initialized");
        }
    }

    pub fn get_active_local_player(&self) -> TakPlayer {
        let current_player = self.with_game(|game| game.game().current_player);
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

    pub fn reset(&mut self) {
        self.has_started.set(false);
        if let Some(game) = &self.game {
            let mut game_lock = game.lock().unwrap();
            game_lock.game_mut().reset();
            self.has_started.set(false);
            self.selected_piece_type.set(TakPieceVariant::Flat);
            self.on_change.toggle();
        }
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: u64) {
        self.with_game_mut(|game| {
            game.game_mut().set_time_remaining(player, time_remaining);
        });
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

    pub fn has_game(&self) -> bool {
        self.game.is_some()
    }

    pub fn has_ongoing_game(&self) -> bool {
        self.game.is_some()
            && *self.has_started.read()
            && self.with_game(|game| game.game().game_state == TakGameState::Ongoing)
    }

    pub fn is_local_player_turn(&self) -> bool {
        let current_player = self.with_game(|game| game.game().current_player);
        if let Some(info) = self.player_info.read().get(&current_player) {
            return info.player_type == PlayerType::Local;
        }
        false
    }

    pub fn is_place_action(&self, pos: TakCoord) -> bool {
        self.with_game(|game| {
            game.partial_move.is_none() && game.game().board.try_get_tower(pos).is_none()
        })
    }

    pub fn get_time_remaining(&self, player: TakPlayer) -> u64 {
        self.with_game(|game| {
            let apply_elapsed = game.game().current_player == player
                && game.game().game_state == TakGameState::Ongoing;
            game.game()
                .get_time_remaining(player, apply_elapsed)
                .unwrap_or_default()
        })
    }

    fn send_move_message(&mut self, action: TakActionRecord) {
        println!("local move: {:?}", action);
        self.message_queue
            .push(ClientGameMessage::Move(action.to_ptn()));
    }

    pub fn correct_selected_piece_type(&mut self) {
        let current_selected = self.selected_piece_type.peek().clone();
        let active_player = self.get_active_local_player();
        let mut selected_piece_type = self.selected_piece_type.clone();
        self.with_game(move |game| {
            let available_piece_types = &game.available_piece_types[active_player.index()];
            match current_selected {
                TakPieceVariant::Capstone
                    if !available_piece_types.contains(&TakPieceVariant::Capstone)
                        && available_piece_types.contains(&TakPieceVariant::Flat) =>
                {
                    selected_piece_type.set(TakPieceVariant::Flat);
                }
                TakPieceVariant::Flat
                    if !available_piece_types.contains(&TakPieceVariant::Flat)
                        && available_piece_types.contains(&TakPieceVariant::Capstone) =>
                {
                    selected_piece_type.set(TakPieceVariant::Capstone);
                }
                TakPieceVariant::Wall
                    if !available_piece_types.contains(&TakPieceVariant::Wall)
                        && available_piece_types.contains(&TakPieceVariant::Capstone) =>
                {
                    selected_piece_type.set(TakPieceVariant::Capstone);
                }
                TakPieceVariant::Wall
                    if !available_piece_types.contains(&TakPieceVariant::Wall)
                        && available_piece_types.contains(&TakPieceVariant::Flat) =>
                {
                    selected_piece_type.set(TakPieceVariant::Flat);
                }
                _ => {}
            }
        });
    }

    pub fn try_parse_action(&self, action: &str) -> Option<TakAction> {
        let game = self.game.as_ref()?;
        let game_lock = game.lock().unwrap();
        TakAction::from_ptn(game_lock.game().board.size as i32, action)
    }

    pub fn maybe_try_do_remote_action(
        &mut self,
        move_index: usize,
        action: TakAction,
    ) -> Result<(), ()> {
        self.with_game_mut(|game| {
            let index = game.game().turn_index;
            if index < move_index {
                return Ok(());
            } else if index > move_index {
                return Err(());
            }
            if let Err(e) = game.try_do_action(action) {
                tracing::error!("Error processing remote action: {:?}", e);
                Err(())
            } else {
                Ok(())
            }
        })
    }

    pub fn try_do_local_place(&mut self, pos: TakCoord, variant: TakPieceVariant) -> Option<()> {
        let tak_move = TakAction::PlacePiece { pos, variant };
        let game = self.game.clone()?;
        let mut game = game.lock().unwrap();

        if !self.is_current_player_local(game.game()) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        let res = game.try_do_action(tak_move);
        match res {
            Ok(_) => {
                self.send_move_message(
                    game.game()
                        .get_last_action()
                        .expect("Last action should exist")
                        .clone(),
                );
                Some(())
            }
            Err(e) => {
                tracing::error!("Error processing place action: {:?}", e);
                None
            }
        }
    }

    pub fn try_do_local_move(&mut self, pos: TakCoord) -> Option<()> {
        let game = self.game.clone()?;
        let mut game = game.lock().unwrap();

        if !self.is_current_player_local(game.game()) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        if let Some(()) = game.add_square_to_partial_move(pos) {
            self.send_move_message(
                game.game()
                    .get_last_action()
                    .expect("Last action should exist")
                    .clone(),
            );
            return Some(());
        }
        None
    }

    fn is_current_player_local(&self, game: &TakGame) -> bool {
        let player = game.current_player;
        if let Some(info) = self.player_info.read().get(&player) {
            info.player_type == PlayerType::Local
        } else {
            false
        }
    }
    /*
    pub fn get_bridges(&self) -> HashMap<TakCoord, (TakPlayer, Vec<TakDir>)> {
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
                if tower.top_type == TakPieceVariant::Wall {
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
                            && other_tower.top_type != TakPieceVariant::Wall
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
    }*/
}
