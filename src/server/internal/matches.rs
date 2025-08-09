use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use tak_core::{TakAction, TakGame, TakGameState, TakPlayer};

use crate::{
    components::ServerGameMessage,
    server::{
        MatchData, MatchId, MatchInstance, MatchUpdate, PlayerInformation, ServerError,
        ServerResult, UserId,
        api::{DRAW_SUBTOPIC, MATCHES_TOPIC, REMATCH_SUBTOPIC},
        internal::cache,
    },
    views::ClientGameMessage,
};

pub struct Matches {
    match_end_senders: Arc<DashMap<MatchId, tokio::sync::oneshot::Sender<TakGameState>>>,
    match_data: Arc<DashMap<MatchId, MatchData>>,
    matches: Arc<DashMap<MatchId, MatchInstance>>,
    players: Arc<DashMap<UserId, MatchId>>,
}

fn new_match_data(instance: MatchInstance) -> ServerResult<MatchData> {
    let game = TakGame::new(instance.game_settings).ok_or(ServerError::InternalServerError(
        "Failed to create game: invalid settings".to_string(),
    ))?;
    let mut player_mapping = fixed_map::Map::new();
    player_mapping.insert(instance.creator_color, instance.player_id);
    player_mapping.insert(instance.creator_color.other(), instance.opponent_id);
    Ok(MatchData {
        game,
        player_mapping,
        rematch_agree: Vec::new(),
        draw_agree: Vec::new(),
        has_ended: false,
    })
}

impl Matches {
    fn new() -> Self {
        Self {
            matches: Arc::new(DashMap::new()),
            players: Arc::new(DashMap::new()),
            match_data: Arc::new(DashMap::new()),
            match_end_senders: Arc::new(DashMap::new()),
        }
    }

    fn has_match(&self, player_id: &UserId) -> bool {
        self.players.contains_key(player_id)
    }

    fn get_matches(&self) -> Vec<(MatchId, MatchInstance)> {
        self.matches
            .iter()
            .map(|x| (x.key().clone(), x.value().clone()))
            .collect()
    }

    fn with_match_data<F, R>(&self, match_id: &MatchId, f: F) -> Option<R>
    where
        F: FnOnce(&mut MatchData) -> R,
    {
        if let Some(mut match_data) = self.match_data.get_mut(match_id) {
            Some(f(&mut match_data))
        } else {
            None
        }
    }

    fn check_game_over(&self, match_id: &MatchId) -> Option<bool> {
        self.with_match_data(match_id, |match_data| {
            match_data.game.check_timeout();
            if match_data.game.game_state == TakGameState::Ongoing {
                return false;
            }
            let Some((_, sender)) = self.match_end_senders.remove(match_id) else {
                return false;
            };
            if let Err(_) = sender.send(match_data.game.game_state.clone()) {
                eprintln!("Failed to send game end notification for match: {match_id}");
            };
            true
        })
    }

    fn with_ongoing_game<T>(
        &self,
        match_id: &MatchId,
        f: impl FnOnce(&mut MatchData) -> T,
    ) -> ServerResult<Option<T>> {
        self.with_match_data(match_id, |match_data| {
            match_data.game.check_timeout();
            if match_data.game.game_state != TakGameState::Ongoing {
                let Some((_, sender)) = self.match_end_senders.remove(match_id) else {
                    return None;
                };
                if let Err(_) = sender.send(match_data.game.game_state.clone()) {
                    eprintln!("Failed to send game end notification for match: {match_id}");
                };
                return None;
            }
            Some(f(match_data))
        })
        .ok_or(ServerError::NotFound)
    }

    async fn add_match(&self, match_id: MatchId, settings: MatchInstance) -> ServerResult<()> {
        let match_data = new_match_data(settings.clone())?;
        self.players
            .insert(settings.player_id.clone(), match_id.clone());
        self.players
            .insert(settings.opponent_id.clone(), match_id.clone());
        self.matches.insert(match_id.clone(), settings);
        self.match_data.insert(match_id.clone(), match_data);
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.match_end_senders.insert(match_id.clone(), tx);
        tokio::spawn(check_match_finished_task(rx, match_id.clone()));
        tokio::spawn(check_match_timeout_task(match_id.clone()));
        Ok(())
    }

