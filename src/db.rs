use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use tokio_postgres::Config;
use tokio_postgres::config::SslMode;
pub use tokio_postgres::{Error, NoTls, Client};
use crate::State;

#[derive(Debug)]
pub struct Media {
    pub _id: String,
    pub _type: String,
    pub _caption: String
}

impl Media {
    pub fn new(_id: String, _type: String, _caption: String) -> Self {
        Self {
            _id,
            _type,
            _caption
        }
    }
}

#[derive(Debug)]
pub enum MediaKind {
    Single(Media),
    Group(Vec<Media>),
}

fn check_connection(guard: &MutexGuard<'_, State> ) {
    if guard.client.is_closed() {
        guard.sig_tx.send(0).unwrap();
    }
}

async fn connect() -> (Client, tokio_postgres::Connection<tokio_postgres::Socket, tokio_postgres::tls::NoTlsStream>) {
    
    let db_host = env::var("DB_HOST").expect("set host");
    let db_port= env::var("DB_PORT").expect("set port").parse::<u16>().unwrap();
    let db_pass = env::var("DB_PASS").expect("set pass");
    let db_user = env::var("DB_USER").expect("set user");
    let db_name = env::var("DB_NAME").expect("set name");


    Config::new()
    .host(&db_host)
    .port(db_port)
    .user(&db_user)
    .password(&db_pass)
    .dbname(&db_name)
    .ssl_mode(SslMode::Disable)
    .connect(NoTls).await.expect("failed to connect to db")
}

pub async fn init_db() -> Result<Client, Error> {
    let (client, connection) = connect().await;
    
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client.batch_execute("
        CREATE TABLE IF NOT EXISTS filedict (
            key varchar(14),
            fileid varchar not null,
            filetype varchar(10) not null,
            filecaption varchar
        )
    "
    ).await.expect("failed to create database");

    Ok(client)
}



pub async fn insert(state: Arc<Mutex<State>>, uid: &str, file_id: &str, file_type: &str, file_caption: &str) -> Result<u64, Error> {
    let guard = state.lock().await;
    check_connection(&guard);
    guard.client.execute("
        INSERT INTO filedict VALUES ($1, $2, $3, $4);
    ",
    &[&uid, &file_id, &file_type, &file_caption]
    ).await
}

pub async fn get(state: Arc<Mutex<State>>, uid: &str) -> Option<MediaKind> {
    let guard = state.lock().await;
    let client = &guard.client;
    check_connection(&guard);
    let r = client.query("
        SELECT * FROM filedict WHERE key = $1;
    ",
    &[&uid]
    ).await;
    
    let cursor = match r {
        Ok(v) => v,
        Err(e) => { 
            eprintln!("error during fetch {}", e);
            return None
        }
    };

    match cursor.len() {
        0 => { None }
        1 => { 
            let row = cursor.into_iter().next().unwrap();
            Some(MediaKind::Single(Media::new(row.get(1), row.get(2), row.get(3))))
        }
        _ => {
            let mut r = Vec::new();
            for row in cursor {
                r.push(
                    Media::new(row.get(1), row.get(2), row.get(3))
                )           
            }
            Some(MediaKind::Group(r))
        }
    } 
}