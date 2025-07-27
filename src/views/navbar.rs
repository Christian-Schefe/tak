use crate::Route;
use crate::components::{NavButton, NavButtonIcon};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fa_solid_icons::FaChessBoard;

#[component]
pub fn Navbar() -> Element {
    rsx! {
        div { id: "navbar-top",
            div { id: "navbar-logo",
                Icon { icon: FaChessBoard }
                "Tak"
            }
        }
        div { id: "navbar-content", Outlet::<Route> {} }

        div { id: "navbar",
            NavButton {
                to: Route::Home {},
                label: "Home",
                icon: NavButtonIcon::Home,
            }
            NavButton {
                to: Route::Rooms {},
                label: "Rooms",
                icon: NavButtonIcon::Room,
            }
            NavButton {
                to: Route::Puzzles {},
                label: "Puzzles",
                icon: NavButtonIcon::Puzzle,
            }
            NavButton {
                to: Route::More {},
                label: "More",
                icon: NavButtonIcon::More,
            }
        }
    }
}
