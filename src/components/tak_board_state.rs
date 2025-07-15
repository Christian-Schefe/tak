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
    game: Arc<Mutex<Option<TakUIState>>>,
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
            game: Arc::new(Mutex::new(None)),
            has_started: Signal::new(false),
            on_change: Signal::new(false),
            player_info: Signal::new(player_info),
            selected_piece_type: Signal::new(TakPieceVariant::Flat),
            message_queue: Signal::new(Vec::new()),
        }
    }

    pub fn trigger_change(&mut self) {
        let new_val = !*self.on_change.peek();
        self.on_change.set(new_val);
    }

    pub fn try_set_from_settings(&mut self, settings: TakGameSettings) -> Option<()> {
        let mut new_game = TakUIState::new(TakGame::new(settings)?);
        let mut on_change = self.on_change.clone();
        new_game.add_listener(move || {
            let new_val = !*on_change.peek();
            on_change.set(new_val);
        });
        let mut game_lock = self.game.lock().unwrap();
        *game_lock = Some(new_game);
        drop(game_lock);
        tracing::info!("Game initialized with settings, {}", self.has_game());
        self.trigger_change();
        Some(())
    }

    pub fn try_set_from_ptn(&mut self, ptn: String) -> Option<()> {
        tracing::info!("from ptn str: {:?}", ptn);
        let ptn = TakPtn::try_from_str(&ptn)?;
        let mut new_game = TakUIState::new(TakGame::try_from_ptn(ptn)?);
        let mut on_change = self.on_change.clone();
        new_game.add_listener(move || {
            let new_val = !*on_change.peek();
            on_change.set(new_val);
        });
        let mut game_lock = self.game.lock().unwrap();
        *game_lock = Some(new_game);
        drop(game_lock);
        tracing::info!("Game initialized with ptn, {}", self.has_game());
        self.trigger_change();
        Some(())
    }

    pub fn with_game<F, R>(&self, f: F) -> Result<R, ()>
    where
        F: FnOnce(&TakUIState) -> R,
    {
        let game_lock = self.game.lock().unwrap();
        let game = game_lock.as_ref().ok_or(())?;
        Ok(f(game))
    }

    pub fn with_game_mut<F, R>(&mut self, f: F) -> Result<R, ()>
    where
        F: FnOnce(&mut TakUIState) -> R,
    {
        let mut game_lock = self.game.lock().unwrap();
        let game = game_lock.as_mut().ok_or(())?;
        Ok(f(game))
    }

    pub fn get_active_local_player(&self) -> TakPlayer {
        let current_player = self
            .with_game(|game| game.game().current_player)
            .expect("Game should exist to get current player");
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
        self.selected_piece_type.set(TakPieceVariant::Flat);
        let mut game_lock = self.game.lock().unwrap();
        game_lock.as_mut().map(|x| x.reset());
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: u64) {
        self.with_game_mut(|game| {
            game.set_time_remaining(player, time_remaining);
        })
        .expect("Game should exist to set time remaining");
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
        let lock = self.game.lock().unwrap();
        lock.is_some()
    }

    pub fn check_ongoing_game(&mut self) -> bool {
        self.has_game()
            && *self.has_started.read()
            && self
                .with_game_mut(|game| {
                    game.check_timeout();
                    game.game().game_state == TakGameState::Ongoing
                })
                .expect("Game should exist to check ongoing state")
    }

    pub fn is_local_player_turn(&self) -> bool {
        let current_player = self
            .with_game(|game| game.game().current_player)
            .expect("Game should exist to check current player");
        if let Some(info) = self.player_info.read().get(&current_player) {
            return info.player_type == PlayerType::Local;
        }
        false
    }

    pub fn is_place_action(&self, pos: TakCoord) -> bool {
        self.with_game(|game| {
            game.partial_move.is_none() && game.game().board.try_get_stack(pos).is_none()
        })
        .expect("Game should exist to check place action")
    }

    pub fn get_time_remaining(&self, player: TakPlayer) -> u64 {
        self.with_game(|game| {
            let apply_elapsed = game.game().current_player == player
                && game.game().game_state == TakGameState::Ongoing;
            game.game()
                .get_time_remaining(player, apply_elapsed)
                .unwrap_or_default()
        })
        .expect("Game should exist to get time remaining")
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
        })
        .expect("Game should exist to correct selected piece type");
    }

    pub fn maybe_try_do_remote_action(
        &mut self,
        move_index: usize,
        action: TakAction,
    ) -> Result<(), ()> {
        self.with_game_mut(|game| {
            let index = game.game().ply_index;
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
        .expect("Game should exist to try do remote action")
    }

    pub fn try_do_local_place(&mut self, pos: TakCoord, variant: TakPieceVariant) -> Option<()> {
        let tak_move = TakAction::PlacePiece { pos, variant };
        let game = self.game.clone();
        let mut lock = game.lock().unwrap();
        let game = lock.as_mut()?;

        if !self.is_current_player_local(game.game()) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        let res = game.try_do_action(tak_move);
        match res {
            Ok(_) => {
                let last_action = game
                    .game()
                    .get_last_action()
                    .expect("Last action should exist")
                    .clone();
                drop(lock);
                self.send_move_message(last_action);
                Some(())
            }
            Err(e) => {
                tracing::error!("Error processing place action: {:?}", e);
                None
            }
        }
    }

    pub fn try_do_local_move(&mut self, pos: TakCoord) -> Option<()> {
        let game = self.game.clone();
        let mut lock = game.lock().unwrap();
        let game = lock.as_mut()?;

        if !self.is_current_player_local(game.game()) {
            tracing::error!("Current player is not local, cannot perform action");
            return None;
        }
        if let Some(res) = game.add_square_to_partial_move(pos) {
            let Ok(()) = res else {
                tracing::error!("Error processing move action: {:?}", res);
                return None;
            };
            let last_action = game
                .game()
                .get_last_action()
                .expect("Last action should exist")
                .clone();
            drop(lock);
            self.send_move_message(last_action);
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
}
