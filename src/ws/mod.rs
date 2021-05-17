use futures::{SinkExt, StreamExt};

use warp::ws::{Message, WebSocket, Ws};
use warp::{Filter, Rejection, Reply};

mod error;
mod types;

pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Copy {
    warp::ws::ws().map(|ws: Ws| ws.on_upgrade(on_connection))
}

async fn on_connection(mut ws: WebSocket) {
    ws.send(Message::text("test")).await.unwrap();
}
