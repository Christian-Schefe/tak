use dioxus::prelude::*;

#[component]
pub fn More() -> Element {
    rsx! {
        div {
            id: "more",
            h1 { "More" }
            p { "This is the more section." }
            // You can add more content or components here as needed
        }
    }
}
