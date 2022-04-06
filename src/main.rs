use censorbot::{State, media_update_handler, handle_start};
use tbot::types::parameters::AllowedUpdates;
use tokio::select;
use tokio::sync::{Mutex, mpsc};
use censorbot::db;

#[tokio::main]
async fn main() {

    let client = db::init_db().await.expect("failed to get Client");
    let (tx, mut rx) = mpsc::unbounded_channel::<usize>();
    
    let mut bot = tbot::from_env!("BOT_TOKEN")    
        .stateful_event_loop(Mutex::new(
            State::new(client, tx)
        ));

    bot.command("start", handle_start);

    bot.unhandled(media_update_handler);

    let p = bot
    .webhook(config::URL, config::PORT)
    .allowed_updates(AllowedUpdates::none().message(true));

    select! {
        _ = p.start() => {},
        _ = rx.recv() => {
            panic!("shutdown signal received")
        }
    }
}
