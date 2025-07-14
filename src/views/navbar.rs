use crate::components::{NavButton, NavButtonIcon};
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{FaArrowLeft, FaChessBoard};
use dioxus_free_icons::Icon;

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
                Icon {
                    icon: FaChessBoard
                }
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
                to: Route::Rooms {},
                label: "Rooms",
                icon: NavButtonIcon::Room
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
