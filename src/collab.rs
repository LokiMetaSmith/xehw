use eframe::egui;
use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::agent::{Agent, Task};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CollabMessage {
    Hello { id: Uuid, name: String },
    Code { text: String },
    AgentUpdate { agents: Vec<Agent>, tasks: Vec<Task> },
}

pub struct CollabSystem {
    sender: Option<ewebsock::WsSender>,
    receiver: Option<ewebsock::WsReceiver>,
    pub status: String,
    pub peers: HashMap<Uuid, String>, // ID -> Name
}

impl Default for CollabSystem {
    fn default() -> Self {
        Self {
            sender: None,
            receiver: None,
            status: "Disconnected".to_string(),
            peers: HashMap::new(),
        }
    }
}

impl CollabSystem {
    pub fn connect(&mut self, url: &str, ctx: &egui::Context) {
        let ctx = ctx.clone();
        let wakeup = move || ctx.request_repaint();
        match ewebsock::connect_with_wakeup(url, ewebsock::Options::default(), wakeup) {
            Ok((sender, receiver)) => {
                self.sender = Some(sender);
                self.receiver = Some(receiver);
                self.status = "Connected".to_string();
            }
            Err(e) => {
                self.status = format!("Error: {}", e);
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.sender.is_some()
    }

    pub fn send(&mut self, msg: CollabMessage) {
        if let Some(sender) = &mut self.sender {
             if let Ok(json) = serde_json::to_string(&msg) {
                 sender.send(ewebsock::WsMessage::Text(json));
             }
        }
    }

    pub fn poll(&mut self) -> Vec<CollabMessage> {
        let mut messages = Vec::new();
        let mut disconnect = false;
        let mut error_msg = None;

        if let Some(receiver) = &mut self.receiver {
            while let Some(event) = receiver.try_recv() {
                match event {
                    ewebsock::WsEvent::Message(ewebsock::WsMessage::Text(text)) => {
                        if let Ok(msg) = serde_json::from_str::<CollabMessage>(&text) {
                            messages.push(msg);
                        } else {
                            log::warn!("Failed to parse collab message: {}", text);
                        }
                    }
                    ewebsock::WsEvent::Closed => {
                        disconnect = true;
                    }
                    ewebsock::WsEvent::Error(e) => {
                        error_msg = Some(e);
                    }
                    _ => {}
                }
            }
        }

        if disconnect {
            self.status = "Disconnected".to_string();
            self.sender = None;
            self.receiver = None;
        }
        if let Some(e) = error_msg {
            self.status = format!("Error: {}", e);
        }

        messages
    }
}