    fn remove_match(&self, match_id: &MatchId) -> ServerResult<MatchInstance> {
        if let Some((_, match_instance)) = self.matches.remove(match_id) {
            self.players.remove(&match_instance.player_id);
            self.players.remove(&match_instance.opponent_id);
            self.match_data.remove(match_id);
            self.match_end_senders.remove(match_id);
            Ok(match_instance)
        } else {
            Err(ServerError::NotFound)
        }
    }
}

pub fn get_match_data(match_id: &MatchId) -> ServerResult<MatchData> {
    if let Some(data) = MATCHES.match_data.get(match_id) {
        Ok(data.clone())
    } else {
        Err(ServerError::NotFound)
    }
}

pub static MATCHES: LazyLock<Matches> = LazyLock::new(|| Matches::new());

pub async fn create_match(instance: MatchInstance) -> ServerResult<MatchId> {
    if MATCHES.has_match(&instance.player_id) || MATCHES.has_match(&instance.opponent_id) {
        return Err(ServerError::Conflict(
            "Match already exists for this player".to_string(),
        ));
    }
    let player_id = instance.player_id.clone();
    let opponent_id = instance.opponent_id.clone();
    let player_info = cache::get_or_retrieve_player_info(&instance.player_id).await?;
    let opponent_info = cache::get_or_retrieve_player_info(&instance.opponent_id).await?;
    let match_id = uuid::Uuid::new_v4().to_string();
    MATCHES
        .add_match(match_id.clone(), instance.clone())
        .await?;
    ws_pubsub::publish_to_topic(
        MATCHES_TOPIC,
        MatchUpdate::Created {
            player_info,
            opponent_info,
            match_id: match_id.clone(),
            settings: instance,
        },
    )
    .await;
    ws_pubsub::publish_to_topic(
        format!("{}/{}", MATCHES_TOPIC, match_id),
        ServerGameMessage::StartGame,
    )
    .await;

    log::info!(
        "Match created: {}, player: {}, opponent: {}",
        match_id,
        player_id,
        opponent_id
    );
    Ok(match_id)
}

pub async fn restart_match(match_id: &MatchId) -> ServerResult<()> {
    let instance = MATCHES.remove_match(match_id)?;

    let player_id = instance.player_id.clone();
    let opponent_id = instance.opponent_id.clone();

    MATCHES.add_match(match_id.clone(), instance).await?;

    ws_pubsub::publish_to_topic(
        format!("{}/{}", MATCHES_TOPIC, match_id),
        ServerGameMessage::StartGame,
    )
    .await;

    log::info!(
        "Match created: {}, player: {}, opponent: {}",
        match_id,
        player_id,
        opponent_id
    );
    Ok(())
}

pub async fn get_matches()
-> ServerResult<Vec<(MatchId, PlayerInformation, PlayerInformation, MatchInstance)>> {
    let mut seek_list = Vec::new();
    for (match_id, instance) in MATCHES.get_matches() {
        seek_list.push((
            match_id.clone(),
            cache::get_or_retrieve_player_info(&instance.player_id).await?,
            cache::get_or_retrieve_player_info(&instance.opponent_id).await?,
            instance,
        ));
    }
    Ok(seek_list)
}

pub async fn get_match_id(player_id: &UserId) -> ServerResult<MatchId> {
    if let Some(match_id) = MATCHES.players.get(player_id) {
        Ok(match_id.value().clone())
    } else {
        Err(ServerError::NotFound)
    }
}

pub async fn get_match(match_id: &MatchId) -> ServerResult<MatchInstance> {
    if let Some(instance) = MATCHES.matches.get(match_id) {
        Ok(instance.value().clone())
    } else {
        Err(ServerError::NotFound)
    }
}

