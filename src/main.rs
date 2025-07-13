use crate::views::Auth;
use dioxus::prelude::*;
use views::{CreateRoom, Home, More, Navbar, PlayComputer, PlayOnline, Puzzles};

mod components;
mod server;
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
    #[route("/createroom")]
    CreateRoom {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.scss");

#[cfg(not(feature = "server"))]
fn main() {
    let server_url = option_env!("SERVER_URL").unwrap_or("http://localhost:8080");
    dioxus::logger::tracing::info!("[Client] Using server URL: {server_url}");
    server_fn::client::set_server_url(server_url);
    launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::Extension;
    use dioxus_fullstack::server::DioxusRouterExt;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::spawn;
    use tower_cookies::CookieManagerLayer;

    spawn(async move {
        let db_url = std::env::var("DB_URL").unwrap_or_else(|_| "localhost:8000".to_string());

        if let Err(e) = server::auth::connect_db(&db_url).await {
            eprintln!("Failed to connect to database: {}", e);
        }
    });

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
        .route(
            "/ws2",
            axum::routing::any(server::websocket::ws_test_handler),
        )
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
