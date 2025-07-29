use std::{collections::HashMap, sync::LazyLock};

use crate::server::{
    PlayerInformation, SeekSettings, ServerError, ServerResult, UserId, internal::cache,
};

pub struct Seeks {
    seeks: HashMap<UserId, SeekSettings>,
}

impl Seeks {
    pub fn new() -> Self {
        Seeks {
            seeks: HashMap::new(),
        }
    }

    pub fn add_seek(&mut self, player_id: UserId, seek: SeekSettings) {
        self.seeks.insert(player_id, seek);
    }

    pub fn get_seek(&self, player_id: &UserId) -> Option<&SeekSettings> {
        self.seeks.get(player_id)
    }

    pub fn remove_seek(&mut self, player_id: &UserId) {
        self.seeks.remove(player_id);
    }
}

pub static SEEKS: LazyLock<tokio::sync::RwLock<Seeks>> =
    LazyLock::new(|| tokio::sync::RwLock::new(Seeks::new()));

pub async fn create_seek(player_id: &UserId, settings: SeekSettings) -> ServerResult<()> {
    let mut seeks = SEEKS.write().await;
    if seeks.get_seek(player_id).is_some() {
        return Err(ServerError::Conflict(
            "Seek already exists for this player".to_string(),
        ));
    }
    seeks.add_seek(player_id.clone(), settings);
    Ok(())
}

pub async fn cancel_seek(player_id: &UserId) -> ServerResult<()> {
    let mut seeks = SEEKS.write().await;
    if seeks.get_seek(player_id).is_none() {
        return Err(ServerError::NotFound);
    }
    seeks.remove_seek(player_id);
    Ok(())
}

pub async fn get_seeks(
    player_id: &UserId,
) -> ServerResult<Vec<(PlayerInformation, SeekSettings, bool)>> {
    let seeks = SEEKS.read().await;
    let mut seek_list = Vec::new();
    for (opponent_id, seek) in seeks.seeks.iter() {
        seek_list.push((
            cache::get_or_retrieve_player_info(opponent_id).await?,
            seek.clone(),
            opponent_id != player_id,
        ));
    }
    Ok(seek_list)
}

pub async fn accept_seek(player_id: &UserId, opponent_id: &UserId) -> ServerResult<()> {
    let mut seeks = SEEKS.write().await;
    let _seek = seeks.get_seek(opponent_id).ok_or(ServerError::NotFound)?;

    if opponent_id == player_id {
        return Err(ServerError::Conflict(
            "Cannot accept your own seek".to_string(),
        ));
    }

    seeks.remove_seek(opponent_id);
    Ok(())
}
