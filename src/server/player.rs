use serde::{Deserialize, Serialize};

use crate::server::db::DB;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub user_id: String,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
    pub rating: f64,
}

#[derive(Debug, Clone)]
pub enum GameResult {
    Win,
    Draw,
    Loss,
}

fn create_player(user_id: String) -> Player {
    Player {
        user_id,
        wins: 0,
        losses: 0,
        draws: 0,
        rating: 1200.0,
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PlayerError {
    #[error("Internal server error")]
    InternalServerError,
    #[error(transparent)]
    DatabaseError(surrealdb::Error),
}

impl From<surrealdb::Error> for PlayerError {
    fn from(error: surrealdb::Error) -> Self {
        PlayerError::DatabaseError(error)
    }
}

pub async fn get_or_insert_player(user_id: &str) -> Result<Player, PlayerError> {
    let player: Option<Player> = DB.select(("player", user_id)).await?;
    if let Some(player) = player {
        return Ok(player);
    }

    let new_player = create_player(user_id.to_string());
    let res: Option<Player> = DB.create(("player", user_id)).content(new_player).await?;
    if let Some(player) = res {
        return Ok(player);
    }
    Err(PlayerError::InternalServerError)
}

pub async fn add_game_result(
    player1_id: &str,
    player2_id: &str,
    result: GameResult,
) -> Result<(), PlayerError> {
    let mut player1 = get_or_insert_player(player1_id).await?;
    let mut player2 = get_or_insert_player(player2_id).await?;

    let s = match result {
        GameResult::Win => {
            player1.wins += 1;
            player2.losses += 1;
            1.0
        }
        GameResult::Draw => {
            player1.draws += 1;
            player2.draws += 1;
            0.5
        }
        GameResult::Loss => {
            player1.losses += 1;
            player2.wins += 1;
            0.0
        }
    };

    let expected_result_player1 =
        1f64 / (1.0 + 10f64.powf((player2.rating - player1.rating) / 400.0));
    let k_factor = 32.0; // K-factor for rating adjustment
    let gain = k_factor * (s - expected_result_player1);
    player1.rating += gain;
    player2.rating -= gain;

    println!(
        "Updating player {}: wins={}, losses={}, draws={}, rating={}",
        player1_id, player1.wins, player1.losses, player1.draws, player1.rating
    );
    println!(
        "Updating player {}: wins={}, losses={}, draws={}, rating={}",
        player2_id, player2.wins, player2.losses, player2.draws, player2.rating
    );

    let _: Option<Player> = DB.update(("player", player1_id)).content(player1).await?;
    let _: Option<Player> = DB.update(("player", player2_id)).content(player2).await?;
    Ok(())
}
