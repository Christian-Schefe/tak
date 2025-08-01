#[cfg(feature = "client-wasm")]
pub fn spawn_maybe_local<F>(f: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(feature = "client-wasm"))]
pub fn spawn_maybe_local<F>(f: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::task::spawn(f);
}

pub async fn sleep(duration: std::time::Duration) {
    #[cfg(not(feature = "client-native"))]
    gloo::timers::future::sleep(duration).await;

    #[cfg(feature = "client-native")]
    tokio::time::sleep(duration).await;
}
