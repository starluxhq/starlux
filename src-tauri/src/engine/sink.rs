use std::sync::Mutex;

use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

use super::{providers, StreamEvent};
use crate::db::{self, Message};
use crate::windows::{QUICKBAR, WORKSPACE};

pub const STREAM_EVENT: &str = "starlux://stream";

/// Fans one run out to three places: the window that started it, the other
/// window so expanding mid-stream keeps the answer, and the database.
pub struct Sink {
    app: AppHandle,
    peer: &'static str,
    channel: Channel<StreamEvent>,
    conversation_id: String,
    model: Mutex<Option<String>>,
}

impl Sink {
    pub fn new(
        app: AppHandle,
        origin: String,
        channel: Channel<StreamEvent>,
        conversation_id: String,
        model: Option<String>,
    ) -> Self {
        Self {
            app,
            peer: if origin == QUICKBAR {
                WORKSPACE
            } else {
                QUICKBAR
            },
            channel,
            conversation_id,
            model: Mutex::new(model),
        }
    }

    pub fn conversation_id(&self) -> String {
        self.conversation_id.clone()
    }

    pub fn send(&self, event: StreamEvent) -> Result<(), String> {
        self.persist(&event);

        let _ = self.app.emit_to(self.peer, STREAM_EVENT, event.clone());

        self.channel.send(event).map_err(|err| err.to_string())
    }

    fn persist(&self, event: &StreamEvent) {
        match event {
            StreamEvent::Meta {
                session_id, model, ..
            } => {
                if let Some(model) = model {
                    *self.model.lock().unwrap() = Some(model.clone());
                }
                let Some(session_id) = session_id.clone() else {
                    return;
                };
                let id = self.conversation_id.clone();
                let model = self.model.lock().unwrap().clone();
                db::write(&self.app, move |db| {
                    db.set_session(&id, &session_id, model.as_deref())
                });
            }
            StreamEvent::End {
                run_id,
                text,
                usage,
                ..
            } => self.persist_answer(Message {
                id: run_id.clone(),
                role: "assistant".to_owned(),
                text: text.clone(),
                model: self.model.lock().unwrap().clone(),
                usage: usage.clone(),
                error: None,
                attachments: Vec::new(),
            }),
            StreamEvent::RateLimit { limit, .. } => {
                let limit = limit.clone();
                db::write(&self.app, move |db| db.set_rate_limit(&limit));
            }
            StreamEvent::Error {
                run_id, message, ..
            } => {
                // Whatever went wrong, who is signed in is one of the things it
                // could have been, and the answer is now a probe old.
                providers::invalidate();
                self.persist_answer(Message {
                    id: run_id.clone(),
                    role: "assistant".to_owned(),
                    text: String::new(),
                    model: self.model.lock().unwrap().clone(),
                    usage: None,
                    error: Some(message.clone()),
                    attachments: Vec::new(),
                })
            }
            _ => {}
        }
    }

    fn persist_answer(&self, message: Message) {
        let id = self.conversation_id.clone();
        db::write(&self.app, move |db| db.add_message(&id, &message));
    }
}
