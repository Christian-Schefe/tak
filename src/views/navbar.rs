use crate::components::{NavButton, NavButtonIcon};
use crate::Route;
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        div {
            id: "navbar-top",
            div {
                id: "navbar-logo",
                "Tak"
            }
        }
        div {
            id: "navbar-content",
            Outlet::<Route> {}
        }

        div {
            id: "navbar",
            NavButton {
                to: Route::Home {},
                label: "Home",
                icon: NavButtonIcon::Home
            }
            NavButton {
                to: Route::Puzzles {},
                label: "Puzzles",
                icon: NavButtonIcon::Puzzle
            }
            NavButton {
                to: Route::More {},
                label: "More",
                icon: NavButtonIcon::More
            }
        }
    }
}
