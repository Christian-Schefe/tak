use crate::components::tak_board_state::TakBoardState;
use crate::server::api::{MATCHES_TOPIC, MyServerFunctions};
use dioxus::core_macro::component;
use dioxus::prelude::*;
use tak_core::{TakAction, TakGameState, TakPlayer};
use ws_pubsub::{use_ws_topic_receive, use_ws_topic_send};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ServerGameMessage {
    StartGame,
    Move(usize, Vec<(TakPlayer, u64)>, String),
    GameOver(TakGameState),
}

#[component]
pub fn TakWebSocket(match_id: String) -> Element {
    let board = use_context::<TakBoardState>();

    let board_clone = board.clone();

    let handle_game_message = async move |board: &mut TakBoardState, msg: ServerGameMessage| {
        match msg {
            ServerGameMessage::StartGame => {
                dioxus::logger::tracing::info!("[WebSocket] Starting game");
                board.reset();
                board.update_from_remote().await;
            }
            ServerGameMessage::Move(move_index, time_remaining, action) => {
                dioxus::logger::tracing::info!("[WebSocket] Processing move action: {action}");
                let Some(action) = TakAction::from_ptn(&action) else {
                    dioxus::logger::tracing::error!(
                        "[WebSocket] Invalid action received: {action}"
                    );
                    return;
                };
                let should_resync = board
                    .maybe_try_do_remote_action(move_index, action)
                    .is_err();
                for (player, duration) in time_remaining {
                    board.set_time_remaining(player, duration);
                }
                if should_resync {
                    dioxus::logger::tracing::info!(
                        "[WebSocket] Resyncing game state after message"
                    );
                    board.update_from_remote().await;
                }
            }
            ServerGameMessage::GameOver(game_state) => {
                dioxus::logger::tracing::info!("[WebSocket] Game over: {game_state:?}");
                if board
                    .with_game(|game| game.game().game_state != game_state)
                    .unwrap_or(true)
                {
                    board.update_from_remote().await;
                }
            }
        };
    };

    use_ws_topic_receive::<_, MyServerFunctions, _>(
        format!("{}/{}", MATCHES_TOPIC, match_id),
        move |msg| {
            let mut board = board_clone.clone();
            async move {
                handle_game_message(&mut board, msg).await;
            }
        },
    );

    let send_service = use_ws_topic_send(format!("{}/{}", MATCHES_TOPIC, match_id));

    use_effect(move || {
        dioxus::logger::tracing::info!(
            "[WebSocket] Messages queue length: {}",
            board.message_queue.len()
        );
        if board.message_queue.len() > 0 {
            let mut msg_queue = board.message_queue.clone();
            let send_service = send_service.clone();
            spawn(async move {
                for message in msg_queue.write().drain(..) {
                    dioxus::logger::tracing::info!("[WebSocket] Sending message: {:?}", message);
                    match send_service.send(message).await {
                        None => {
                            dioxus::logger::tracing::error!(
                                "[WebSocket] Send service is not running, skipping message"
                            );
                        }
                        Some(Err(e)) => {
                            dioxus::logger::tracing::error!(
                                "[WebSocket] Failed to send message: {}",
                                e
                            );
                        }
                        Some(Ok(())) => {
                            dioxus::logger::tracing::info!("[WebSocket] Message sent successfully");
                        }
                    }
                }
            });
        }
    });

    rsx! {}
}
