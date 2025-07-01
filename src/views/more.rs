use crate::views::auth::do_logout;
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn More() -> Element {
    let mut is_logging_out = use_signal_sync(|| false);

    let nav = use_navigator();

    use_effect(move || {
        if *is_logging_out.read() {
            nav.push(Route::Auth {});
        }
    });

    rsx! {
        div {
            id: "more",
            h1 { "More" }
            p { "This is the more section." }
            button {
                onclick: move |_| {
                    do_logout(move |_| {
                        is_logging_out.set(true);
                    });
                },
                "Logout"
            }
        }
    }
}
