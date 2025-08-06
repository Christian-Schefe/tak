use dioxus::prelude::*;
use ws_pubsub::use_ws_topic_receive_dynamic;

use crate::{
    Route,
    server::{
        NOTIFICATION_TOPIC, SEEK_ACCEPTED_SUBTOPIC,
        api::{MyServerFunctions, get_user_id},
    },
};

#[component]
pub fn Notifications() -> Element {
    let player_id = use_resource(|| get_user_id());

    let mut is_seek_accepted = use_signal(|| None);

    use_ws_topic_receive_dynamic::<_, MyServerFunctions, _>(
        move || match player_id.read().as_ref() {
            Some(Ok(Ok(x))) => Some(format!(
                "{}/{}/{}",
                NOTIFICATION_TOPIC, x, SEEK_ACCEPTED_SUBTOPIC
            )),
            _ => None,
        },
        move |match_id: String| async move {
            is_seek_accepted.set(Some(match_id));
        },
    );

    rsx! {
        if let Some(match_id) = is_seek_accepted.read().as_ref().cloned() {
            div { class: "notification",
                "Seek accepted! You can now play with your opponent."
                button {
                    onclick: move |_| {
                        is_seek_accepted.set(None);
                        let nav = use_navigator();
                        nav.push(Route::PlayOnline { match_id: match_id.clone() });
                    },
                    "Join"
                }
            }
        }
    }
}
