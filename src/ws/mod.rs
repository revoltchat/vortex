use std::collections::HashMap;
use std::sync::Arc;

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use warp::ws::{Message, WebSocket, Ws};
use warp::{Filter, Rejection, Reply};

use crate::{
    rtc::RtcState,
    state::{
        room::{Room, RoomEvent},
        user::UserInfo,
    },
};

mod error;
mod types;

use error::{WSCloseType, WSError, WSErrorType};
use types::{WSCommand, WSCommandType, WSEvent, WSReply, WSReplyType};

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
    } else {
        ws_sink.send(Message::close()).await.ok();
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

                        ws_sink
                            .send(Message::text(serde_json::to_string(&reply)?))
                            .await?;
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

    // Transport initialization
    let rtc_state = loop {
        match ws_stream.next().await {
            Some(message) => {
                let message = message.map_err(|_| WSCloseType::ServerError)?;
                // Try to get the text message, ignore otherwise (might be ping, binary)
                if let Ok(text) = message.to_str() {
                    let out: WSCommand = serde_json::from_str(text)?;
                    if let WSCommandType::InitializeTransports { init_data } = out.command_type {
                        let router = room.router().ok_or(WSCloseType::RoomClosed)?;
                        let rtc_state = RtcState::initialize(router, init_data)
                            .await
                            .map_err(|_| WSCloseType::ServerError)?;
                        let reply_data = rtc_state.get_init_data();

                        let reply = WSReply {
                            id: out.id,
                            reply_type: WSReplyType::InitializeTransports { reply_data },
                        };

                        ws_sink
                            .send(Message::text(serde_json::to_string(&reply)?))
                            .await?;
                        break rtc_state;
                    } else {
                        return Err(WSCloseType::InvalidState);
                    }
                }
            }
            // Client disconnected before they authenticated, clean up
            None => {
                room.users().remove(&user_id).await.ok();
                return Ok(());
            }
        }
    };

    // TODO: implement some sort of way to automatically remove a user from a room if the thread panics
    // the Room user remove function is async but the Drop trait is not

    let result = event_loop(&room, &user_id, rtc_state, ws_sink, ws_stream).await;
    room.users().remove(&user_id).await.ok();
    result
}

