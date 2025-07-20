use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{reactor, ReactorScope};

use crate::{determine_time_to_use, iterative_deepening, Action, Board, Settings};

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) =>
        (web_sys::console::log_1(
            &wasm_bindgen::JsValue::from_str(&format!($($t)*))))
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakumiWorkerInput {
    position: String,
    max_depth: usize,
    settings: Settings,
    time_remaining: u64,
    increment: u64,
}

impl TakumiWorkerInput {
    pub fn new(
        position: String,
        max_depth: usize,
        settings: Settings,
        time_remaining: u64,
        increment: u64,
    ) -> Self {
        Self {
            position,
            max_depth,
            settings,
            time_remaining,
            increment,
        }
    }
}

#[reactor]
pub async fn TakumiWorker(mut scope: ReactorScope<TakumiWorkerInput, Action>) {
    console_log!("TestWorker function triggered");
    while let Some(input) = scope.next().await {
        let mut board = Board::try_from_pos_str(&input.position, input.settings)
            .expect("Failed to create board from TPS");

        let time_to_use = determine_time_to_use(&board, input.time_remaining, input.increment);
        console_log!("Determined time to use: {} ms", time_to_use);
        let (score, reached_depth, action) =
            iterative_deepening(&mut board, input.max_depth, time_to_use);

        console_log!(
            "Best move calculated: {:?} with score {} at depth {}",
            action,
            score,
            reached_depth
        );
        scope
            .send(action.expect("Should have a best move"))
            .await
            .expect("Failed to send action");
    }
}
