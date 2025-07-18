use crate::views::auth::do_logout;
use crate::Route;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::FaChartBar;
use dioxus_free_icons::icons::io_icons::{IoLogOut, IoSettings};
use dioxus_free_icons::Icon;

#[component]
pub fn More() -> Element {
    let mut is_logging_out = use_signal_sync(|| false);

    let nav = use_navigator();

    use_effect(move || {
        if *is_logging_out.read() {
            nav.push(Route::Auth {});
        }
    });

    let on_logout = move |_| {
        do_logout(move |_| {
            is_logging_out.set(true);
        });
    };

    rsx! {
        div { id: "more-view",
            MoreButton {
                onclick: |_| (),
                icon: MoreButtonIcon::Stats,
                label: "Stats",
            }
            MoreButton {
                onclick: |_| (),
                icon: MoreButtonIcon::Settings,
                label: "Settings",
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
    Settings,
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
        MoreButtonIcon::Settings => rsx! {
            Icon {
                width: 30,
                height: 30,
                fill: "black",
                icon: IoSettings,
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
