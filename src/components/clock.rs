use crate::tak::TakPlayer;
use crate::views::TakBoardState;
use dioxus::core_macro::component;
use dioxus::prelude::*;

#[component]
pub fn Clock(player: TakPlayer) -> Element {
    let board = use_context::<TakBoardState>();
    let time_remaining = use_signal_sync(|| None);

    let _update_task: Coroutine<()> = use_coroutine(move |_| {
        let mut time_remaining = time_remaining.clone();
        let board_clone = board.clone();
        async move {
            loop {
                gloo::timers::future::sleep(std::time::Duration::from_millis(100)).await;
                time_remaining.set(Some(board_clone.get_time_remaining(player)));
            }
        }
    });

    let class_name = match player {
        TakPlayer::White => "light",
        TakPlayer::Black => "dark",
    };

    rsx! {
        div {
            class: "clock clock-{class_name}",
            p {
                {if let Some(t) = time_remaining.read().as_ref() { format!("{:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60) } else { "00:00".to_string() }}
            }
        }
    }
}
