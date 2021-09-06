use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

use mediasoup::producer::Producer;
use mediasoup::rtp_parameters::MediaKind;

use super::room::{Room, RoomEvent};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ProduceType {
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "video")]
    Video,

    #[serde(rename = "saudio")]
    #[serde(alias = "screenshareaudio")]
    ScreenshareAudio,
    #[serde(rename = "svideo")]
    #[serde(alias = "screensharevideo")]
    ScreenshareVideo,
}

impl ProduceType {
    pub fn into_kind(self) -> MediaKind {
        match self {
            ProduceType::Audio | ProduceType::ScreenshareAudio => MediaKind::Audio,
            ProduceType::Video | ProduceType::ScreenshareVideo => MediaKind::Video,
        }
    }
}

impl From<ProduceType> for MediaKind {
    fn from(produce_type: ProduceType) -> MediaKind {
        produce_type.into_kind()
    }
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
    pub(super) fn new(room: Arc<Room>, id: String, token: String) -> User {
        User {
            id: id,
            token: Some(token.clone()),
            room: room,

            audio: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_ref().map(|string| string.as_str())
    }

    pub fn registered(&self) -> bool {
        self.token.is_none()
    }

    pub async fn register(&mut self) {
        if let Some(token) = self.token.take() {
            let mut registrations = self.room.registrations.write().await;
            registrations.remove(&token);
            debug!("User {} registered", &self.id);
            self.room.send_event(RoomEvent::UserJoined(self.id.clone()));
        }
    }

    pub fn get_producer(&self, produce_type: ProduceType) -> Option<&Producer> {
        let producer = match produce_type {
            ProduceType::Audio => &self.audio,
            _ => todo!(),
        };

        producer.as_ref()
    }

    pub fn set_producer(
        &mut self,
        produce_type: ProduceType,
        new_producer: Option<Producer>,
    ) -> Result<(), ()> {
        if !self.registered() {
            return Err(());
        }
        let producer = match produce_type {
            ProduceType::Audio => &mut self.audio,
            _ => todo!(),
        };

        *producer = new_producer;
        Ok(())
    }

    pub fn into_info(&self) -> UserInfo {
        UserInfo::from(self)
    }
}

/// Structure passed to clients connected over WebSocket
#[derive(Serialize)]
pub struct UserInfo {
    audio: bool,
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> UserInfo {
        UserInfo {
            audio: user.audio.is_some(),
        }
    }
}
