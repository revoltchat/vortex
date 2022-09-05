use std::sync::Arc;

use dashmap::DashMap;
use postage::{
    broadcast::{channel, Receiver, Sender},
    sink::Sink,
};

/// Room event which indicates something happened to a peer
#[derive(Debug, Clone)]
pub enum RoomEvent {
    Test,
}

/// Room consisting of clients which can communicate with one another
#[derive(Debug)]
pub struct Room {
    sender: Sender<RoomEvent>,
}

lazy_static! {
    static ref ROOMS: DashMap<String, Arc<Room>> = DashMap::new();
}

impl Room {
    /// Create a new Room and initialise internal channels and maps
    fn new() -> Self {
        let (sender, _dropped) = channel(10);

        Room { sender }
    }

    /// Get or create a Room by its ID
    pub fn get(id: &str) -> Arc<Room> {
        if let Some(room) = ROOMS.get(id) {
            room.clone()
        } else {
            let room: Arc<Room> = Arc::new(Room::new());
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
}