async fn event_loop(
    room: &Arc<Room>,
    user_id: &str,
    mut rtc_state: RtcState,
    ws_sink: &mut SplitSink<WebSocket, Message>,
    ws_stream: &mut SplitStream<WebSocket>,
) -> Result<(), WSCloseType> {
    let mut room_stream = room.subscribe().ok_or(WSCloseType::RoomClosed)?;
    let mut ws_stream = ws_stream.fuse();

    loop {
        tokio::select! {
            message = ws_stream.next() => {
                if let Some(message) = message {
                    let message = message.map_err(|_| WSCloseType::ServerError)?;
                    // Try to get the text message, ignore otherwise (might be ping, binary)
                    if let Ok(text) = message.to_str() {
                        let out: WSCommand = serde_json::from_str(text)?;
                        match &out.command_type {
                            WSCommandType::ConnectTransport { connect_data } => {
                                let result = rtc_state.connect_transport(connect_data).await;
                                if let Ok(_) = result {
                                    let reply = WSReply {
                                        id: out.id,
                                        reply_type: WSReplyType::ConnectTransport,
                                    };

                                    ws_sink
                                        .send(Message::text(serde_json::to_string(&reply)?))
                                        .await?;
                                } else {
                                    let error = WSError::from_command(out, WSErrorType::TransportConnectionFailure);
                                    ws_sink
                                        .send(Message::text(serde_json::to_string(&error)?))
                                        .await?;
                                }
                            },
                            WSCommandType::RoomInfo => {
                                let users = room.users();
                                let guard = users.guard().await;
                                let mut user_info: HashMap<String, UserInfo> = HashMap::new();
                                for user in guard.iter() {
                                    let user = user.read().await;
                                    user_info.insert(user.id().to_string(), user.into_info());
                                }

                                let reply = WSReply {
                                    id: out.id,
                                    reply_type: WSReplyType::RoomInfo {
                                        id: room.id().to_string(),
                                        video_allowed: false,
                                        users: user_info,
                                    }
                                };

                                ws_sink
                                    .send(Message::text(serde_json::to_string(&reply)?))
                                    .await?;
                            },
                            WSCommandType::StartProduce { produce_type, rtp_parameters } => {
                                let users = room.users();
                                let user = users
                                    .get(user_id)
                                    .await
                                    .ok_or_else(|| WSCloseType::ServerError)?;
                                
                                let result = rtc_state.start_produce(produce_type, rtp_parameters.clone()).await;
                                match result {
                                    Ok(producer) => {
                                        let producer_id = producer.id();
                                        let mut mut_user = user.write().await;
                                        mut_user.set_producer(*produce_type, Some(producer)).ok();
                                        room.send_event(RoomEvent::UserStartProduce(user_id.to_string(), *produce_type));
                                        
                                        let reply = WSReply {
                                            id: out.id,
                                            reply_type: WSReplyType::StartProduce { producer_id }
                                        };
        
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&reply)?))
                                            .await?;
                                    },
                                    Err(err) => {
                                        error!("Error while trying to start produce for user {}: {:?}", user_id, err);
                                        let error = WSError::from_command(out, WSErrorType::ProducerFailure);
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&error)?))
                                            .await?;
                                    }
                                }
                            },
                            WSCommandType::StopProduce { produce_type } => {
                                let users = room.users();
                                let user = users
                                    .get(user_id)
                                    .await
                                    .ok_or_else(|| WSCloseType::ServerError)?;
                                
                                let mut mut_user = user.write().await;
                                match mut_user.get_producer(*produce_type) {
                                    Some(_) => {
                                        mut_user.set_producer(*produce_type, None).ok();
                                        room.send_event(RoomEvent::UserStopProduce(user_id.to_string(), *produce_type));
                                        let reply = WSReply {
                                            id: out.id,
                                            reply_type: WSReplyType::StopProduce,
                                        };
        
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&reply)?))
                                            .await?;
                                    },
                                    None => {
                                        let error = WSError::from_command(out, WSErrorType::ProducerNotFound);
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&error)?))
                                            .await?;
                                    }
                                }
                            },
                            WSCommandType::StartConsume { produce_type, user_id: producing_id } => {
                                let producing_id = producing_id.clone();
                                let users = room.users();
                                match users.get(&producing_id).await {
                                    Some(producing_user) => {
                                        let producing_user = producing_user.read().await;
                                        match producing_user.get_producer(*produce_type) {
                                            Some(producer) => {
                                                let router = room.router().ok_or_else(|| WSCloseType::ServerError)?;
                                                if router.can_consume(&producer.id(), rtc_state.rtp_capabilities()) {
                                                    match rtc_state.start_consume(producer.id()).await {
                                                        Ok(consumer) => {
                                                            let reply = WSReply {
                                                                id: out.id,
                                                                reply_type: WSReplyType::StartConsume {
                                                                    id: consumer.id(),
                                                                    producer_id: consumer.producer_id(),
                                                                    kind: consumer.kind(),
                                                                    rtp_parameters: consumer.rtp_parameters().clone(),
                                                                },
                                                            };
                            
                                                            ws_sink
                                                                .send(Message::text(serde_json::to_string(&reply)?))
                                                                .await?;
                                                        },
                                                        Err(err) => {
                                                            error!("Error while trying to start produce: {:?}", err);
                                                            let error = WSError::from_command(out, WSErrorType::ProducerFailure);
                                                            ws_sink
                                                                .send(Message::text(serde_json::to_string(&error)?))
                                                                .await?;
                                                        }
                                                    };
                                                } else {
                                                    let error = WSError::from_command(out, WSErrorType::ConsumerFailure);
                                                    ws_sink
                                                        .send(Message::text(serde_json::to_string(&error)?))
                                                        .await?;
                                                }
                                            },
                                            None => {
                                                let error = WSError::from_command(out, WSErrorType::ProducerNotFound);
                                                ws_sink
                                                    .send(Message::text(serde_json::to_string(&error)?))
                                                    .await?;
                                            }
                                        }
                                    },
                                    None => {
                                        let error = WSError::from_command(out, WSErrorType::UserNotFound(producing_id));
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&error)?))
                                            .await?;
                                    }
                                };
                            },
                            WSCommandType::StopConsume { id } => {
                                let id = id.clone();
                                match rtc_state.stop_consume(&id) {
                                    Ok(_) => {
                                        let reply = WSReply {
                                            id: out.id,
                                            reply_type: WSReplyType::StopConsume,
                                        };
        
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&reply)?))
                                            .await?;
                                    },
                                    Err(_) => {
                                        let error = WSError::from_command(out, WSErrorType::ConsumerNotFound(id.to_string()));
                                        ws_sink
                                            .send(Message::text(serde_json::to_string(&error)?))
                                            .await?;
                                    }
                                }
                            },
                            _ => return Err(WSCloseType::InvalidState),
                        };
                    }
                } else {
                    return Ok(());
                }
            },
            event = room_stream.recv() => {
                let event = event.map_err(|_| WSCloseType::ServerError)?;
                match event {
                    RoomEvent::UserJoined(id) => {
                        if id != user_id {
                            let event = WSEvent::UserJoined { id };
                            ws_sink
                                .send(Message::text(serde_json::to_string(&event)?))
                                .await?;
                        }
                    },
                    RoomEvent::UserLeft(id) => {
                        if id == user_id {
                            return Err(WSCloseType::Kicked);
                        }

                        let event = WSEvent::UserLeft { id };
                        ws_sink
                            .send(Message::text(serde_json::to_string(&event)?))
                            .await?;
                    },
                    RoomEvent::UserStartProduce(id, produce_type) => {
                        if id != user_id {
                            let event = WSEvent::UserStartProduce { id, produce_type };
                            ws_sink
                                .send(Message::text(serde_json::to_string(&event)?))
                                .await?;
                        }
                    },
                    RoomEvent::UserStopProduce(id, produce_type) => {
                        if id != user_id {
                            let event = WSEvent::UserStopProduce { id, produce_type };
                            ws_sink
                                .send(Message::text(serde_json::to_string(&event)?))
                                .await?;
                        }
                    }
                    RoomEvent::RoomDelete => {
                        return Err(WSCloseType::RoomClosed);
                    },
                }
            }
        }
    }
}
