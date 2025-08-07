use dioxus_fullstack::client::Client;
use dioxus_fullstack::server_fn::error::FromServerFnError;

use crate::views::AUTH_TOKEN_KEY;

pub struct AuthClient;

#[cfg(feature = "web")]
impl<Error: FromServerFnError> Client<Error> for AuthClient {
    type Request = dioxus::prelude::server_fn::request::browser::BrowserRequest;
    type Response = dioxus::prelude::server_fn::response::browser::BrowserResponse;

    fn send(req: Self::Request) -> impl Future<Output = Result<Self::Response, Error>> + Send {
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let headers = req.headers();

        if let Some(token) = &token {
            headers.append("Authorization", &format!("Bearer {token}"));
        }

        <dioxus_fullstack::client::browser::BrowserClient as Client<Error>>::send(req)
    }

    fn open_websocket(
        path: &str,
    ) -> impl Future<
        Output = Result<
            (
                impl futures_util::Stream<
                    Item = Result<tokio_tungstenite_wasm::Bytes, tokio_tungstenite_wasm::Bytes>,
                > + Send
                + 'static,
                impl futures_util::Sink<tokio_tungstenite_wasm::Bytes> + Send + 'static,
            ),
            Error,
        >,
    > + Send {
        <dioxus_fullstack::client::browser::BrowserClient as Client<Error>>::open_websocket(path)
    }

    fn spawn(future: impl Future<Output = ()> + Send + 'static) {
        <dioxus_fullstack::client::browser::BrowserClient as Client<Error>>::spawn(future)
    }
}

#[cfg(not(feature = "web"))]
impl<Error: FromServerFnError> Client<Error> for AuthClient {
    type Request =
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<Error>>::Request;
    type Response =
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<Error>>::Response;

    fn send(mut req: Self::Request) -> impl Future<Output = Result<Self::Response, Error>> + Send {
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let headers = req.headers_mut();

        if let Some(token) = &token {
            headers.append("Authorization", format!("Bearer {token}").parse().unwrap());
        }

        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<Error>>::send(req)
    }

    fn open_websocket(
        path: &str,
    ) -> impl Future<
        Output = Result<
            (
                impl futures_util::Stream<
                    Item = Result<tokio_tungstenite_wasm::Bytes, tokio_tungstenite_wasm::Bytes>,
                > + Send
                + 'static,
                impl futures_util::Sink<tokio_tungstenite_wasm::Bytes> + Send + 'static,
            ),
            Error,
        >,
    > + Send {
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<Error>>::open_websocket(path)
    }

    fn spawn(future: impl Future<Output = ()> + Send + 'static) {
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<Error>>::spawn(future)
    }
}
