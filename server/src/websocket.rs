use std::{collections::HashMap, time::Instant};

use crate::{
    server::ChatServer,
    server::{Connect, Message},
    session::SessionId,
    ServerData,
};
use ::actix::{prelude::*, Actor, StreamHandler};
use ::actix_web::{web, HttpRequest, HttpResponse};

use ::serde::{Deserialize, Serialize};
use actix_web_actors::ws;
use asteroid_colonies_logic::{
    construction::{Construction, ConstructionType},
    ItemType, Pos, Position,
};

/// Open a WebSocket instance and give it to the client.
/// `session_id` should be created by `/api/session` beforehand.
#[actix_web::get("/ws/{session_id}")]
pub(crate) async fn websocket_index(
    data: web::Data<ServerData>,
    session_id: web::Path<String>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let session_id: SessionId = session_id.into_inner().into();

    let session_ws = SessionWs {
        data: data.clone(),
        session_id,
        addr: data.srv.clone(),
        chunks_digest: HashMap::new(),
        last_updated: Instant::now(),
    };

    // let srv = data.srv.clone();
    // srv.do_send(Connect{addr: Addr(session_ws).recipient()});

    let resp = ws::start(session_ws, &req, stream);
    println!(
        "websocket connection established for session {}: {:?}",
        session_id, resp
    );
    resp
}

/// Define HTTP actor
struct SessionWs {
    pub data: web::Data<ServerData>,
    pub session_id: SessionId,
    pub addr: Addr<ChatServer>,
    pub chunks_digest: HashMap<Position, u64>,
    pub last_updated: Instant,
}

impl Actor for SessionWs {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        // self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(Connect {
                session_id: self.session_id,
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                match res {
                    Ok(_) => (),
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);

        // self.addr.do_send(ChatHistoryRequest(self.session_id));
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<Message> for SessionWs {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        match msg {
            Message::Text(txt) => ctx.text(txt),
            Message::Bin(bin) => ctx.binary(bin),
            Message::StateWithDiff => {
                let game = self.data.game.lock().unwrap();
                match game.serialize_with_diffs(&self.chunks_digest) {
                    Ok(bytes) => ctx.binary(bytes),
                    Err(e) => ctx.text(format!("Error: {e}")),
                }
            }
        };
        self.last_updated = Instant::now();
    }
}

#[derive(Deserialize, Serialize, Debug, Message, Clone)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientMessage {
    pub session_id: SessionId,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatHistoryRequest(pub SessionId);

type WsResult = Result<ws::Message, ws::ProtocolError>;

#[derive(Deserialize)]
#[serde(tag = "type", content = "payload")]
enum WsMessage {
    Excavate {
        x: i32,
        y: i32,
    },
    Move {
        from: Pos,
        to: Pos,
    },
    MoveItem {
        from: Pos,
        to: Pos,
        item: ItemType,
    },
    Build {
        pos: Pos,
        #[serde(rename = "type")]
        ty: ConstructionType,
    },
    BuildPlan {
        build_plan: Vec<Construction>,
    },
    CancelBuild {
        pos: Pos,
    },
    Deconstruct {
        pos: Pos,
    },
    DeconstructConveyor {
        pos: Pos,
    },
    DeconstructPowerGrid {
        pos: Pos,
    },
    SetRecipe {
        pos: Pos,
        name: Option<String>,
    },
    ChunksDigest {
        // The payload represents HashMap<Position, u64>, but we do not deserialize into JSON for
        // performance reasons.
        chunks_digest: String,
    },
    Cleanup {
        pos: Pos,
    },
}

impl StreamHandler<WsResult> for SessionWs {
    fn handle(&mut self, msg: WsResult, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                println!("client received ws text: {text}");
                let payload: WsMessage = if let Ok(payload) = serde_json::from_str(&text) {
                    payload
                } else {
                    return ctx.text("{\"type\": \"response\", \"payload\": \"fail\"}");
                };

                if let Err(e) = self.handle_message(payload) {
                    return ctx.text(&*format!(
                        "{{\"type\": \"response\", \"payload\": \"fail: {}\"}}",
                        e.to_string()
                    ));
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                if let Ok(chunks_digest) = bincode::deserialize(&bin) {
                    self.chunks_digest = chunks_digest;
                }
            }
            Ok(ws::Message::Close(_op)) => {
                // Once WebSocket session is closed, we drop this actor and forget about client state.
                // If the client is still lingering and can attempt reconnection, it may be a bit of waste,
                // but it should be pretty rare occasion, and we would need to implement cache retention
                // mechanism that expires after some amount of time. It's probably an optimization for later.
                ctx.stop();
            }
            _ => (),
        }
    }
}

fn map_err(s: &str) -> anyhow::Error {
    anyhow::anyhow!("{s}")
}

impl SessionWs {
    fn handle_message(&mut self, payload: WsMessage) -> anyhow::Result<()> {
        let mut game = self.data.game.lock().unwrap();

        match payload {
            WsMessage::Excavate { x, y } => {
                game.excavate(x, y).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            WsMessage::Move { from, to } => {
                game.move_building(from, to)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            WsMessage::MoveItem { from, to, item } => {
                game.move_item(from, to, item)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            WsMessage::Build { pos, ty } => match ty {
                ConstructionType::Building(ty) => {
                    game.build(pos[0], pos[1], ty)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                ConstructionType::PowerGrid => {
                    game.build_power_grid(pos[0], pos[1])
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                _ => return Err(anyhow::anyhow!("Invalid build type")),
            },
            WsMessage::BuildPlan { build_plan } => {
                game.build_plan(&build_plan);
            }
            WsMessage::CancelBuild { pos } => {
                game.cancel_build(pos[0], pos[1]);
            }
            WsMessage::Deconstruct { pos } => {
                game.deconstruct(pos[0], pos[1])
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            WsMessage::DeconstructConveyor { pos } => {
                game.deconstruct_conveyor(pos[0], pos[1]).map_err(map_err)?;
            }
            WsMessage::DeconstructPowerGrid { pos } => {
                game.deconstruct_power_grid(pos[0], pos[1])
                    .map_err(map_err)?;
            }
            WsMessage::SetRecipe { pos, name } => {
                game.set_recipe(pos[0], pos[1], name.as_ref().map(|s| s as &_))
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            WsMessage::ChunksDigest { chunks_digest } => {
                self.chunks_digest = serde_json::from_str(&chunks_digest)?;
                return Ok(());
            }
            WsMessage::Cleanup { pos } => {
                game.cleanup_item(pos).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
        }
        self.data.set_signal_push(true);

        // self.addr.do_send(NotifyBodyState {
        //     session_id: Some(self.session_id),
        //     body_state: payload,
        // });

        Ok(())
    }
}
