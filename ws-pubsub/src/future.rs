#[cfg(feature = "client-wasm")]
pub fn spawn_local<F>(f: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(feature = "client-wasm"))]
pub fn spawn_local<F>(f: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    tokio::task::spawn_local(f);
}
