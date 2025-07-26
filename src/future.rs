pub async fn sleep(duration: std::time::Duration) {
    #[cfg(feature = "web")]
    gloo::timers::future::sleep(duration).await;

    #[cfg(not(feature = "web"))]
    tokio::time::sleep(duration).await;
}
