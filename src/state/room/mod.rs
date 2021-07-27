use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use mediasoup::router::{Router, RouterOptions};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    RwLock,
};

use super::user::{ProduceType, User};
use crate::{api::ApiError, rtc::get_worker_pool};

pub mod users;
pub use users::RoomUsers;

#[derive(Clone, Debug)]
pub enum RoomEvent {
    UserJoined(String),
    UserLeft(String),
    UserStartProduce(String, ProduceType),
    UserStopProduce(String, ProduceType),
    RoomDelete,
}

lazy_static! {
    pub static ref ROOMS: RwLock<HashMap<String, Arc<Room>>> = RwLock::new(HashMap::new());
}

pub type RoomUserMap = HashMap<String, RwLock<User>>;
pub type RoomRegistrationMap = HashMap<String, String>;

pub struct Room {
    id: String,
    closed: AtomicBool,
    router: Router,
    sender: Sender<RoomEvent>,

    users: RwLock<RoomUserMap>,
    pub(super) registrations: RwLock<RoomRegistrationMap>,
}

impl Room {
    pub async fn new(id: String /* video_allowed: bool */) -> Result<Arc<Self>, ApiError> {
        if ROOMS.read().await.contains_key(&id) {
            return Err(ApiError::RoomAlreadyExists(id));
        }

        let worker = get_worker_pool().get_worker();

        let mut options = RouterOptions::default();
        options.media_codecs.push(crate::rtc::create_opus_codec(2));
        let router = worker
            .create_router(options)
            .await
            .map_err(|_| ApiError::InternalServerError)?;

        let (sender, _) = broadcast::channel(32);
        info!("Created new room {}", id);
        let room = Arc::new(Room {
            id: id.clone(),
            closed: AtomicBool::new(false),
            router,
            sender,

            users: RwLock::new(HashMap::new()),
            registrations: RwLock::new(HashMap::new()),
        });

        ROOMS.write().await.insert(id, room.clone());

        Ok(room)
    }

    pub async fn get(id: &str) -> Option<Arc<Self>> {
        ROOMS.read().await.get(id).map(|arc| arc.clone())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn delete(&self) {
        let result =
            self.closed
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed);

        if result.is_ok() {
            info!("Deleting room {}", self.id);
            ROOMS.write().await.remove(&self.id);
            self.send_event(RoomEvent::RoomDelete);
        }
    }

    pub fn closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn send_event(&self, event: RoomEvent) {
        self.sender.send(event).ok();
    }

    pub fn subscribe(&self) -> Option<Receiver<RoomEvent>> {
        match self.closed() {
            false => Some(self.sender.subscribe()),
            true => None,
        }
    }

    pub fn router(&self) -> Option<&Router> {
        match self.closed() {
            false => Some(&self.router),
            true => None,
        }
    }

    pub fn users(self: &Arc<Room>) -> RoomUsers {
        RoomUsers::from_room(self.clone())
    }
}

impl Drop for Room {
    fn drop(&mut self) {
        debug!("Room {} dropped, mediasoup Router cleaned up", self.id);
    }
}
