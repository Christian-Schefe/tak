use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::fa_solid_icons::{FaFlag, FaHandshake, FaHandshakeSlash},
};

#[component]
pub fn GameActionsOnline() -> Element {
    rsx! {
        div {
            class: "game-actions",
            GameActionButton { icon: GameActionIcon::Resign, onclick: move || {} }
            GameActionButton { icon: GameActionIcon::OfferDraw, onclick: move || {} }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameActionIcon {
    Resign,
    OfferDraw,
    CancelOffer,
}

#[component]
fn GameActionButton(icon: GameActionIcon, onclick: Callback) -> Element {
    let icon = match icon {
        GameActionIcon::Resign => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaFlag,
            }
        },
        GameActionIcon::OfferDraw => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaHandshake,
            }
        },
        GameActionIcon::CancelOffer => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaHandshakeSlash,
            }
        },
    };
    rsx! {
        button {
            onclick: move |_| {
                onclick.call(());
            },
            {icon}
        }
    }
}
