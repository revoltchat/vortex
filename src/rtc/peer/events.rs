use anyhow::Result;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use crate::signaling::packets::{MediaType, ServerError};

use super::Peer;

impl Peer {
    /// Register a new track that the client wants to provide
    pub fn register_track(&self, id: String, media_type: MediaType) -> Result<()> {
        if self.track_map.contains_key(&media_type) {
            return Err(ServerError::MediaTypeSatisfied.into());
        } else {
            self.track_map.insert(media_type, id);
            Ok(())
        }
    }

    /// Register event handlers
    pub async fn register_handlers(&self) {
        // Set handler for connection state
        self.connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                debug!("Peer connection state: {}", s);
                Box::pin(async {})
            }))
            .await;
    }
}
