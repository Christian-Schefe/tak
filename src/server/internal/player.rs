use tak_core::{TakGame, TakPlayer};
use uuid::Uuid;

use crate::server::{
    error::{ServerError, ServerResult},
    internal::dto::{GameRecord, PlayerRecord},
    UserId,
};

#[derive(Debug, Clone)]
pub enum GameResult {
    Win,
    Draw,
    Loss,
}

pub fn create_player(user_id: &str) -> PlayerRecord {
    PlayerRecord {
        user_id: user_id.to_string(),
        wins: 0,
        losses: 0,
        draws: 0,
        rating: 1200.0,
    }
}

pub async fn get_or_insert_player(user_id: &UserId) -> ServerResult<PlayerRecord> {
    super::dto::try_get_or_insert(user_id, || create_player(user_id)).await
}

pub async fn add_game(
    game: TakGame,
    player_mapping: fixed_map::Map<TakPlayer, String>,
) -> ServerResult<()> {
    let game_id = Uuid::new_v4().to_string();
    let ptn = game.to_ptn().to_str();

    let white_player_id = player_mapping
        .get(TakPlayer::White)
        .ok_or_else(|| {
            ServerError::InternalServerError("Failed to get white player ID".to_string())
        })?
        .to_string();
    let black_player_id = player_mapping
        .get(TakPlayer::Black)
        .ok_or_else(|| {
            ServerError::InternalServerError("Failed to get black player ID".to_string())
        })?
        .to_string();

    let white_player = super::cache::get_or_retrieve_player_info(&white_player_id).await?;
    let black_player = super::cache::get_or_retrieve_player_info(&black_player_id).await?;

    let game_record = GameRecord {
        game_id: game_id.clone(),
        white_player,
        black_player,
        ptn,
    };

    super::dto::try_create(&game_id, game_record).await?;
    Ok(())
}

pub async fn add_game_result(
    player1_id: &UserId,
    player2_id: &UserId,
    result: GameResult,
) -> ServerResult<()> {
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

    super::dto::try_update(player1_id, player1).await?;
    super::dto::try_update(player2_id, player2).await?;
    Ok(())
}
