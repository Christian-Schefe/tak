use crate::components::TakBoardState;
use crate::tak::Player;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use web_sys::js_sys::Date;

#[component]
pub fn Clock() -> Element {
    let board = use_context::<TakBoardState>();
    let mut time_remaining_white = use_signal(|| None);
    let mut time_remaining_black = use_signal(|| None);
    to_owned![time_remaining_white, time_remaining_black];

    let update_task: Coroutine<()> = use_coroutine(move |rx| {
        let board_clone = board.clone();
        async move {
            loop {
                //tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                gloo::timers::future::sleep(std::time::Duration::from_millis(100)).await;
                time_remaining_white.set(Some(board_clone.get_time_remaining(Player::White)));
                time_remaining_black.set(Some(board_clone.get_time_remaining(Player::Black)));
            }
        }
    });

    rsx! {
        div {
            class: "clock-container",
            div {
                class: "clock clock-light",
                {if let Some(t) = time_remaining_white.read().as_ref() { format!("{:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60) } else { "00:00".to_string() }}
            }
            div {
                class: "clock clock-dark",
                {if let Some(t) = time_remaining_black.read().as_ref() { format!("{:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60) } else { "00:00".to_string() }}
            }
        }
    }
}

fn current_time() -> String {
    let now = Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    )
}
