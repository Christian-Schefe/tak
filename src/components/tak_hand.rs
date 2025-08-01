use crate::components::tak_board_state::TakBoardState;
use dioxus::prelude::*;
use tak_core::TakPlayer;

#[component]
pub fn TakHand(player: TakPlayer) -> Element {
    let state = use_context::<TakBoardState>();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        state
            .with_game(|game| {
                let hand = &game.preview_game().hands[player.index()];
                (
                    game.get_visible_active_player() == player,
                    hand.stones,
                    hand.capstones,
                )
            })
            .expect("Game should exist to get hand data")
    });

    let (current_player, flats, caps) = data.read().clone();

    rsx! {
        div {
            class: "tak-piece-hand",
            class: if current_player { "tak-piece-hand-current" } else { "" },
            {format!("{}/{}", flats, caps)}
        }
    }
}
