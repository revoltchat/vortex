use std::sync::Arc;

use dashmap::DashMap;
use postage::{
    broadcast::{channel, Receiver, Sender},
    sink::Sink,
};
use webrtc::track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocal};

use crate::signaling::packets::{MediaType, RemoteTrack};

use super::peer::PeerTrackMap;

/// Room event which indicates something happened to a peer
#[derive(Debug, Clone)]
pub enum RoomEvent {
    CreateTrack(RemoteTrack),
    RemoveTrack { removed_tracks: Vec<String> },
    UserJoin { user_id: String },
    UserLeft { user_id: String },
}

/// Room consisting of clients which can communicate with one another
#[derive(Debug)]
pub struct Room {
    #[allow(dead_code)]
    id: String,
    sender: Sender<RoomEvent>,
    user_tracks: DashMap<String, PeerTrackMap>,
    tracks: DashMap<String, Arc<TrackLocalStaticRTP>>,
}

lazy_static! {
    static ref ROOMS: DashMap<String, Arc<Room>> = DashMap::new();
}

impl Room {
    /// Create a new Room and initialise internal channels and maps
    fn new(id: String) -> Self {
        let (sender, _dropped) = channel(10);

        Room {
            id,
            sender,
            user_tracks: Default::default(),
            tracks: Default::default(),
        }
    }

    /// Get or create a Room by its ID
    pub fn get(id: &str) -> Arc<Room> {
        if let Some(room) = ROOMS.get(id) {
            room.clone()
        } else {
            let room: Arc<Room> = Arc::new(Room::new(id.to_string()));
            ROOMS.insert(id.to_string(), room.clone());

            room
        }
    }

    /// Publish an event to the room
    pub fn publish(&self, event: RoomEvent) {
        info!("Room {} emitted {:?}", self.id, event);
        self.sender.clone().try_send(event).ok();
    }

    /// Listen for events from the room
    pub fn listener(&self) -> Receiver<RoomEvent> {
        self.sender.subscribe()
    }

    /// Get all currently availabe tracks which can be consumed
    pub async fn get_available_tracks(&self) -> Vec<RemoteTrack> {
        let mut tracks = vec![];

        for item in &self.user_tracks {
            let user_id = item.key();
            let track_map = item.value().lock().await;

            tracks.extend(track_map.iter().map(|(media_type, id)| RemoteTrack {
                id: id.to_owned(),
                media_type: media_type.clone(),
                user_id: user_id.to_owned(),
            }));
        }

        tracks
    }

    /// Get all user IDs currently in the room
    pub fn get_user_ids(&self) -> Vec<String> {
        self.user_tracks
            .iter()
            .map(|item| item.key().to_owned())
            .collect()
    }

    /// Check if a user is in a room
    pub fn in_room(&self, id: &str) -> bool {
        self.user_tracks.contains_key(id)
    }

    /// Join a new user into the room
    pub fn join_user(&self, id: String, track_map: PeerTrackMap) {
        // Let everyone know we joined
        self.publish(RoomEvent::UserJoin {
            user_id: id.to_owned(),
        });

        // Add tracks to map
        self.user_tracks.insert(id, track_map);
    }

    /// Remove a user from the room
    pub async fn remove_user(&self, id: &str) {
        // Find all associated track information
        if let Some((_, tracks)) = self.user_tracks.remove(id) {
            let tracks = tracks.lock().await;
            let removed_tracks = tracks
                .iter()
                .map(|(_, id)| id.to_owned())
                .collect::<Vec<String>>();

            // Release Mutex lock
            drop(tracks);

            for id in &removed_tracks {
                self.close_track(id);
            }

            self.publish(RoomEvent::RemoveTrack { removed_tracks });
        }

        // Let everyone know we left
        self.publish(RoomEvent::UserLeft {
            user_id: id.to_owned(),
        });

        // TODO: if room is empty, clean up room
    }

    /// Add a local track
    pub fn add_track(
        &self,
        user_id: String,
        media_type: MediaType,
        local_track: Arc<TrackLocalStaticRTP>,
    ) {
        let id = local_track.id().to_owned();
        info!("{user_id} started broadcasting track with ID {id} to all users");

        self.tracks.insert(id.to_owned(), local_track);
        self.publish(RoomEvent::CreateTrack(RemoteTrack {
            id,
            user_id,
            media_type,
        }));
    }

    /// Get a local track
    pub fn get_track(&self, id: &str) -> Option<Arc<TrackLocalStaticRTP>> {
        self.tracks.get(id).map(|value| value.clone())
    }

    /// Remove a local track
    pub fn remove_track(&self, id: String) {
        self.close_track(&id);

        self.publish(RoomEvent::RemoveTrack {
            removed_tracks: vec![id],
        });
    }

    /// Close local track
    fn close_track(&self, id: &str) {
        info!("Track {id} has been removed");
        self.tracks.remove(id);

        // TODO: stop the RTP sender thread and drop
    }
}
