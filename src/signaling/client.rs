use std::sync::Arc;

use anyhow::Result;
use futures::TryStreamExt;

use crate::rtc::{peer::Peer, room::Room};

use super::{
    packets::{PacketC2S, PacketS2C, ServerError},
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
        Self {
            user,
            room,
            peer: None,
        }
    }

    /// Run client lifecycle
    pub async fn run(mut self, stream: ReadWritePair) -> Result<()> {
        // Initialise the peer
        self.peer = Some(Peer::new(self.room.clone()).await?);

        // Start working
        let result = self.lifecycle_listen(stream).await;

        // Clean up after ourselves
        self.lifecycle_clean_up().await?;

        // Return work result
        result
    }

    pub async fn lifecycle_listen(&mut self, stream: ReadWritePair) -> Result<()> {
        // Deconstruct read / write pair
        let (mut read, write) = stream;

        // Let the client know what is currently available
        write
            .send(PacketS2C::Accept {
                available_tracks: self.room.get_available_tracks(),
            })
            .await?;

        // Read incoming messages
        while let Some(msg) = read.try_next().await? {
            if let Some(msg) = PacketC2S::from(msg)? {
                self.handle_message(msg, &write).await?;
            }
        }

        Ok(())
    }

    /// Clean up after ourselves by disconnecting from the room,
    /// closing the peer connection and removing tracks.
    pub async fn lifecycle_clean_up(&mut self) -> Result<()> {
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
                let mut tracks = vec![];

                write.send(PacketS2C::Continue { tracks }).await
            }
            PacketC2S::Continue { tracks } => todo!(),
            PacketC2S::Remove { removed_tracks } => todo!(),
            PacketC2S::Negotiation { sdp } => todo!(),
        }
    }
}
