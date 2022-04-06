use std::net::{IpAddr, Ipv4Addr};

use censorbot::{State, media_update_handler, handle_start};
use tbot::types::parameters::AllowedUpdates;
use tokio::select;
use tokio::sync::{Mutex, mpsc};
use censorbot::db;
use censorbot::config::{URL, PORT};

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
    .webhook(URL, PORT.parse::<u16>().unwrap())
    .allowed_updates(AllowedUpdates::none().message(true))
    .ip_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
    .http();

    select! {
        _ = p.start() => {},
        _ = rx.recv() => {
            panic!("shutdown signal received")
        }
    }
}
