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
    let nav = use_navigator();

    use_ws_topic_receive_dynamic::<_, MyServerFunctions, _>(
        move || match player_id.read().as_ref() {
            Some(Ok(Ok(x))) => Some(format!(
                "{}/{}/{}",
                NOTIFICATION_TOPIC, x, SEEK_ACCEPTED_SUBTOPIC
            )),
            _ => None,
        },
        move |match_id: String| async move {
            nav.push(Route::PlayOnline {
                match_id: match_id.clone(),
            });
        },
    );

    rsx! {}
}
