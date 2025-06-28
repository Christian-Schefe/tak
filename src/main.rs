use dioxus::prelude::*;

use views::{Home, More, Navbar, PlayComputer, Puzzles};

mod components;
mod tak;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/puzzles")]
    Puzzles {},
    #[route("/more")]
    More {},
    #[route("/play-computer")]
    PlayComputer {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

fn main() {
    //launch(App);
    crate::tak::test_tak_game();
    crate::tak::test_read_tak_game();
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}
