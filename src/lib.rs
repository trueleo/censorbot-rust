use std::collections::HashMap;
use std::sync::Arc;
use tbot::Bot;
use tbot::contexts::methods::ChatMethods;
use tbot::contexts::{Command, Text, Unhandled};
use tbot::types::chat;
use tbot::types::input_file;
use tiny_id::ShortCodeGenerator;
use tokio::sync::Mutex;

pub const BOTNAME: &str = "viewchanbot";

#[derive(Debug, Default)]
pub struct Media {
    media_id: String,
    media_type: &'static str,
}

impl Media {
    pub fn new(media_id: String, media_type: &'static str) -> Self {
        Media {
            media_id,
            media_type,
        }
    }
}

#[derive(Debug)]
pub enum MediaKind {
    Single(Media),
    Group(Vec<Media>),
}

#[derive(Debug, Default)]
pub struct State {
    uuid_media: HashMap<String, MediaKind>,
    group_uuid: HashMap<String, String>,
}

pub async fn send_deeplinks(
    bot: &Bot,
    botname: &str,
    chat_id: chat::Id,
    uuid: &str,
    reply_text: &str,
) {
    let text = format!("{}\n\nt.me/{}/?start={}", reply_text, botname, uuid);
    bot.send_message(chat_id, &text).call().await.unwrap();
}

pub async fn media_update_handler(context: Arc<Unhandled>, state: Arc<Mutex<State>>) {
    use tbot::types::message;
    use tbot::types::update;
 
    let (chat_id, message) = match &context.update {
        update::Kind::Message(m) => {
            (m.chat.id, m)
        }
        _ => unreachable!()
    };

    use message::Kind::*;
    let (media_id, media_type, media_group_id) = match &message.kind {
        Photo(v, _, media_group_id) => (&v[0].file_id, "photo", media_group_id),
        Audio(a, _) => (&a.file_id, "audio", &None),
        Sticker(s) => (&s.file_id, "sticker", &None),
        Video(v, _, media_group_id) => (&v.file_id, "video", media_group_id),
        Voice(v, _) => ( &v.file_id, "voice", &None),
        VideoNote(v) => (&v.file_id, "videonote", &None),
        Animation(a, _) => (&a.file_id, "animation", &None),
        _ => {
            context
            .bot
            .send_message(
                chat_id,
                "Send any supported media like photo(s), video(s), sticker ..etc",
            )
            .call()
            .await
            .unwrap();
            return;
        }
    };

    let _caption = format!("Censored {0}", media_type);

    if let Some(group_id) = media_group_id {
        let uid = state
            .lock()
            .await
            .group_uuid
            .get(group_id)
            .and_then(|s| Some(s.clone()));

        if let Some(uid) = uid {
            match state.lock().await.uuid_media.get_mut(&uid) {
                Some(MediaKind::Group(v)) => v.push(Media::new(media_id.0.clone(), media_type)),
                _ => unreachable!(),
            }
        } else {
            let mut generator = ShortCodeGenerator::new_lowercase_alphanumeric(8);
            let uid = generator.next_string();
            let mut guard = state.lock().await;
            guard.group_uuid.insert(group_id.clone(), uid.clone());
            guard.uuid_media.insert(
                uid.clone(),
                MediaKind::Group(vec![Media::new(media_id.0.clone(), media_type)]),
            );
            send_deeplinks(&context.bot, BOTNAME, chat_id, &uid, "hmm").await;
        }
    } else {
        let mut generator = ShortCodeGenerator::new_lowercase_alphanumeric(8);
        let uid = generator.next_string();

        send_deeplinks(&context.bot, BOTNAME, chat_id, &uid, "hmm").await;

        state.lock().await.uuid_media.insert(
            uid,
            MediaKind::Single(Media::new(media_id.0.clone(), media_type)),
        );
    }
}

pub async fn handle_start(context: Arc<Command<Text>>, state: Arc<Mutex<State>>) {
    let key = &context.text.value;
    let guard = state.lock().await;
    let media = guard.uuid_media.get(key);
    if let Some(m) = media {
        match m {
            MediaKind::Single(v) => match v {
                Media {
                    media_id,
                    media_type: "photo",
                } => {
                    let file = input_file::Photo::with_id(media_id.as_str().into());
                    context.send_photo(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "audio",
                } => {
                    let file = input_file::Audio::with_id(media_id.as_str().into());
                    context.send_audio(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "sticker",
                } => {
                    let file = input_file::Sticker::with_id(media_id.as_str().into());
                    context.send_sticker(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "video",
                } => {
                    let file = input_file::Video::with_id(media_id.as_str().into());
                    context.send_video(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "voice",
                } => {
                    let file = input_file::Voice::with_id(media_id.as_str().into());
                    context.send_voice(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "videonote",
                } => {
                    let file = input_file::VideoNote::with_id(media_id.as_str().into());
                    context.send_video_note(file).call().await.unwrap();
                }
                Media {
                    media_id,
                    media_type: "animation",
                } => {
                    let file = input_file::Animation::with_id(media_id.as_str().into());
                    context.send_animation(file).call().await.unwrap();
                }
                _ => unreachable!(),
            },
            MediaKind::Group(m) => {
                let reply_group_media: Vec<input_file::GroupMedia> = m
                    .iter()
                    .map(|m| match m {
                        Media {
                            media_id,
                            media_type: "photo",
                        } => input_file::GroupMedia::Photo(input_file::Photo::with_id(
                            media_id.as_str().into(),
                        )),

                        Media {
                            media_id,
                            media_type: "video",
                        } => input_file::GroupMedia::Video(input_file::Video::with_id(
                            media_id.as_str().into(),
                        )),
                        _ => unreachable!(),
                    })
                    .collect();
                context
                    .send_media_group(&reply_group_media)
                    .call()
                    .await
                    .unwrap();
            }
        }
    } else {
        context
            .send_message("No media found")
            .call()
            .await
            .unwrap();
    }
}

// async fn media_predicate(context: Arc<Unhandled>, _state: Arc<Mutex<State>>) -> bool {
//     use tbot::types::update::Kind::Message;
//     use tbot::
//     if let Message(ref m) = context.update {
//         use message::Kind;
//         match m.kind {
//             Kind::Photo(..)
//             | Kind::Video(..)
//             | Kind::Animation(..)
//             | Kind::Sticker(..)
//             | Kind::Audio(..)
//             | Kind::Voice(..) => true,

//             _ => false,
//         }
//     } else {
//         false
//     }
// }
