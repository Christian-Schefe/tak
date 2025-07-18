use gloo_worker::Registrable;
use takumi::TakumiWorker;
use wasm_bindgen::prelude::wasm_bindgen;

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) =>
        (web_sys::console::log_1(
            &wasm_bindgen::JsValue::from_str(&format!($($t)*))))
}

#[wasm_bindgen(start)]
pub async fn start_worker() {
    console_log!("worker started");
    TakumiWorker::registrar().register();
}
