use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use tak_core::{TakAction, TakGame, TakGameState, TakPlayer};

use crate::{
    components::ServerGameMessage,
    server::{
        MatchData, MatchId, MatchInstance, MatchUpdate, PlayerInformation, ServerError,
        ServerResult, UserId, api::MATCHES_TOPIC, internal::cache,
    },
    views::ClientGameMessage,
};

pub struct Matches {
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
    })
}

impl Matches {
    fn new() -> Self {
        Self {
            matches: Arc::new(DashMap::new()),
            players: Arc::new(DashMap::new()),
            match_data: Arc::new(DashMap::new()),
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

    fn add_match(
        &self,
        player_id: UserId,
        opponent_id: UserId,
        settings: MatchInstance,
    ) -> ServerResult<MatchId> {
        let match_id = uuid::Uuid::new_v4().to_string();
        let match_data = new_match_data(settings.clone())?;
        self.matches.insert(match_id.clone(), settings);
        self.players.insert(player_id, match_id.clone());
        self.players.insert(opponent_id, match_id.clone());
        self.match_data.insert(match_id.clone(), match_data);
        Ok(match_id)
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
    let player_id = &instance.player_id;
    let opponent_id = &instance.opponent_id;
    if MATCHES.has_match(player_id) {
        return Err(ServerError::Conflict(
            "Match already exists for this player".to_string(),
        ));
    }
    let player_info = cache::get_or_retrieve_player_info(player_id).await?;
    let opponent_info = cache::get_or_retrieve_player_info(opponent_id).await?;
    let match_id = MATCHES.add_match(player_id.clone(), opponent_id.clone(), instance.clone())?;
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
    Ok(match_id)
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
        Ok(match_id.clone())
    } else {
        Err(ServerError::NotFound)
    }
}

pub async fn get_match(match_id: &MatchId) -> ServerResult<MatchInstance> {
    if let Some(instance) = MATCHES.matches.get(match_id) {
        Ok(instance.clone())
    } else {
        Err(ServerError::NotFound)
    }
}

pub async fn handle_player_publish(player_id: &UserId, topic: String, message: ClientGameMessage) {
    let match_id = topic
        .strip_prefix(&format!("{}/", MATCHES_TOPIC))
        .expect("Invalid topic format")
        .to_string();

    let ClientGameMessage::Move(action_str) = message;

    let payload = MATCHES
        .with_match_data(&match_id, |match_data| {
            let tak_player = match_data
                .player_mapping
                .iter()
                .find(|&(_, id)| id == player_id)
                .map(|(player, _)| player)
                .expect("Player not found in match data");

            if match_data.game.game_state != TakGameState::Ongoing {
                println!("Game is not ongoing");
                return None;
            }
            if match_data.game.check_timeout() {
                println!("Game has timed out");
                return None;
            }
            if match_data.game.current_player != tak_player {
                println!("Not your turn");
                return None;
            }
            let Some(action) = TakAction::from_ptn(&action_str) else {
                println!("Invalid action: {action_str}");
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
    }
}
