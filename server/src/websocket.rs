use crate::{
    server::ChatServer,
    server::{Connect, Message, TimeScaleMessage},
    session::SessionId,
    ServerData,
};
use ::actix::{prelude::*, Actor, StreamHandler};
use ::actix_web::{web, HttpRequest, HttpResponse};
use ::asteroid_colonies_logic::{
    // CelestialBody, SessionId, SetRocketStateWs,
    AsteroidColoniesGame as Game,
    // WsMessage
};
use ::serde::{Deserialize, Serialize};
use actix_web_actors::ws;
use asteroid_colonies_logic::SerializeGame;

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
    };

    // let srv = data.srv.clone();
    // srv.do_send(Connect{addr: Addr(session_ws).recipient()});

    let resp = ws::start(session_ws, &req, stream);
    println!(
        "websocket received for session {:?}: {:?}",
        session_id, resp
    );
    resp
}

/// Define HTTP actor
struct SessionWs {
    pub data: web::Data<ServerData>,
    pub session_id: SessionId,
    pub addr: Addr<ChatServer>,
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
        ctx.text(msg.0);
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SetStateWs(pub String);

impl std::fmt::Debug for SetStateWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SetStateWs>")
    }
}

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotifyState {
    pub session_id: Option<SessionId>,
    pub set_state: SetStateWs,
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

impl StreamHandler<WsResult> for SessionWs {
    fn handle(&mut self, msg: WsResult, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // let payload: WsMessage = if let Ok(payload) = serde_json::from_str(&text) {
                //     payload
                // } else {
                //     return ctx.text("{\"type\": \"response\", \"payload\": \"fail\"}");
                // };

                // match payload {
                //     WsMessage::SetRocketState(payload) => {
                //         if let Err(e) = self.handle_set_rocket_state(payload) {
                //             return ctx.text(&*format!(
                //                 "{{\"type\": \"response\", \"payload\": \"fail: {}\"}}",
                //                 e.to_string()
                //             ));
                //         }
                //     }
                //     WsMessage::Message { payload } => {
                //         println!("Got message: {:?}", payload);
                //         self.addr.do_send(ClientMessage {
                //             session_id: self.session_id,
                //             message: payload,
                //         });
                //     }
                //     WsMessage::TimeScale { payload } => {
                //         let mut data = self.data.universe.write().unwrap();
                //         println!("Got timeScale: {}", payload.time_scale);
                //         data.time_scale = payload.time_scale;
                //         self.addr.do_send(TimeScaleMessage {
                //             time_scale: payload.time_scale,
                //         });
                //     }
                //     WsMessage::ChatHistoryRequest => {
                //         self.addr.do_send(ChatHistoryRequest(self.session_id));
                //     }
                // }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

impl SessionWs {
    fn handle_set_rocket_state(&mut self, payload: SetStateWs) -> anyhow::Result<()> {
        let mut game = self.data.game.write().unwrap();

        // self.addr.do_send(NotifyBodyState {
        //     session_id: Some(self.session_id),
        //     body_state: payload,
        // });

        Ok(())
    }
}
