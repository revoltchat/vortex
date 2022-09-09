use std::{sync::Arc, time::Duration};

use anyhow::Result;
use webrtc::{
    peer_connection::peer_connection_state::RTCPeerConnectionState,
    rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication,
    rtp_transceiver::rtp_receiver::RTCRtpReceiver,
    track::{
        track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter},
        track_remote::TrackRemote,
    },
    Error,
};

use crate::signaling::packets::{MediaType, Negotiation, ServerError};

use super::Peer;

impl Peer {
    /// Register a new track that the client wants to provide
    pub async fn register_track(&self, id: String, media_type: MediaType) -> Result<()> {
        let mut track_map = self.track_map.lock().await;
        if track_map.contains_key(&media_type) {
            return Err(ServerError::MediaTypeSatisfied.into());
        } else {
            track_map.insert(media_type, id);
            Ok(())
        }
    }

    /// Unregister an existing track in order to remove it
    pub async fn unregister_track(&self, id: &str) {
        let mut track_map = self.track_map.lock().await;
        track_map.retain(|_, item| item != id);
    }

    /// Register event handlers
    pub async fn register_handlers(&self) {
        let peer = self.clone();

        // Set handler for connection state
        self.connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                debug!("Peer connection state: {}", s);
                Box::pin(async {})
            }))
            .await;

        // Monitor negotiation state
        let peer_negotiation = peer.clone();
        self.connection
            .on_negotiation_needed(Box::new(move || {
                let peer_negotiation = peer_negotiation.clone();
                Box::pin(async move {
                    if let Err(error) = peer_negotiation.renegotiate().await {
                        error!("Failed to re-negotiate: {}", error.to_string());
                    }
                })
            }))
            .await;

        // Catch any new ICE candidates
        let peer_ice = peer.clone();
        self.connection
            .on_ice_candidate(Box::new(move |candidate| {
                let negotiation_fn = peer_ice.negotation_fn.clone();
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        if let Ok(candidate) = candidate.to_json().await {
                            (negotiation_fn)(Negotiation::ICE {
                                candidate: candidate.into(),
                            })
                            .await
                            .ok();
                        }
                    }
                })
            }))
            .await;

        // Set handler for new tracks
        self.connection
            .on_track(Box::new(
                move |track: Option<Arc<TrackRemote>>, _receiver: Option<Arc<RTCRtpReceiver>>| {
                    if let Some(track) = track {
                        // Spawn a new task for handling this track
                        let peer = peer.clone();
                        tokio::spawn(async move {
                            // Verify this is a track we are expecting to receive
                            let id = track.id().await;

                            // Find the media type
                            let mut media_type_buffer = peer.media_type_buffer.lock().await;
                            let item = if media_type_buffer.is_empty() {
                                None
                            } else {
                                Some(media_type_buffer.remove(0))
                            };

                            // Release Mutex lock
                            drop(media_type_buffer);

                            if let Some(media_type) =
                                item
                            {
                                // Write the media type
                                let mut track_map = peer.track_map.lock().await;
                                track_map.insert(media_type.clone(), id.to_owned());

                                // Release Mutex lock
                                drop(track_map);

                                if matches!(media_type, MediaType::Video | MediaType::ScreenVideo) {
                                    // Send a PLI on an interval so that the publisher is pushing a keyframe every rtcpPLIInterval
                                    // This is a temporary fix until we implement incoming RTCP events, then we would push a PLI only when a viewer requests it
                                    let media_ssrc = track.ssrc();
                                    tokio::spawn(async move {
                                        let mut result = Result::<usize>::Ok(0);
                                        while result.is_ok() {
                                            let timeout = tokio::time::sleep(Duration::from_secs(1));
                                            tokio::pin!(timeout);

                                            // TODO: need to kill this
                                            tokio::select! {
                                                _ = timeout.as_mut() => {
                                                    result = peer.connection.write_rtcp(&[Box::new(PictureLossIndication {
                                                        sender_ssrc: 0,
                                                        media_ssrc,
                                                    })])
                                                    .await
                                                    .map_err(Into::into);
                                                }
                                            };
                                        }
                                    });
                                }
                                
                                // Create track that we send video back through
                                let local_track = Arc::new(TrackLocalStaticRTP::new(
                                    track.codec().await.capability,
                                    id.to_owned(),
                                    format!("{}:{}:{id}", peer.user_id, media_type),
                                ));

                                // Send to other peers
                                peer.room.add_track(
                                    peer.user_id,
                                    media_type.clone(),
                                    local_track.clone(),
                                );

                                // Read and forward RTP packets
                                // TODO: kill track here
                                while let Ok((rtp, _)) = track.read_rtp().await {
                                    if let Err(err) = local_track.write_rtp(&rtp).await {
                                        if Error::ErrClosedPipe != err {
                                            print!(
                                                "output track write_rtp got error: {} and break",
                                                err
                                            );
                                            break;
                                        } else {
                                            print!("output track write_rtp got error: {}", err);
                                        }
                                    }
                                }
                            } else {
                                error!("Stream sent by {} has no media type for ID {}", peer.user_id, id);
                            }
                        });
                    }

                    Box::pin(async {})
                },
            ))
            .await;
    }
}
