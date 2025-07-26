use dioxus::prelude::server_fn::{
    client::{Client, browser::BrowserClient},
    request::browser::BrowserRequest,
    response::browser::BrowserResponse,
};

use crate::views::AUTH_TOKEN_KEY;

pub struct AuthClient;

#[cfg(any(feature = "web", feature = "server"))]
impl<CustomErr> Client<CustomErr> for AuthClient {
    type Request = BrowserRequest;
    type Response = BrowserResponse;

    fn send(
        req: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, dioxus::prelude::ServerFnError<CustomErr>>> + Send
    {
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let headers = req.headers();

        if let Some(token) = &token {
            headers.append("Authorization", &format!("Bearer {token}"));
        }

        BrowserClient::send(req)
    }
}

#[cfg(not(any(feature = "web", feature = "server")))]
impl<CustomErr> Client<CustomErr> for AuthClient {
    type Request =
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<CustomErr>>::Request;
    type Response =
        <dioxus::prelude::server_fn::client::reqwest::ReqwestClient as Client<CustomErr>>::Response;

    fn send(
        mut req: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, dioxus::prelude::ServerFnError<CustomErr>>> + Send
    {
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let headers = req.headers_mut();

        if let Some(token) = &token {
            headers.append("Authorization", format!("Bearer {token}").parse().unwrap());
        }

        dioxus::prelude::server_fn::client::reqwest::ReqwestClient::send(req)
    }
}
