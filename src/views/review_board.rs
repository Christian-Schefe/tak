use std::collections::HashMap;

use dioxus::prelude::*;
use tak_core::TakPlayer;

use crate::{
    components::{
        tak_board_state::{PlayerInfo, PlayerType, TakBoardState},
        TakBoard,
    },
    server::api::get_game,
};

#[component]
pub fn ReviewBoard(game_id: String) -> Element {
    let game = use_resource(move || get_game(game_id.clone()));

    let state = use_context_provider(|| TakBoardState::new(HashMap::new()));

    let mut board_clone = state.clone();
    let mut ply_index = use_signal(|| 0);

    use_effect(move || {
        let Some(Ok(Ok(game_info))) = &*game.read() else {
            return;
        };
        dioxus::logger::tracing::info!(
            "Reviewing game: {}, ptn: {}",
            game_info.game_id,
            game_info.ptn
        );
        if board_clone
            .try_set_from_ptn(game_info.ptn.clone())
            .is_none()
        {
            return;
        }
        let mut player_info = board_clone.player_info.write();
        player_info.insert(
            TakPlayer::White,
            PlayerInfo {
                name: game_info.white_player.username.clone(),
                rating: Some(game_info.white_player.rating),
                player_type: PlayerType::Remote,
            },
        );
        player_info.insert(
            TakPlayer::Black,
            PlayerInfo {
                name: game_info.black_player.username.clone(),
                rating: Some(game_info.black_player.rating),
                player_type: PlayerType::Remote,
            },
        );
        dioxus::logger::tracing::info!("Player info set: {:?}", *player_info);
        drop(player_info);
        ply_index.set(
            board_clone
                .with_game(|game| game.game().ply_index)
                .expect("Game should exist to get ply index"),
        );
    });

    let board_clone = state.clone();
    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        board_clone.has_game()
    });

    let mut board_clone = state.clone();

    use_effect(move || {
        let ply_index = *ply_index.read();
        if !board_clone.has_game() {
            dioxus::logger::tracing::info!("No game yet");
            return;
        }
        board_clone
            .with_game_mut(|game| {
                game.try_seek_ply_index(ply_index);
            })
            .expect("Should be able to seek to ply index");
    });

    let ply_index_clone = ply_index.clone();
    let on_press_backwards = move || {
        let mut ply_index_clone = ply_index_clone.clone();
        let val = *ply_index_clone.peek();
        if val > 0 {
            ply_index_clone.set(val - 1);
        }
    };

    let ply_index_clone = ply_index.clone();
    let on_press_forwards = move || {
        let mut ply_index_clone = ply_index_clone.clone();
        let val = *ply_index_clone.peek();
        if val
            < state
                .with_game(|game| game.game().ply_index)
                .expect("Game should have actions")
        {
            ply_index_clone.set(val + 1);
        }
    };

    let on_press_backwards_clone = on_press_backwards.clone();
    let on_press_forwards_clone = on_press_forwards.clone();
    let on_key_down = move |event: KeyboardEvent| {
        if event.key() == Key::ArrowLeft {
            on_press_backwards_clone();
        } else if event.key() == Key::ArrowRight {
            on_press_forwards_clone();
        }
    };

    rsx! {
        div { id: "review-board-view", onkeydown: on_key_down,
            if *show_board.read() {
                TakBoard {}
                div { class: "review-board-controls",
                    button { onclick: move |_| on_press_backwards(), "<" }
                    button { onclick: move |_| on_press_forwards(), ">" }
                }
            }
        }
    }
}
