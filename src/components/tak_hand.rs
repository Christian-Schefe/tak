use crate::tak::TakPlayer;
use crate::views::TakBoardState;
use dioxus::prelude::*;

#[component]
pub fn TakHand(player: TakPlayer) -> Element {
    let state = use_context::<TakBoardState>();
    rsx! {
        div {
            class: "tak-piece-hand",
            class: if *state.player.read() == player {
                "tak-piece-hand-current"
            } else {
                ""
            },
            {
                let stones = state.remaining_stones.read().get(&player).cloned().unwrap_or((0, 0));
                format!("{}/{}", stones.0, stones.1)
            }
        }
    }
}