pub async fn offer_draw(player_id: &UserId) -> ServerResult<()> {
    let match_id = MATCHES
        .players
        .get(player_id)
        .map(|x| x.value().clone())
        .ok_or_else(|| ServerError::NotFound)?;

    let did_draw = MATCHES
        .with_ongoing_game(&match_id, |match_data| {
            if match_data.draw_agree.contains(player_id) {
                return Err(ServerError::Conflict("Already offered draw".to_string()));
            }
            match_data.draw_agree.push(player_id.clone());
            if match_data.draw_agree.len() == 2 {
                match_data.game.abort(None);
                Ok(true)
            } else {
                Ok(false)
            }
        })?
        .unwrap_or(Err(ServerError::NotAllowed("Game has ended".to_string())))?;

    ws_pubsub::publish_to_topic(
        format!("{}/{}/{}", MATCHES_TOPIC, match_id, DRAW_SUBTOPIC),
        (),
    )
    .await;

    if did_draw {
        MATCHES.check_game_over(&match_id);
    }

    log::info!("Player {} offered draw for match: {}", player_id, match_id);
    Ok(())
}

pub async fn agree_rematch(player_id: &UserId) -> ServerResult<()> {
    let match_id = MATCHES
        .players
        .get(player_id)
        .map(|x| x.value().clone())
        .ok_or_else(|| ServerError::NotFound)?;

    let should_rematch = MATCHES
        .with_match_data(&match_id, |match_data| {
            if !match_data.has_ended {
                return Err(ServerError::Conflict(
                    "Cannot agree to rematch while game is ongoing".to_string(),
                ));
            }
            if match_data.rematch_agree.contains(player_id) {
                return Err(ServerError::Conflict(
                    "Already agreed to rematch".to_string(),
                ));
            }
            match_data.rematch_agree.push(player_id.clone());
            Ok(match_data.rematch_agree.len() == 2)
        })
        .unwrap_or(Err(ServerError::NotFound))?;

    ws_pubsub::publish_to_topic(
        format!("{}/{}/{}", MATCHES_TOPIC, match_id, REMATCH_SUBTOPIC),
        (),
    )
    .await;

    log::info!(
        "Player {} agreed to rematch for match: {}",
        player_id,
        match_id
    );

    if should_rematch {
        restart_match(&match_id).await?;
        log::info!("Match rematch started for match: {}", match_id);
    }
    Ok(())
}

pub async fn retract_rematch(player_id: &UserId) -> ServerResult<()> {
    let match_id = MATCHES
        .players
        .get(player_id)
        .map(|x| x.value().clone())
        .ok_or_else(|| ServerError::NotFound)?;

    MATCHES
        .with_match_data(&match_id, |match_data| {
            if !match_data.has_ended {
                return Err(ServerError::Conflict(
                    "Cannot retract rematch while game hasn't ended".to_string(),
                ));
            }
            if !match_data.rematch_agree.contains(player_id) {
                return Err(ServerError::Conflict(
                    "Hasn't agreed to rematch".to_string(),
                ));
            }
            match_data.rematch_agree.retain(|x| x != player_id);
            Ok(())
        })
        .unwrap_or(Err(ServerError::NotFound))?;

    ws_pubsub::publish_to_topic(
        format!("{}/{}/{}", MATCHES_TOPIC, match_id, REMATCH_SUBTOPIC),
        (),
    )
    .await;

    Ok(())
}

pub async fn leave_match(player_id: &UserId) -> ServerResult<()> {
    let match_id = MATCHES
        .players
        .get(player_id)
        .map(|x| x.value().clone())
        .ok_or_else(|| ServerError::NotFound)?;

    MATCHES
        .with_match_data(&match_id, |match_data| {
            if !match_data.has_ended {
                return Err(ServerError::Conflict(
                    "Cannot leave match while game hasn't ended".to_string(),
                ));
            }
            match_data.rematch_agree.clear();
            Ok(())
        })
        .unwrap_or(Err(ServerError::NotFound))?;

    ws_pubsub::publish_to_topic(
        format!("{}/{}/{}", MATCHES_TOPIC, match_id, REMATCH_SUBTOPIC),
        (),
    )
    .await;

    log::info!("Player {} left match: {}", player_id, match_id);

    MATCHES.remove_match(&match_id)?;
    log::info!("Match removed: {match_id}");
    Ok(())
}

