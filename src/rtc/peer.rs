use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use webrtc::peer_connection::RTCPeerConnection;

use crate::signaling::packets::MediaType;

use super::room::Room;

mod api;
mod events;
mod negotiation;

pub type PeerTrackMap = Arc<DashMap<MediaType, String>>;

/// Abstraction of a WebRTC peer connection
// #[derive(Clone)]
pub struct Peer {
    room: Arc<Room>,
    connection: Arc<RTCPeerConnection>,
    track_map: PeerTrackMap,
}

impl Peer {
    /// Create a new Peer
    pub async fn new(room: Arc<Room>) -> Result<Self> {
        // Create a new RTCPeerConnection
        let connection = api::create_peer_connection().await?;

        // Construct new Peer
        let peer = Self {
            connection,
            room,
            track_map: Default::default(),
        };

        // Register event handlers
        peer.register_handlers().await;

        Ok(peer)
    }

    /// Clean up any open connections
    pub async fn clean_up(&self) -> Result<()> {
        self.connection.close().await.map_err(Into::into)
    }
}
