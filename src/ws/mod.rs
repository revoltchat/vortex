use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use warp::ws::{Message, WebSocket, Ws};
use warp::{Filter, Rejection, Reply};

use crate::state::room::Room;

mod error;
mod types;

use error::WSCloseType;
use types::{WSCommand, WSCommandType, WSReply, WSReplyType};

pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Copy {
    warp::ws::ws().map(|ws: Ws| ws.on_upgrade(on_connection))
}

async fn on_connection(ws: WebSocket) {
    let (mut ws_sink, mut ws_stream) = ws.split();
    let result = handle(&mut ws_sink, &mut ws_stream).await;
    if let Err(close) = result {
        let code = close as u16;
        let reason = close.to_string();
        ws_sink.send(Message::close_with(code, reason)).await.ok();
    }
}

async fn handle(
    ws_sink: &mut SplitSink<WebSocket, Message>,
    ws_stream: &mut SplitStream<WebSocket>,
) -> Result<(), WSCloseType> {
    // Authentication
    let (room, user_id) = loop {
        match ws_stream.next().await {
            Some(message) => {
                let message = message.map_err(|_| WSCloseType::ServerError)?;
                // Try to get the text message, ignore otherwise (might be ping, binary)
                if let Ok(text) = message.to_str() {
                    let out: WSCommand = serde_json::from_str(text)?;
                    if let WSCommandType::Authenticate { room_id, token } = out.command_type {
                        let room = Room::get(&room_id).await.ok_or(WSCloseType::Unauthorized)?;
                        let users = room.users();
                        // Attempt to register user
                        let user = users
                            .register(&token)
                            .await
                            .ok_or(WSCloseType::Unauthorized)?;
                        let id = user.read().await.id().to_string();

                        let reply = WSReply {
                            id: out.id,
                            reply_type: WSReplyType::Authenticate {
                                user_id: id.clone(),
                                room_id: room.id().to_string(),
                                rtp_capabilities: room
                                    .router()
                                    .ok_or(WSCloseType::RoomClosed)?
                                    .rtp_capabilities()
                                    .clone(),
                            },
                        };

                        ws_sink.send(Message::text(serde_json::to_string(&reply)?)).await?;
                        break (room, id);
                    } else {
                        return Err(WSCloseType::InvalidState);
                    }
                }
            }
            // Client disconnected before they authenticated, return
            None => return Ok(()),
        }
    };

    Ok(())
}
