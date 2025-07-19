use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{reactor, ReactorScope};

use crate::{iterative_deepening, Action, Board, Settings};

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
    depth: usize,
    settings: Settings,
    max_duration: u64,
}

impl TakumiWorkerInput {
    pub fn new(position: String, depth: usize, settings: Settings, max_duration: u64) -> Self {
        Self {
            position,
            depth,
            settings,
            max_duration,
        }
    }
}

#[reactor]
pub async fn TakumiWorker(mut scope: ReactorScope<TakumiWorkerInput, Action>) {
    console_log!("TestWorker function triggered");
    while let Some(input) = scope.next().await {
        let mut board = Board::try_from_pos_str(&input.position, input.settings)
            .expect("Failed to create board from TPS");
        let (score, reached_depth, action) =
            iterative_deepening(&mut board, input.depth, input.max_duration);
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
