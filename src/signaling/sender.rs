use std::sync::Arc;

use anyhow::Result;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use super::packets::PacketS2C;

type Sink = SplitSink<WebSocketStream<TcpStream>, Message>;

/// Sink side of the WebSocket stream behind a Mutex for distributed writing
#[derive(Clone)]
pub struct Sender {
    write: Arc<Mutex<Sink>>,
}

impl Sender {
    /// Create a new Sender
    pub fn new(sink: Sink) -> Self {
        Sender {
            write: Arc::new(Mutex::new(sink)),
        }
    }

    /// Send a packet through the WebSocket
    pub async fn send(&self, packet: PacketS2C) -> Result<()> {
        self.write
            .lock()
            .await
            .send(Message::Text(serde_json::to_string(&packet)?))
            .await?;

        Ok(())
    }
}

/// Pair of sink and stream
pub type ReadWritePair = (SplitStream<WebSocketStream<TcpStream>>, Sender);
