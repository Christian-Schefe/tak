use crate::Route;
use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/styling/home.css");

#[component]
pub fn Home() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-options",
            Link {
                to: Route::PlayComputer {},
                "Play Online"
            }
            Link {
                to: Route::PlayComputer {},
                "Play Computer"
            }
        }
    }
}
