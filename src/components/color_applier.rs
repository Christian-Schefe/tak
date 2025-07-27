use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, window};

use crate::views::ColorSchemes;

#[component]
pub fn ColorApplier() -> Element {
    use_effect(|| {
        let scheme = match crate::storage::try_get::<ColorSchemes>(COLOR_SCHEME_KEY) {
            Ok(Some(scheme)) => scheme.scheme(),
            Ok(None) => return,
            Err(e) => {
                dioxus::logger::tracing::error!("[ColorApplier] Failed to get color scheme: {}", e);
                return;
            }
        };
        let Some(scheme) = scheme else {
            dioxus::logger::tracing::error!("[ColorApplier] No color scheme found");
            return;
        };
        scheme.apply();
        if let Err(e) = crate::storage::set(COLOR_SCHEME_COLORS, scheme.to_json_array()) {
            dioxus::logger::tracing::error!(
                "[ColorApplier] Failed to set color scheme colors: {}",
                e
            );
        }
        dioxus::logger::tracing::info!("[ColorApplier] Applied color scheme: {:?}", scheme);
    });
    rsx! {}
}

pub const COLOR_SCHEME_KEY: &str = "color_scheme";
pub const COLOR_SCHEME_COLORS: &str = "color_scheme_colors";

pub fn set_css_variable(name: &str, value: &str) {
    let window = window().expect("should have a window");
    let document = window.document().expect("should have a document");

    let root = document
        .document_element()
        .expect("should have a document element");

    let element = root.dyn_into::<HtmlElement>().unwrap();

    element
        .style()
        .set_property(name, value)
        .expect("failed to set CSS variable");
}

pub fn unset_css_variable(name: &str) {
    let window = window().expect("should have a window");
    let document = window.document().expect("should have a document");

    let root = document
        .document_element()
        .expect("should have a document element");

    let element = root.dyn_into::<HtmlElement>().unwrap();

    element
        .style()
        .remove_property(name)
        .expect("failed to unset CSS variable");
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ColorScheme {
    pub clr_black: String,
    pub clr_white: String,

    pub clr_text: String,

    pub clr_error: String,
    pub clr_warning: String,

    pub clr_primary: String,
    pub clr_primary_light: String,

    pub clr_background: String,
    pub clr_surface: String,
    pub clr_surface_light: String,

    pub clr_board_dark: String,
    pub clr_board_light: String,
    pub clr_board_highlight: String,

    pub clr_piece_dark: String,
    pub clr_piece_light: String,
}

impl ColorScheme {
    pub fn apply(&self) {
        for (name, value) in self.to_name_pairs() {
            set_css_variable(&format!("--{}", name), &value);
        }
    }

    pub fn unset(&self) {
        for (name, _) in self.to_name_pairs() {
            unset_css_variable(&format!("--{}", name));
        }
    }

    pub fn to_json_array(&self) -> String {
        format!(
            "[{}]",
            self.to_name_pairs()
                .iter()
                .map(|(name, value)| format!("[\"{}\",\"{}\"]", name, value))
                .collect::<Vec<String>>()
                .join(",")
        )
    }

    pub fn to_name_pairs(&self) -> Vec<(String, String)> {
        vec![
            ("clr-black".to_string(), self.clr_black.clone()),
            ("clr-white".to_string(), self.clr_white.clone()),
            ("clr-text".to_string(), self.clr_text.clone()),
            ("clr-error".to_string(), self.clr_error.clone()),
            ("clr-warning".to_string(), self.clr_warning.clone()),
            ("clr-primary".to_string(), self.clr_primary.clone()),
            (
                "clr-primary-light".to_string(),
                self.clr_primary_light.clone(),
            ),
            ("clr-background".to_string(), self.clr_background.clone()),
            ("clr-surface".to_string(), self.clr_surface.clone()),
            (
                "clr-surface-light".to_string(),
                self.clr_surface_light.clone(),
            ),
            ("clr-board-dark".to_string(), self.clr_board_dark.clone()),
            ("clr-board-light".to_string(), self.clr_board_light.clone()),
            (
                "clr-board-highlight".to_string(),
                self.clr_board_highlight.clone(),
            ),
            ("clr-piece-dark".to_string(), self.clr_piece_dark.clone()),
            ("clr-piece-light".to_string(), self.clr_piece_light.clone()),
        ]
    }
}
