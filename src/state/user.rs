use std::{str::FromStr, sync::Arc};

use tokio::sync::RwLock;
use mediasoup::producer::Producer;

use super::room::Room;

#[non_exhaustive]
pub enum ProduceType {
    Audio,
}

impl FromStr for ProduceType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s == "audio" => Ok(Self::Audio),
            _ => Err(()),
        }
    }
}

pub struct User {
    id: String,
    token: Option<String>,
    room: Arc<Room>,

    audio: Option<Producer>,
}

impl User {
    pub async fn new(room: Arc<Room>, id: String, token: String) -> Result<(), ()> {
        let user = User {
            id: id.clone(),
            token: Some(token.clone()),
            room: room.clone(),

            audio: None
        };

        let mut users = room.users.write().await;
        if users.contains_key(&id) { return Err(()); }
        users.insert(id.clone(), RwLock::new(user));
        drop(users);

        let mut registrations = room.registrations.write().await;
        registrations.insert(token, id).expect("Registration already exists");
        drop(registrations);

        Ok(())
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn token(&self) -> Option<&str> {
        self.token.as_ref().map(|string| string.as_str())
    }

    pub fn registered(&self) -> bool {
        self.token.is_none()
    }

    pub fn get_producer(&self, produce_type: ProduceType) -> Option<&Producer> {
        let producer = match produce_type {
            ProduceType::Audio => &self.audio,
        };

        producer.as_ref()
    }

    pub fn set_producer(&mut self, produce_type: ProduceType, new_producer: Producer) {
        let producer = match produce_type {
            ProduceType::Audio => &mut self.audio,
        };

        *producer = Some(new_producer);
    }
}
