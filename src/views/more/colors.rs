use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::{COLOR_SCHEME_COLORS, COLOR_SCHEME_KEY, ColorScheme};

#[component]
pub fn Colors() -> Element {
    let on_click = |_| {
        set_color_scheme(ColorSchemes::Jungle);
    };
    let on_click2 = |_| {
        unset_color_scheme();
    };

    rsx! {
        div { id: "colors-view",
            h1 { "Colors" }
            p { "This is a placeholder for the colors settings page." }
            button {
                onclick: on_click,
                "Change Jungle"
            }
            button {
                onclick: on_click2,
                "Unset Color Scheme"
            }
        }
    }
}

fn set_color_scheme(scheme: ColorSchemes) {
    let colors = scheme.scheme();
    colors.apply();
    if let Err(e) = crate::storage::set(COLOR_SCHEME_KEY, scheme) {
        dioxus::logger::tracing::error!("Failed to set color scheme: {}", e);
    }
    if let Err(e) = crate::storage::set(COLOR_SCHEME_COLORS, colors) {
        dioxus::logger::tracing::error!("Failed to set color scheme colors: {}", e);
    }
}

fn unset_color_scheme() {
    jungle_theme().unset();
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
}

impl ColorSchemes {
    pub fn scheme(&self) -> ColorScheme {
        match self {
            ColorSchemes::Jungle => jungle_theme(),
        }
    }
}

pub fn jungle_theme() -> ColorScheme {
    ColorScheme {
        clr_black: "#000000".to_string(),
        clr_white: "#ffffff".to_string(),

        clr_error: "#ff0000".to_string(),
        clr_warning: "#e8a01b".to_string(),

        clr_primary: "oklch(0.7938 0.1422 87.06)".to_string(),
        clr_primary_light: "oklch(0.8438 0.1422 87.06)".to_string(),

        clr_background: "oklch(0.2049 0.0292 196.02)".to_string(),
        clr_surface: "oklch(0.3549 0.0292 196.02)".to_string(),
        clr_surface_light: "oklch(0.5049 0.0292 196.02)".to_string(),

        clr_board_dark: "oklch(0.6756 0.0292 196.02)".to_string(),
        clr_board_light: "oklch(0.7049 0.0292 196.02)".to_string(),

        clr_board_selected_dark: "oklch(0.7602 0.0292 196.02)".to_string(),
        clr_board_selected_light: "oklch(0.7702 0.0292 196.02)".to_string(),
        clr_board_highlight_dark: "oklch(0.6401 0.1147 91.34)".to_string(),
        clr_board_highlight_light: "oklch(0.6869 0.1213 91.34)".to_string(),
        clr_board_selected_highlight_dark: "oklch(0.7502 0.104 254.84)".to_string(),
        clr_board_selected_highlight_light: "oklch(0.7602 0.104 254.84)".to_string(),

        clr_piece_dark: "oklch(0.3533 0.0292 254.84)".to_string(),
        clr_piece_light: "oklch(0.9233 0.0292 254.84)".to_string(),
    }
}
