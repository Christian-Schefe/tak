use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{reactor, ReactorScope};

use crate::{minimax, Action, Board};

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
}

impl TakumiWorkerInput {
    pub fn new(position: String, depth: usize) -> Self {
        Self { position, depth }
    }
}

#[reactor]
pub async fn TakumiWorker(mut scope: ReactorScope<TakumiWorkerInput, Action>) {
    console_log!("TestWorker function triggered");
    while let Some(input) = scope.next().await {
        let mut board =
            Board::try_from_pos_str(&input.position).expect("Failed to create board from TPS");
        let (_, action) = minimax(&mut board, input.depth);
        console_log!("Best move calculated: {:?}", action);
        scope
            .send(action.expect("Should have a best move"))
            .await
            .expect("Failed to send action");
    }
}
