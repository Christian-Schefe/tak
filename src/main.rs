use crate::views::Auth;
use dioxus::prelude::*;
use views::{
    CreateRoomComputer, CreateRoomLocal, CreateRoomOnline, History, Home, More, Navbar,
    PlayComputer, PlayLocal, PlayOnline, Puzzles, ReviewBoard, Rooms, Rules, Stats,
};

mod components;
mod server;
mod views;
mod storage;

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
    #[route("/more/rules")]
    Rules {},
    #[route("/more/stats")]
    Stats {},
    #[route("/more/history")]
    History {},

    #[route("/review/:game_id")]
    ReviewBoard { game_id: String },

    #[route("/play/computer")]
    PlayComputer {},
    #[route("/play/local")]
    PlayLocal {},
    #[route("/play/online")]
    PlayOnline {},

    #[route("/create/online")]
    CreateRoomOnline {},
    #[route("/create/local")]
    CreateRoomLocal {},
    #[route("/create/computer")]
    CreateRoomComputer {},

    #[route("/rooms")]
    Rooms {},

    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
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
    use dioxus_fullstack::server::DioxusRouterExt;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::spawn;
    use tower_cookies::CookieManagerLayer;

    spawn(async move {
        let db_url = std::env::var("DB_URL").unwrap_or_else(|_| "localhost:8000".to_string());

        if let Err(e) = server::internal::db::connect_db(&db_url).await {
            eprintln!("Failed to connect to database: {}", e);
            return;
        }
        if let Err(e) = server::internal::dto::setup_db().await {
            eprintln!("Failed to set up database: {}", e);
            return;
        }
    });

    let ip =
        dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);

    let config = ServeConfig::new().unwrap();

    let router = axum::Router::new()
        .serve_dioxus_application(config, App)
        .route(
            "/ws",
            axum::routing::any(server::internal::websocket::ws_handler),
        )
        .route(
            "/ws2",
            axum::routing::any(server::internal::websocket::ws_test_handler),
        )
        .nest_service("/webworker", tower_http::services::ServeDir::new("workers"))
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

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        {format!("Not Found: {}", route.join("/"))}
    }
}
