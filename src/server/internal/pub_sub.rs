use crate::{server::internal::matches::handle_player_publish, views::ClientGameMessage};

pub fn setup_handlers() {
    ws_pubsub::handle_subscribe_to_topic(
        "matches/+",
        |player_id, topic, msg: ClientGameMessage| async move {
            handle_player_publish(&player_id, topic, msg).await;
        },
    );
}
