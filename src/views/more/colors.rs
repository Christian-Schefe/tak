use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::{COLOR_SCHEME_COLORS, COLOR_SCHEME_KEY, ColorScheme};

#[component]
pub fn Colors() -> Element {
    let unset_theme = |_| {
        unset_color_scheme();
    };

    rsx! {
        div { id: "colors-view",
            h1 { "Themes" }
            button { class: "color-theme-button", onclick: unset_theme, "Classic" }
            for scheme in ColorSchemes::ALL.iter() {
                button {
                    class: "color-theme-button",
                    onclick: move |_| set_color_scheme(scheme.clone()),
                    "{scheme.name()}"
                }
            }
        }
    }
}

fn set_color_scheme(scheme: ColorSchemes) {
    let Some(colors) = scheme.scheme() else {
        dioxus::logger::tracing::error!("Failed to get color scheme for: {:?}", scheme);
        return;
    };
    colors.apply();
    if let Err(e) = crate::storage::set(COLOR_SCHEME_KEY, scheme) {
        dioxus::logger::tracing::error!("Failed to set color scheme: {}", e);
    }
    if let Err(e) = crate::storage::set(COLOR_SCHEME_COLORS, colors.to_json_array()) {
        dioxus::logger::tracing::error!("Failed to set color scheme colors: {}", e);
    }
}

fn unset_color_scheme() {
    ColorScheme::default().unset();
    if let Err(e) = crate::storage::remove(COLOR_SCHEME_KEY) {
        dioxus::logger::tracing::error!("Failed to unset color scheme: {}", e);
    }
    if let Err(e) = crate::storage::remove(COLOR_SCHEME_COLORS) {
        dioxus::logger::tracing::error!("Failed to unset color scheme colors: {}", e);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColorSchemes {
    Jungle,
    Aether,
    Ignis,
    Bubblegum,
}

impl ColorSchemes {
    pub const ALL: [ColorSchemes; 4] = [
        ColorSchemes::Jungle,
        ColorSchemes::Aether,
        ColorSchemes::Ignis,
        ColorSchemes::Bubblegum,
    ];

    pub fn scheme(&self) -> Option<ColorScheme> {
        serde_json::from_str(match self {
            ColorSchemes::Jungle => JUNGLE_THEME,
            ColorSchemes::Aether => AETHER_THEME,
            ColorSchemes::Ignis => IGNIS_THEME,
            ColorSchemes::Bubblegum => BUBBLEGUM_THEME,
        })
        .ok()
    }

    fn name(&self) -> &str {
        match self {
            ColorSchemes::Jungle => "Jungle",
            ColorSchemes::Aether => "Aether",
            ColorSchemes::Ignis => "Ignis",
            ColorSchemes::Bubblegum => "Bubblegum",
        }
    }
}

pub const JUNGLE_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/jungle_theme.json"
));
pub const AETHER_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/aether_theme.json"
));
pub const IGNIS_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/ignis_theme.json"
));
pub const BUBBLEGUM_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/bubblegum_theme.json"
));
