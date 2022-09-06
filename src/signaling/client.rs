use std::sync::Arc;

use anyhow::Result;
use futures::{
    future::{select, Either},
    pin_mut, FutureExt, TryStreamExt,
};
use postage::stream::Stream;

use crate::rtc::{
    peer::Peer,
    room::{Room, RoomEvent},
};

use super::{
    packets::{MediaType, Negotiation, PacketC2S, PacketS2C, ServerError},
    sender::{ReadWritePair, Sender},
    server::UserInformation,
};

/// Information about user, room and peer connection
pub struct Client {
    user: UserInformation,
    room: Arc<Room>,
    peer: Option<Peer>,
}

impl Client {
    /// Create a new Client for a user in a room
    pub fn new(user: UserInformation, room_id: String) -> Self {
        let room = Room::get(&room_id);
        info!("Created a new client for {user:?} in room {room_id}.");

        Self {
            user,
            room,
            peer: None,
        }
    }

    /// Run client lifecycle
    pub async fn run(mut self, stream: ReadWritePair) -> Result<()> {
        // Initialise the peer
        let sender = stream.1.clone();
        self.peer = Some(
            Peer::new(
                self.user.id.to_owned(),
                self.room.clone(),
                Box::new(move |negotiation| {
                    let sender = sender.clone();
                    Box::pin(async move { sender.send(PacketS2C::Negotiation(negotiation)).await })
                }),
            )
            .await?,
        );

        // Start working
        let result = self.lifecycle_listen(stream).await;

        // Clean up after ourselves
        self.lifecycle_clean_up().await?;

        // Return work result
        result
    }

    /// Listen for incoming packets
    pub async fn lifecycle_listen(&mut self, stream: ReadWritePair) -> Result<()> {
        // Deconstruct read / write pair
        let (mut read, write) = stream;

        // Let the client know what is currently available
        debug!("Announcing current state to client");
        write
            .send(PacketS2C::Accept {
                available_tracks: self.room.get_available_tracks(),
            })
            .await?;

        debug!("Now accepting incoming messages and room events");

        // Create a worker task for reading WS messages
        let ws_worker = async {
            // Read incoming messages
            while let Some(msg) = read.try_next().await? {
                if let Some(msg) = PacketC2S::from(msg)? {
                    self.handle_message(msg, &write).await?;
                }
            }

            Ok(())
        }
        .fuse();

        // Create a worker task for reading room events
        let event_worker = async {
            let mut listener = self.room.listener();

            // Read incoming events
            while let Some(event) = listener.recv().await {
                match event {
                    RoomEvent::CreateTrack(track) => {
                        if track.user_id == self.user.id {
                            continue;
                        }

                        write.send(PacketS2C::Announce { track }).await?;
                    }
                    RoomEvent::RemoveTrack { removed_tracks } => {
                        for track in &removed_tracks {
                            self.peer.as_ref().unwrap().remove_track(track).await?;
                        }

                        write.send(PacketS2C::Remove { removed_tracks }).await?;
                    }
                }
            }

            // TODO: maybe throw an error for listener being closed?
            Ok(())
        }
        .fuse();

        // Pin futures on the stack
        pin_mut!(ws_worker, event_worker);

        // Wait for either one to complete
        match select(ws_worker, event_worker).await {
            Either::Left((result, _)) => result,
            Either::Right((result, _)) => result,
        }
    }

    /// Clean up after ourselves by disconnecting from the room,
    /// closing the peer connection and removing tracks.
    pub async fn lifecycle_clean_up(&mut self) -> Result<()> {
        info!("User {} disconnected", self.user.id);
        self.room.remove_user(&self.user.id);
        self.peer.as_ref().unwrap().clean_up().await
    }

    /// Handle incoming packet
    async fn handle_message(&self, packet: PacketC2S, write: &Sender) -> Result<()> {
        let peer = self.peer.as_ref().unwrap();

        match packet {
            PacketC2S::Connect { .. } => Err(ServerError::AlreadyConnected.into()),
            PacketC2S::RequestTrack {
                audio,
                video,
                screen_audio,
                screen_video,
            } => {
                // Keep track of all the IDs being registered
                let mut tracks = vec![];

                // Register audio track
                if let Some(id) = audio {
                    peer.register_track(id.to_owned(), MediaType::Audio)?;
                    tracks.push(id);
                }

                // Register video track
                if let Some(id) = video {
                    peer.register_track(id.to_owned(), MediaType::Video)?;
                    tracks.push(id);
                }

                // Register screenshare audio track
                if let Some(id) = screen_audio {
                    peer.register_track(id.to_owned(), MediaType::ScreenAudio)?;
                    tracks.push(id);
                }

                // Register screenshare video track
                if let Some(id) = screen_video {
                    peer.register_track(id.to_owned(), MediaType::ScreenVideo)?;
                    tracks.push(id);
                }

                // Confirm said IDs back to the user
                write.send(PacketS2C::Continue { tracks }).await
            }
            PacketC2S::Continue { tracks } => {
                for id in tracks {
                    peer.add_track(id).await?;
                }

                Ok(())
            }
            PacketC2S::Remove { removed_tracks } => {
                todo!()
            }
            PacketC2S::Negotiation(negotiation) => {
                match negotiation {
                    Negotiation::SDP { description } => {
                        peer.consume_sdp(description).await?;
                    }
                    Negotiation::ICE { candidate } => {
                        peer.consume_ice(candidate.into()).await?;
                    }
                }

                Ok(())
            }
        }
    }
}
