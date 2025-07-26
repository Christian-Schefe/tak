mod colors;
mod history;
mod rules;
mod stats;

pub use colors::*;
pub use history::*;
pub use rules::*;
pub use stats::*;

use crate::Route;
use crate::views::AUTH_TOKEN_KEY;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fa_solid_icons::{FaBookOpen, FaChartBar, FaPalette, FaScroll};
use dioxus_free_icons::icons::io_icons::{IoLogOut, IoSettings};

#[component]
pub fn More() -> Element {
    let nav = use_navigator();

    let on_logout = move |_| {
        if let Err(e) = crate::storage::set(AUTH_TOKEN_KEY, None::<String>) {
            dioxus::logger::tracing::error!("[More] Error logging out: {}", e);
        }
        dioxus::logger::tracing::info!("User logged out");
        nav.push(Route::Auth {});
    };

    let on_click_rules = move |_| {
        nav.push(Route::Rules {});
    };

    let on_click_stats = move |_| {
        nav.push(Route::Stats {});
    };

    let on_click_history = move |_| {
        nav.push(Route::History {});
    };

    let on_click_colors = move |_| {
        nav.push(Route::Colors {});
    };

    rsx! {
        div { id: "more-view",
            MoreButton {
                onclick: on_click_stats,
                icon: MoreButtonIcon::Stats,
                label: "Stats",
            }
            MoreButton {
                onclick: on_click_rules,
                icon: MoreButtonIcon::Rules,
                label: "Rules",
            }
            MoreButton {
                onclick: on_click_history,
                icon: MoreButtonIcon::History,
                label: "History",
            }
            MoreButton {
                onclick: |_| (),
                icon: MoreButtonIcon::Settings,
                label: "Settings",
            }
            MoreButton {
                onclick: on_click_colors,
                icon: MoreButtonIcon::Theme,
                label: "Theme",
            }
            MoreButton {
                onclick: on_logout,
                icon: MoreButtonIcon::Logout,
                label: "Logout",
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoreButtonIcon {
    Stats,
    Rules,
    History,
    Settings,
    Theme,
    Logout,
}

#[component]
pub fn MoreButton(
    icon: MoreButtonIcon,
    label: String,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let icon = match icon {
        MoreButtonIcon::Stats => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaChartBar,
            }
        },
        MoreButtonIcon::Rules => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaBookOpen,
            }
        },
        MoreButtonIcon::History => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaScroll,
            }
        },
        MoreButtonIcon::Settings => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: IoSettings,
            }
        },
        MoreButtonIcon::Theme => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: FaPalette,
            }
        },
        MoreButtonIcon::Logout => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: IoLogOut,
            }
        },
    };

    rsx! {
        button { class: "more-button", onclick,
            {icon}
            "{label}"
        }
    }
}
