use std::{cell::RefCell, rc::Rc, sync::Arc};

use dioxus::hooks::UnboundedSender;
use futures::{
    StreamExt,
    channel::{mpsc, oneshot},
};

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
    #[cfg(all(feature = "client-wasm", not(feature = "client-native")))]
    gloo::timers::future::sleep(duration).await;

    #[cfg(feature = "client-native")]
    tokio::time::sleep(duration).await;
}

pub struct Service<T, R> {
    pub sender: Arc<UnboundedSender<(T, oneshot::Sender<R>)>>,
    pub receiver: Rc<RefCell<mpsc::UnboundedReceiver<(T, oneshot::Sender<R>)>>>,
}

impl<T, R> Clone for Service<T, R> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl<T, R> Service<T, R> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            sender: Arc::new(sender),
            receiver: Rc::new(RefCell::new(receiver)),
        }
    }

    pub async fn send(&self, msg: T) -> R {
        let (tx, rx) = oneshot::channel();
        self.sender.unbounded_send((msg, tx)).unwrap();
        rx.await.unwrap()
    }
}

pub async fn run_service<T: 'static, R: 'static, Input, Fut>(
    service: Service<T, R>,
    mut input: Input,
    handler: impl Fn(Input, T) -> Fut + 'static,
) where
    Fut: std::future::Future<Output = (R, Input)>,
{
    let Ok(mut receiver) = service.receiver.try_borrow_mut() else {
        dioxus::logger::tracing::error!("Failed to get mutable reference to receiver");
        return;
    };
    while let Some((msg, reply)) = receiver.next().await {
        let (res, new_input) = handler(input, msg).await;
        input = new_input;
        let _ = reply.send(res);
    }
}
