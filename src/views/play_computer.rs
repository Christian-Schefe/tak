use crate::components::{TakBoard, TakWebSocket};
use crate::Route;
use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/styling/computer.css");

#[component]
pub fn PlayComputer() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-computer",
            TakBoard {

            }
            TakWebSocket {}
        }
    }
}
