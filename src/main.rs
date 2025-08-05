use crate::{
    components::{ColorApplier, PubSubClient},
    server::MatchId,
    views::Auth,
};
use dioxus::prelude::*;
use flexi_logger::{
    DeferredNow,
    filter::{LogLineFilter, LogLineWriter},
};
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
    #[route("/play/online/:match_id")]
    PlayOnline { match_id: MatchId },

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

pub struct NoDioxusFilter;
impl LogLineFilter for NoDioxusFilter {
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &log::Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()> {
        if !record
            .module_path()
            .is_some_and(|x| x.starts_with("dioxus_signals"))
        {
            log_line_writer.write(now, record)?;
        }
        Ok(())
    }
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
    use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::spawn;

    let _logger = Logger::try_with_str("info, my::critical::module=trace")
        .expect("Failed to initialize logger")
        .log_to_file(FileSpec::default().directory("logs"))
        .write_mode(WriteMode::Async)
        .filter(Box::new(NoDioxusFilter))
        .rotate(
            Criterion::Size(10 * 1024 * 1024),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(7),
        )
        .start()
        .expect("Failed to start logger");

    log::info!("Server starting...");

    spawn(async move {
        let db_url = std::env::var("DB_URL").unwrap_or_else(|_| "localhost:8000".to_string());
        let db_user = std::env::var("SURREALDB_USER").expect("SURREALDB_USER not set");
        let db_pass = std::env::var("SURREALDB_PASS").expect("SURREALDB_PASS not set");

        if let Err(e) = server::internal::db::connect_db(&db_url, &db_user, &db_pass).await {
            log::error!("Failed to connect to database: {}", e);
            return;
        }
        if let Err(e) = server::internal::dto::setup_db().await {
            log::error!("Failed to set up database: {}", e);
            return;
        }
    });

    server::internal::pub_sub::setup_handlers();

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

    log::info!("Server running at: {}", address);

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
        PubSubClient {}
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        {format!("Not Found: {}", route.join("/"))}
    }
}
