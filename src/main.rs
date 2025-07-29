use crate::{
    components::{ColorApplier, PlaytakClient},
    views::Auth,
};
use dioxus::prelude::*;
use views::{
    Colors, CreateRoomComputer, CreateRoomLocal, CreateRoomOnline, History, Home, More, Navbar,
    PlayComputer, PlayLocal, PlayOnline, Puzzles, ReviewBoard, Rules, Seeks, Settings, Stats,
};

mod components;
mod future;
mod server;
mod storage;
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
    #[route("/more/rules")]
    Rules {},
    #[route("/more/stats")]
    Stats {},
    #[route("/more/history")]
    History {},
    #[route("/more/colors")]
    Colors {},
    #[route("/more/settings")]
    Settings {},

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

    #[route("/seeks")]
    Seeks {},

    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

const _MAIN_HTML: Asset = asset!("/index.html");
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.scss");

fn check_features() {
    #[cfg(feature = "web")]
    println!("Running in web mode");

    #[cfg(feature = "desktop")]
    println!("Running in desktop mode");

    #[cfg(feature = "mobile")]
    println!("Running in mobile mode");

    #[cfg(feature = "server")]
    println!("Running in server mode");
}

#[cfg(not(feature = "server"))]
fn main() {
    check_features();
    let server_url = option_env!("SERVER_URL").unwrap_or("http://localhost:8080");
    dioxus::logger::tracing::info!("[Client] Using server URL: {server_url}");
    server_fn::client::set_server_url(server_url);
    launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    check_features();
    use dioxus_fullstack::server::DioxusRouterExt;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::spawn;

    println!("Starting server...");

    spawn(async move {
        let db_url = std::env::var("DB_URL").unwrap_or_else(|_| "localhost:8000".to_string());
        let Ok(db_user) = std::env::var("SURREALDB_USER") else {
            eprintln!("SURREALDB_USER not set");
            return;
        };
        let Ok(db_pass) = std::env::var("SURREALDB_PASS") else {
            eprintln!("SURREALDB_PASS not set");
            return;
        };

        if let Err(e) = server::internal::db::connect_db(&db_url, &db_user, &db_pass).await {
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
        ColorApplier {}
        PlaytakClient {}
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        {format!("Not Found: {}", route.join("/"))}
    }
}
