use std::sync::Arc;

use dashmap::DashMap;
use postage::{
    broadcast::{channel, Receiver, Sender},
    sink::Sink,
};

use crate::signaling::packets::RemoteTrack;

use super::peer::PeerTrackMap;

/// Room event which indicates something happened to a peer
#[derive(Debug, Clone)]
pub enum RoomEvent {
    Test,
}

/// Room consisting of clients which can communicate with one another
#[derive(Debug)]
pub struct Room {
    #[allow(dead_code)]
    id: String,
    sender: Sender<RoomEvent>,
    user_tracks: DashMap<String, PeerTrackMap>,
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
        self.sender.clone().try_send(event).ok();
    }

    /// Listen for events from the room
    pub fn listener(&self) -> Receiver<RoomEvent> {
        self.sender.subscribe()
    }

    /// Get all currently availabe tracks which can be consumed
    pub fn get_available_tracks(&self) -> Vec<RemoteTrack> {
        vec![]
    }

    /// Check if a user is in a room
    pub fn in_room(&self, id: &str) -> bool {
        self.user_tracks.contains_key(id)
    }

    /// Join a new user into the room
    pub fn join_user(&self, id: String, track_map: PeerTrackMap) {
        self.user_tracks.insert(id, track_map);
    }

    /// Remove a user from the room
    pub fn remove_user(&self, id: &str) {
        self.user_tracks.remove(id);
        // TODO: announce removal of tracks and remove from tracks once added
        // TODO: if room is empty, clean up room
    }
}
