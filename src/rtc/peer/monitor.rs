use std::sync::Arc;

use anyhow::Result;
use webrtc::{
    peer_connection::RTCPeerConnection, rtp_transceiver::rtp_sender::RTCRtpSender,
    track::track_local::track_local_static_rtp::TrackLocalStaticRTP,
};

/// Monitor a track local static and push it through to a peer connection
pub struct Monitor {
    rtp_sender: Arc<RTCRtpSender>,
}

impl Monitor {
    /// Construct a new Monitor
    pub async fn from(
        connection: &RTCPeerConnection,
        local_track: Arc<TrackLocalStaticRTP>,
    ) -> Result<Monitor> {
        let rtp_sender = connection.add_track(local_track).await?;

        // Read incoming RTCP packets
        // Before these packets are returned they are processed by interceptors. For things
        // like NACK this needs to be called.
        let sender = rtp_sender.clone();
        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = sender.read(&mut rtcp_buf).await {}
            // TODO: interrupt once kill
        });

        Ok(Monitor { rtp_sender })
    }

    /// Stop listening
    pub fn close(self) -> Arc<RTCRtpSender> {
        self.rtp_sender
    }
}
