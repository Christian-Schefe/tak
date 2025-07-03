use crate::tak::TakPlayer;
use crate::views::TakBoardState;
use dioxus::core_macro::component;
use dioxus::prelude::*;

#[component]
pub fn Clock() -> Element {
    let board = use_context::<TakBoardState>();
    let time_remaining_white = use_signal_sync(|| None);
    let time_remaining_black = use_signal_sync(|| None);

    let _update_task: Coroutine<()> = use_coroutine(move |_| {
        let mut time_remaining_white = time_remaining_white.clone();
        let mut time_remaining_black = time_remaining_black.clone();
        let board_clone = board.clone();
        async move {
            loop {
                gloo::timers::future::sleep(std::time::Duration::from_millis(100)).await;
                time_remaining_white.set(Some(board_clone.get_time_remaining(TakPlayer::White)));
                time_remaining_black.set(Some(board_clone.get_time_remaining(TakPlayer::Black)));
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
