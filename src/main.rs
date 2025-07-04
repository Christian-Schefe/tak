use crate::views::Auth;
use dioxus::prelude::*;
use views::{Home, More, Navbar, PlayComputer, PlayOnline, Puzzles};

mod components;
mod server;
mod tak;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[route("/auth")]
    Auth {},
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/puzzles")]
    Puzzles {},
    #[route("/more")]
    More {},
    #[route("/playcomputer")]
    PlayComputer {},
    #[route("/playonline")]
    PlayOnline {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

#[cfg(not(feature = "server"))]
fn main() {
    launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::Extension;
    use dioxus_fullstack::server::DioxusRouterExt;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower_cookies::CookieManagerLayer;

    if let Err(e) = server::auth::connect_db().await {
        eprintln!("Failed to connect to database: {}", e);
    }

    let ip =
        dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);

    let shared_state = server::websocket::SharedState::new();
    let session_store = server::auth::create_session_store();

    let config = ServeConfig::new().unwrap();

    let router = axum::Router::new()
        .serve_dioxus_application(config, App)
        .route("/ws", axum::routing::any(server::websocket::ws_handler))
        .route("/ws2", axum::routing::any(server::websocket::ws_test_handler))
        .layer(Extension(session_store))
        .layer(Extension(shared_state))
        .layer(CookieManagerLayer::new())
        .into_make_service_with_connect_info::<SocketAddr>();

    println!("Server running at {}", address);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}
