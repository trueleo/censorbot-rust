use std::net::{IpAddr, Ipv6Addr};
use std::str::FromStr;

use censorbot::{State, media_update_handler, handle_start};
use tbot::types::parameters::AllowedUpdates;
use tokio::select;
use tokio::sync::{Mutex, mpsc};
use censorbot::db;
use censorbot::config::{URL, PORT, HOST_IP};

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
    .bind_to(IpAddr::V6(Ipv6Addr::from_str(HOST_IP).expect("unable to parse HOST_IP")))
    .http();

    select! {
        err = p.start() =>  { eprintln!("{:?}", err) },
        _ = rx.recv() => {
            panic!("shutdown signal received")
        }
    }
}
