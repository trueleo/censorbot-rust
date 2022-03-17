pub mod config;
pub mod db;

use tbot::Bot;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use tbot::markup::{bold, link, markdown_v2};
use tbot::types::{chat, input_file, update, message};
use tbot::contexts::{methods::Message, Command, Unhandled};
use db::{Client, MediaKind};
use tokio::sync::mpsc::UnboundedSender;


#[derive(Debug)]
pub struct State {
    group_uuid: HashMap<String, String>,
    client: Client,
    sig_tx: UnboundedSender<usize>
}

impl State {
    pub fn new(client: Client, sig_tx: UnboundedSender<usize> ) -> Self {
        State {
            group_uuid: HashMap::default(),
            client,
            sig_tx
        }
    }
}


pub async fn media_update_handler(context: Arc<Unhandled>, state: Arc<Mutex<State>>) {
      
    use update::Kind::Message;
    let (chat_id, message) = match &context.update {
        Message(m) => (m.chat.id, m),
        _ => unreachable!(),
    };
    
    use message::Kind::*;
    let (file_id, file_type, file_caption, media_group_id) = match &message.kind {
        Photo {
            photo,
            caption,
            media_group_id,
        } => (
            &photo[0].file_id,
            "photo",
            Some(caption),
            media_group_id.as_ref(),
        ),

        Audio { audio, caption, .. } => (&audio.file_id, "audio", Some(caption), None),

        Sticker(sticker) => (&sticker.file_id, "sticker", None, None),

        Video {
            video,
            caption,
            media_group_id,
        } => (
            &video.file_id,
            "video",
            Some(caption),
            media_group_id.as_ref(),
        ),

        Voice { voice, caption } => (&voice.file_id, "voice", Some(caption), None),

        VideoNote(videonote) => (&videonote.file_id, "videonote", None, None),

        Animation { animation, caption } => (&animation.file_id, "animation", Some(caption), None),

        _ => {
            context
                .bot
                .send_message(chat_id, config::NOT_SUPPORTED_TYPE)
                .call()
                .await
                .unwrap();
            return;
        }
    };

    let uid = if let Some(group_id) = media_group_id {
        let mut guard = state.lock().await;
        let uid = guard.group_uuid.get(group_id).and_then(|s| Some(s.clone()));

        match uid {
            Some(uid) => {
                drop(guard);
                uid
            }
            None => {
                let deeplink_caption = format!("Censored Album\n\n");
                let uid = gen_uid();
                guard.group_uuid.insert(group_id.clone(), uid.clone());
                drop(guard);
                send_deeplinks(&context.bot, chat_id, uid.clone(), deeplink_caption).await;
                uid
            }
        }
    } else {
        let deeplink_caption = format!("Censored {0}\n\n", uppercase_first_letter(file_type));
        let uid = gen_uid();
        send_deeplinks(&context.bot, chat_id, uid.clone(), deeplink_caption).await;
        uid
    };

    if let Err(e) = db::insert(
        Arc::clone(&state),
        &uid,
        &file_id.0,
        file_type,
        file_caption.map_or("", |t| &t.value as &str),
    ).await
    {
        eprintln!("insert error: {}", e);
    }
}


pub async fn handle_start(context: Arc<Command>, state: Arc<Mutex<State>>) {
    use input_file::{Photo, PhotoOrVideo, Video};

    let key = &context.text.value;

    if key.is_empty() {
        context
            .send_message(config::START_MESSAGE)
            .call()
            .await
            .unwrap();
        return;
    }

    let media = db::get(Arc::clone(&state), key).await;

    if let Some(m) = media {
        match m {
            MediaKind::Single(v) => match (&v._id as &str, &v._type as &str, &v._caption as &str) {
                (_id, "photo" , caption) => {
                    let file = input_file::Photo::with_id(_id.into()).caption(caption);
                    context.send_photo(file).call().await.unwrap();
                }
                (_id, "audio", caption) => {
                    let file = input_file::Audio::with_id(_id.into()).caption(caption);
                    context.send_audio(file).call().await.unwrap();
                }
                (_id, "sticker", _) => {
                    let file = input_file::Sticker::with_id(_id.into());
                    context.send_sticker(file).call().await.unwrap();
                }
                (_id, "video", caption) => {
                    let file = input_file::Video::with_id(_id.into()).caption(caption);
                    context.send_video(file).call().await.unwrap();
                }
                (_id, "voice", caption) => {
                    let file = input_file::Voice::with_id(_id.into()).caption(caption);
                    context.send_voice(file).call().await.unwrap();
                }
                (_id, "videonote", _) => {
                    let file = input_file::VideoNote::with_id(_id.into());
                    context.send_video_note(file).call().await.unwrap();
                }
                (_id, "animation", caption) => {
                    let file = input_file::Animation::with_id(_id.into()).caption(caption);
                    context.send_animation(file).call().await.unwrap();
                }
                _ => unreachable!(),
            },

            MediaKind::Group(m) => {
                let reply_group_media: Vec<input_file::PhotoOrVideo> = m
                    .iter()
                    .map(|m| match (&m._id as &str, &m._type as &str, &m._caption as &str) {
                        (_id, "photo", caption) => PhotoOrVideo::Photo(Photo::with_id(_id.into()).caption(caption)),
                        (_id, "video", caption) => PhotoOrVideo::Video(Video::with_id(_id.into()).caption(caption)),
                        _ => unreachable!(),
                    }).collect();
                
                context
                    .send_media_group(reply_group_media)
                    .call()
                    .await
                    .unwrap();
            }
        }
    } else {
        context
            .send_message(config::NOT_FOUND_MESSAGE)
            .call()
            .await
            .unwrap();
    }
}


fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}


fn gen_uid() -> String {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;

    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(13)
        .map(char::from)
        .collect();

    rand_string

}


pub async fn send_deeplinks(bot: &Bot, chat_id: chat::Id, uuid: String, reply_text: String) {
    let link = link(
        ">> view",
        format!("t.me/{}/?start={}", config::BOTNAME, uuid),
    );
    let text = markdown_v2((reply_text, bold(link)));
    bot.send_message(chat_id, text).call().await.unwrap();
}
