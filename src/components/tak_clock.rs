use crate::components::tak_board_state::TakBoardState;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use tak_core::TakPlayer;

#[component]
pub fn TakClock(player: TakPlayer) -> Element {
    let board = use_context::<TakBoardState>();
    let time_remaining = use_signal_sync(|| None);

    let _update_task: Coroutine<()> = use_coroutine(move |_| {
        let mut time_remaining = time_remaining.clone();
        let mut board_clone = board.clone();
        async move {
            loop {
                crate::future::sleep(std::time::Duration::from_millis(100)).await;
                let time = board_clone.get_time_remaining(player);
                time_remaining.set(time);
                if time.is_some_and(|x| x == 0) {
                    board_clone.check_ongoing_game();
                }
            }
        }
    });

    let class_name = match player {
        TakPlayer::White => "light",
        TakPlayer::Black => "dark",
    };

    let time_remaining_str = time_remaining
        .read()
        .as_ref()
        .map_or("-:--".to_string(), |&t| {
            if t >= 20000 {
                format!("{}:{:02}", (t / 1000) / 60, (t / 1000) % 60)
            } else {
                format!("0:{:02}.{}", t / 1000, (t / 100) % 10)
            }
        });

    rsx! {
        div { class: "clock clock-{class_name}",
            p { {time_remaining_str} }
        }
    }
}
