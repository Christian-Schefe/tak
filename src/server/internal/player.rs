use tak_core::{TakGame, TakGameState, TakPlayer};
use uuid::Uuid;

use crate::server::{
    GameId, UserId,
    error::{ServerError, ServerResult},
    internal::{
        db::DB,
        dto::{GameRecord, PlayerRecord, Record},
    },
};

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
    let ptn = game.to_ptn();

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

    let white_player = super::cache::retrieve_player_info(&white_player_id).await?;
    let black_player = super::cache::retrieve_player_info(&black_player_id).await?;

    let game_record = GameRecord {
        game_id: game_id.clone(),
        white_player,
        black_player,
        ptn: ptn.to_str(),
        timestamp: chrono::Utc::now().into(),
    };

    println!(
        "Adding game: {}, white: {}, black: {}, ptn: {}",
        game_id, white_player_id, black_player_id, game_record.ptn
    );

    super::dto::try_create(&game_id, game_record).await?;
    add_game_result(&white_player_id, &black_player_id, &game).await?;
    Ok(())
}

pub async fn add_game_result(
    white_player_id: &UserId,
    black_player_id: &UserId,
    game: &TakGame,
) -> ServerResult<()> {
    let mut player1 = get_or_insert_player(white_player_id).await?;
    let mut player2 = get_or_insert_player(black_player_id).await?;

    let s = match game.game_state {
        TakGameState::Win(TakPlayer::White, _) => {
            player1.wins += 1;
            player2.losses += 1;
            1.0
        }
        TakGameState::Draw => {
            player1.draws += 1;
            player2.draws += 1;
            0.5
        }
        TakGameState::Win(TakPlayer::Black, _) => {
            player1.losses += 1;
            player2.wins += 1;
            0.0
        }
        TakGameState::Canceled => {
            return Err(ServerError::InternalServerError(
                "Game was canceled, cannot update results".to_string(),
            ));
        }
        TakGameState::Ongoing => {
            return Err(ServerError::InternalServerError(
                "Game is still ongoing, cannot update results".to_string(),
            ));
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
        white_player_id, player1.wins, player1.losses, player1.draws, player1.rating
    );
    println!(
        "Updating player {}: wins={}, losses={}, draws={}, rating={}",
        black_player_id, player2.wins, player2.losses, player2.draws, player2.rating
    );

    super::dto::try_update(white_player_id, player1).await?;
    super::dto::try_update(black_player_id, player2).await?;
    Ok(())
}

pub async fn get_game(game_id: &GameId) -> ServerResult<GameRecord> {
    let game = super::dto::try_get(game_id).await?;
    Ok(game)
}

pub async fn get_games_of_player(
    user_id: &UserId,
    pagination: Option<(usize, usize)>,
) -> ServerResult<Vec<GameRecord>> {
    let query = match pagination {
        Some(_) => format!(
            "SELECT * FROM type::table($table) WHERE black_player.user_id = type::string($user_id) OR white_player.user_id = type::string($user_id) ORDER BY timestamp DESC START $offset LIMIT $limit"
        ),
        None => format!(
            "SELECT * FROM type::table($table) WHERE black_player.user_id = type::string($user_id) OR white_player.user_id = type::string($user_id) ORDER BY timestamp DESC"
        ),
    };
    let mut q = DB
        .query(&query)
        .bind(("table", GameRecord::table_name()))
        .bind(("user_id", user_id.clone()));
    if let Some((page, page_size)) = pagination {
        let offset = page * page_size;
        q = q
            .bind(("offset", offset as i64))
            .bind(("limit", page_size as i64));
    }
    let mut result = q.await?;
    let games: Vec<GameRecord> = result.take(0)?;
    Ok(games)
}