pub async fn handle_player_publish(player_id: &UserId, topic: String, message: ClientGameMessage) {
    let match_id = topic
        .strip_prefix(&format!("{}/", MATCHES_TOPIC))
        .expect("Invalid topic format")
        .to_string();

    let ClientGameMessage::Move(action_str) = message;

    log::info!("Received action for match: {match_id}, player: {player_id}, action: {action_str}");
    MATCHES.check_game_over(&match_id);

    let payload = MATCHES
        .with_match_data(&match_id, |match_data| {
            let tak_player = match_data
                .player_mapping
                .iter()
                .find(|&(_, id)| id == player_id)
                .map(|(player, _)| player)
                .expect("Player not found in match data");

            if match_data.game.game_state != TakGameState::Ongoing {
                log::warn!("Game is not ongoing");
                return None;
            }
            if match_data.game.check_timeout() {
                log::warn!("Game has timed out");
                return None;
            }
            if match_data.game.current_player != tak_player {
                log::warn!("Not your turn");
                return None;
            }
            let Some(action) = TakAction::from_ptn(&action_str) else {
                log::warn!("Invalid action: {action_str}");
                return None;
            };

            let move_index = match_data.game.ply_index;
            let res = match match_data.game.try_do_action(action) {
                Ok(()) => match_data
                    .game
                    .get_last_action()
                    .expect("Action history should not be empty"),
                Err(e) => {
                    println!(
                        "Error processing action: {e:?}, {}",
                        match_data.game.to_tps().to_string()
                    );
                    return None;
                }
            }
            .clone();

            let time_remaining = TakPlayer::ALL
                .into_iter()
                .map(|x| (x, match_data.game.get_time_remaining(x, true).unwrap()))
                .collect::<Vec<_>>();

            Some(ServerGameMessage::Move(
                move_index,
                time_remaining,
                res.to_ptn(),
            ))
        })
        .flatten();

    if let Some(msg) = payload {
        ws_pubsub::publish_to_topic(format!("{}/{}", MATCHES_TOPIC, match_id), msg).await;
    } else {
        log::warn!("Failed to process action for match: {match_id}");
    }

    MATCHES.check_game_over(&match_id);
}

async fn check_match_timeout_task(match_id: MatchId) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        match MATCHES.check_game_over(&match_id) {
            Some(true) => break,
            Some(false) => {}
            None => {
                log::error!("Match data not found for match: {match_id}");
                break;
            }
        }
    }
    log::info!("Match timeout check completed for match: {match_id}");
}

async fn check_match_finished_task(
    game_end_receiver: tokio::sync::oneshot::Receiver<TakGameState>,
    match_id: MatchId,
) {
    let game_state = match game_end_receiver.await {
        Ok(state) => state,
        Err(_) => {
            log::error!("Game end receiver was dropped, stopping gameover check");
            return;
        }
    };

    log::info!("Match {match_id} finished with state: {game_state:?}");

    let match_data = MATCHES
        .with_match_data(&match_id, |match_data| {
            if match_data.game.game_state != game_state {
                log::warn!(
                    "Game state mismatch: expected {:?}, got {:?}",
                    match_data.game.game_state,
                    game_state
                );
                return None;
            }
            if match_data.has_ended {
                log::warn!("Match {match_id} has already ended");
                return None;
            }
            match_data.has_ended = true;
            Some(match_data.clone())
        })
        .flatten();

    let Some(match_data) = match_data else {
        log::warn!("Match data not found for match: {match_id}");
        return;
    };

    let msg = ServerGameMessage::GameOver(game_state.clone());

    ws_pubsub::publish_to_topic(format!("{}/{}", MATCHES_TOPIC, match_id), msg).await;

    let game = match_data.game;
    let player_mapping = match_data.player_mapping;

    if game.game_state == TakGameState::Canceled {
        log::info!("Game was canceled, not saving game record");
        return;
    }
    if let Err(e) = super::player::add_game(game, player_mapping).await {
        log::error!("Failed to add game: {:?}", e);
    } else {
        log::info!("Game added successfully for match: {match_id}");
    }
}
