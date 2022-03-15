use censorbot::{State, media_update_handler, handle_start};
use tbot::types::parameters::UpdateKind;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let mut bot = tbot::from_env!("BOT_TOKEN")
        .stateful_event_loop(Mutex::new(State::default()));

    bot.command("start", handle_start);

    bot.unhandled(media_update_handler);

    bot.polling()
        .allowed_updates(&[UpdateKind::Message])
        .start()
        .await
        .unwrap();
}
