use crate::{
    // humanhash::human_hash,
    session::SessionId,
};
use ::actix::prelude::*;
// use ::orbiter_logic::SessionId;
use ::serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message for chat server communications
#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub struct TimeScaleMessage {
    pub time_scale: f64,
}

/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub session_id: SessionId,
    pub addr: Recipient<Message>,
}

/// Chat server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub enum Message {
    Text(String),
    Bin(Vec<u8>),
    StateWithDiff,
}

// const CHAT_HISTORY_MAX: usize = 100;
// const CHAT_LOG_FILE: &'static str = "chatlog.json";

/// `ChatServer` manages chat rooms and responsible for coordinating chat session.
///
/// Implementation is very naïve.
pub(crate) struct ChatServer {
    sessions: HashMap<SessionId, Recipient<Message>>,
    // chat_history: VecDeque<ClientMessage>,
}

impl ChatServer {
    pub fn new() -> ChatServer {
        // fn load_log() -> anyhow::Result<VecDeque<ClientMessage>> {
        //     #[derive(Deserialize)]
        //     #[serde(rename_all = "camelCase")]
        //     struct ClientMessageSerial {
        //         session_id: String,
        //         message: String,
        //     }
        //     let data = std::fs::read(CHAT_LOG_FILE)?;
        //     let data = String::from_utf8(data)?;
        //     let ret: Vec<ClientMessageSerial> = serde_json::from_str(&data)?;
        //     println!("Loaded {} chat items from log file", ret.len());
        //     Ok(ret
        //         .into_iter()
        //         .map(|val| ClientMessage {
        //             session_id: SessionId::from(val.session_id),
        //             message: val.message,
        //         })
        //         .collect())
        // }

        ChatServer {
            sessions: HashMap::new(),
            // chat_history: match load_log() {
            //     Ok(val) => val,
            //     Err(e) => {
            //         println!("Failed to load chat log: {:?}", e);
            //         VecDeque::new()
            //     }
            // },
        }
    }
}

impl ChatServer {
    /// Send message to all users
    fn send_message(&self, message: &str, skip_id: Option<SessionId>) {
        for (i, addr) in &self.sessions {
            if Some(*i) != skip_id {
                let _ = addr.do_send(Message::Text(message.to_owned()));
            }
        }
    }

    /// Send message to all users
    fn send_message_bin(&self, message: &[u8], skip_id: Option<SessionId>) {
        for (i, addr) in &self.sessions {
            if Some(*i) != skip_id {
                let _ = addr.do_send(Message::Bin(message.to_owned()));
            }
        }
    }

    fn send_message_with_diff(&self, skip_id: Option<SessionId>) {
        for (i, addr) in &self.sessions {
            if Some(*i) != skip_id {
                let _ = addr.do_send(Message::StateWithDiff);
            }
        }
    }

    fn cleanup(&mut self) {
        println!("Cleaning up sessions!");
        self.sessions.retain(|_, s| s.connected());
    }
}

/// Make actor from `ChatServer`
impl Actor for ChatServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.session_id, msg.addr);
        let res_msg = format!(
            "{{\"type\": \"joined\", \"sessionId\": \"{}\" }}",
            msg.session_id.to_string()
        );

        println!(
            "Session {} is connected, now we have {} sessions",
            msg.session_id,
            self.sessions.len()
        );
        println!("res_msg: {}", res_msg);
        println!("sending to {:?}", self.sessions);

        // notify all users in same room
        self.send_message(&res_msg, None);
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SetStateWs(pub String);

impl std::fmt::Debug for SetStateWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SetStateWs>")
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SetStateBinWs(pub Vec<u8>);

impl std::fmt::Debug for SetStateBinWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SetStateBinWs>")
    }
}

/// A message from server timer thread to the server actor.
#[derive(Deserialize, Serialize, Debug)]
pub(crate) enum NotifyStateEnum {
    SetState(SetStateWs),
    SetStateBin(SetStateBinWs),
    SetStateWithDiff,
    Cleanup,
}

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotifyState {
    pub session_id: Option<SessionId>,
    pub set_state: NotifyStateEnum,
}

#[derive(Serialize)]
struct Payload<T: Serialize> {
    #[serde(rename = "type")]
    type_: &'static str,
    payload: T,
}

impl Handler<NotifyState> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: NotifyState, _: &mut Context<Self>) {
        let session_id = msg.session_id;

        match msg.set_state {
            NotifyStateEnum::SetState(msg) => {
                let payload = Payload {
                    type_: "clientUpdate",
                    payload: msg,
                };

                self.send_message(&serde_json::to_string(&payload).unwrap(), session_id);
            }
            NotifyStateEnum::SetStateBin(msg) => self.send_message_bin(&msg.0, session_id),
            NotifyStateEnum::SetStateWithDiff => self.send_message_with_diff(session_id),
            NotifyStateEnum::Cleanup => self.cleanup(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotifyNewBody {
    pub session_id: SessionId,
    pub body: serde_json::Value,
    pub body_parent: String,
}

impl Handler<NotifyNewBody> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: NotifyNewBody, _: &mut Context<Self>) {
        let session_id = msg.session_id;

        let payload = Payload {
            type_: "newBody",
            payload: msg,
        };

        self.send_message(&serde_json::to_string(&payload).unwrap(), Some(session_id));
    }
}

// #[derive(Deserialize, Serialize, Debug)]
// #[serde(rename_all = "camelCase")]
// pub(crate) struct PlayerMessage {
//     pub player: String,
//     pub message: String,
// }

// impl From<ClientMessage> for PlayerMessage {
//     fn from(msg: ClientMessage) -> Self {
//         Self {
//             player: human_hash(&msg.session_id.0, 1, "-").unwrap_or_else(|_| "unknown".to_string()),
//             message: msg.message,
//         }
//     }
// }

// impl Handler<ClientMessage> for ChatServer {
//     type Result = ();

//     fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
//         if CHAT_HISTORY_MAX <= self.chat_history.len() {
//             self.chat_history.pop_front();
//         }
//         self.chat_history.push_back(msg.clone());
//         if let Ok(mut f) = std::fs::File::create(CHAT_LOG_FILE) {
//             if let Ok(s) = serde_json::to_string(&self.chat_history) {
//                 if let Err(e) = f.write_all(s.as_bytes()) {
//                     println!("Failed to save chat log: {:?}", e);
//                 }
//             }
//         }
//         let payload = Payload {
//             type_: "message",
//             payload: PlayerMessage::from(msg),
//         };

//         self.send_message(&serde_json::to_string(&payload).unwrap(), None);
//     }
// }

impl Handler<TimeScaleMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: TimeScaleMessage, _: &mut Context<Self>) {
        let payload = Payload {
            type_: "timeScale",
            payload: msg,
        };

        self.send_message(&serde_json::to_string(&payload).unwrap(), None);
    }
}

// impl Handler<ChatHistoryRequest> for ChatServer {
//     type Result = ();

//     fn handle(&mut self, msg: ChatHistoryRequest, _: &mut Context<Self>) {
//         println!(
//             "Handling ChatHistoryRequest returning {} items",
//             self.chat_history.len()
//         );
//         let session_id = msg.0;

//         if let Some(session) = self.sessions.get(&session_id) {
//             session.do_send(Message(
//                 serde_json::to_string(&Payload {
//                     type_: "chatHistory",
//                     payload: self
//                         .chat_history
//                         .iter()
//                         .map(|msg| PlayerMessage::from(msg.clone()))
//                         .collect::<Vec<_>>(),
//                 })
//                 .unwrap(),
//             ));
//         }
//     }
// }
