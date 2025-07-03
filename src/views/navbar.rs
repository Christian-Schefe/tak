use crate::components::{NavButton, NavButtonIcon};
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::FaArrowLeft;
use dioxus_free_icons::Icon;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let nav = use_navigator();

    let on_back_click = move |_| {
        if nav.can_go_back() {
            nav.go_back();
        } else {
            nav.push(Route::Home {});
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        div {
            id: "navbar-top",
            button {
                id: "navbar-back-button",
                onclick: on_back_click,
                Icon {
                    icon: FaArrowLeft
                }
            }
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
