use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::*;
use dioxus_free_icons::Icon;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavButtonIcon {
    Home,
    Puzzle,
    More,
}

#[component]
pub fn NavButton(to: Route, label: String, icon: NavButtonIcon) -> Element {
    let icon = match icon {
        NavButtonIcon::Home => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaHouse,
            }
        },
        NavButtonIcon::Puzzle => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaPuzzlePiece,
            }
        },
        NavButtonIcon::More => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaBars
            }
        },
    };
    rsx! {
        Link {
            to,
            class: "nav-button",
            active_class: "nav-button-active",
            div {
                {icon},
                "{label}"
            }
        }
    }
}
