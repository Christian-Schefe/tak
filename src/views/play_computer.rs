use crate::Route;
use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/styling/play_computer.css");

#[component]
pub fn PlayComputer() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            
        }
    }
}
