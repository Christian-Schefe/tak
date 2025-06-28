use crate::Route;
use dioxus::prelude::*;

const PUZZLES_CSS: Asset = asset!("/assets/styling/puzzles.css");

#[component]
pub fn Puzzles() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: PUZZLES_CSS }

        div {
            id: "puzzles",
        }
    }
}
