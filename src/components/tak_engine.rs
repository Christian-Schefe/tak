use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo_worker::Spawnable;
use tak_core::{TakAction, TakCoord, TakPieceVariant};
use takumi::{TakumiWorker, TakumiWorkerInput};

use crate::components::tak_board_state::{PlayerType, TakBoardState};

#[component]
pub fn TakEngine() -> Element {
    let mut state = use_context::<TakBoardState>();

    use_effect(move || {
        let _ = state.on_change.read();

        if !state.check_ongoing_game() || !state.is_matching_player_turn(PlayerType::Computer) {
            return;
        }

        let (tps, size) = state
            .with_game(|game| (game.game().to_tps().to_string(), game.game().board.size))
            .expect("Game should exist to get TPS");
        dioxus::logger::tracing::info!("Starting minimax with position: {}", tps);
        let mut state = state.clone();
        spawn(async move {
            let mut bridge =
                TakumiWorker::spawner().spawn("/webworker/takumi_worker/takumi_worker.js");
            dioxus::logger::tracing::info!("sending");
            bridge.send(TakumiWorkerInput::new(tps, 3)).await.unwrap();
            dioxus::logger::tracing::info!("sent");
            let action = bridge.next().await.unwrap();
            let action = map_action(size, action);
            dioxus::logger::tracing::info!("received action: {:?}", action);
            state
                .with_game_mut(|game| {
                    game.try_do_action(action)
                        .expect("Applying best move should succeed");
                })
                .expect("Game should exist to apply best move");
        });
    });

    rsx! {}
}

fn map_action(size: usize, best_move: takumi::Action) -> TakAction {
    let best_move = match best_move {
        takumi::Action::Place(pos, variant) => TakAction::PlacePiece {
            pos: TakCoord {
                x: (pos % size) as i32,
                y: (size - 1 - (pos / size)) as i32,
            },
            variant: match variant {
                takumi::Board::VARIANT_FLAT => TakPieceVariant::Flat,
                takumi::Board::VARIANT_WALL => TakPieceVariant::Wall,
                takumi::Board::VARIANT_CAPSTONE => TakPieceVariant::Capstone,
                _ => panic!("Invalid piece variant in minimax move"),
            },
        },
        takumi::Action::Spread(pos, dir, take, spreads) => TakAction::MovePiece {
            pos: TakCoord {
                x: (pos % size) as i32,
                y: (size - 1 - (pos / size)) as i32,
            },
            dir: match dir {
                takumi::Board::DIR_UP => tak_core::TakDir::Up,
                takumi::Board::DIR_DOWN => tak_core::TakDir::Down,
                takumi::Board::DIR_LEFT => tak_core::TakDir::Left,
                takumi::Board::DIR_RIGHT => tak_core::TakDir::Right,
                _ => panic!("Invalid direction in minimax move"),
            },
            take: take as usize,
            drops: takumi::decode_spread_vec(spreads),
        },
    };
    best_move
}
