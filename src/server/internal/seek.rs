use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use tak_core::TakPlayer;

use crate::server::{
    MatchId, MatchInstance, PlayerInformation, RematchColor, SeekSettings, SeekUpdate, ServerError,
    ServerResult, UserId,
    api::SEEK_TOPIC,
    internal::{cache, matches},
};

pub struct Seeks {
    seeks: Arc<DashMap<UserId, SeekSettings>>,
}

impl Seeks {
    fn new() -> Self {
        Self {
            seeks: Arc::new(DashMap::new()),
        }
    }

    fn add_seek(&self, player_id: UserId, seek: SeekSettings) {
        self.seeks.insert(player_id, seek);
    }

    fn has_seek(&self, player_id: &UserId) -> bool {
        self.seeks.contains_key(player_id)
    }

    fn get_seek(&self, player_id: &UserId) -> Option<SeekSettings> {
        self.seeks.get(player_id).map(|x| x.clone())
    }

    fn remove_seek(&self, player_id: &UserId) {
        self.seeks.remove(player_id);
    }

    fn get_seeks(&self) -> Vec<(UserId, SeekSettings)> {
        self.seeks
            .iter()
            .map(|x| (x.key().clone(), x.value().clone()))
            .collect()
    }
}

pub static SEEKS: LazyLock<Seeks> = LazyLock::new(|| Seeks::new());

pub async fn create_seek(player_id: &UserId, settings: SeekSettings) -> ServerResult<()> {
    if SEEKS.has_seek(player_id) {
        return Err(ServerError::Conflict(
            "Seek already exists for this player".to_string(),
        ));
    }
    if !settings.game_settings.validate() {
        return Err(ServerError::BadRequest(
            "Invalid game settings for seek".to_string(),
        ));
    }
    let player_info = cache::get_or_retrieve_player_info(player_id).await?;
    SEEKS.add_seek(player_id.clone(), settings.clone());
    ws_pubsub::publish_to_topic(
        SEEK_TOPIC,
        SeekUpdate::Created {
            player_info,
            settings,
        },
    )
    .await;
    log::info!("Seek created for player: {}", player_id);
    Ok(())
}

pub async fn cancel_seek(player_id: &UserId) -> ServerResult<()> {
    if !SEEKS.has_seek(player_id) {
        return Err(ServerError::NotFound);
    }
    SEEKS.remove_seek(player_id);
    ws_pubsub::publish_to_topic(
        SEEK_TOPIC,
        SeekUpdate::Removed {
            player_id: player_id.clone(),
        },
    )
    .await;
    log::info!("Seek cancelled for player: {}", player_id);
    Ok(())
}

pub async fn get_seeks() -> ServerResult<Vec<(PlayerInformation, SeekSettings)>> {
    let mut seek_list = Vec::new();
    for (opponent_id, seek) in SEEKS.get_seeks() {
        seek_list.push((
            cache::get_or_retrieve_player_info(&opponent_id).await?,
            seek,
        ));
    }
    Ok(seek_list)
}

pub async fn accept_seek(player_id: &UserId, opponent_id: &UserId) -> ServerResult<MatchId> {
    let seek = SEEKS.get_seek(opponent_id).ok_or(ServerError::NotFound)?;

    if opponent_id == player_id {
        return Err(ServerError::Conflict(
            "Cannot accept your own seek".to_string(),
        ));
    }

    SEEKS.remove_seek(opponent_id);
    ws_pubsub::publish_to_topic(
        SEEK_TOPIC,
        SeekUpdate::Removed {
            player_id: player_id.clone(),
        },
    )
    .await;

    let first_player_is_white = match seek.creator_color {
        Some(TakPlayer::White) => true,
        Some(TakPlayer::Black) => false,
        None => rand::random(),
    };

    let creator_color = if first_player_is_white {
        TakPlayer::White
    } else {
        TakPlayer::Black
    };

    log::info!(
        "Accepting seek for player: {}, opponent: {}",
        player_id,
        opponent_id
    );

    let match_id = matches::create_match(MatchInstance {
        player_id: player_id.clone(),
        opponent_id: opponent_id.clone(),
        game_settings: seek.game_settings,
        rated: seek.rated,
        creator_color,
        rematch_color: RematchColor::Alternate,
    })
    .await?;

    Ok(match_id)
}

pub async fn get_seek(player_id: &UserId) -> ServerResult<SeekSettings> {
    if let Some(seek) = SEEKS.get_seek(player_id) {
        Ok(seek)
    } else {
        Err(ServerError::NotFound)
    }
}
