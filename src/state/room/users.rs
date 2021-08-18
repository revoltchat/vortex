use rand::prelude::*;
use std::collections::hash_map::Values;
use std::{ops::Deref, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard};

use super::{Room, RoomEvent, RoomUserMap};
use crate::api::ApiError;
use crate::state::user::User;

fn generate_token(rng: &mut dyn RngCore) -> Result<String, ApiError> {
    let mut token_bytes = [0; 24];
    rng.try_fill_bytes(&mut token_bytes)
        .map_err(|_| ApiError::InternalServerError)?;
    Ok(base64::encode_config(&token_bytes, base64::URL_SAFE))
}

pub struct RoomUsers {
    room: Arc<Room>,
}

impl<'r> RoomUsers {
    pub fn from_room(room: Arc<Room>) -> Self {
        RoomUsers { room }
    }

    pub async fn new(&'r self, id: String) -> Result<UserGuard<'r>, ApiError> {
        let token = {
            let registrations = self.room.registrations.read().await;
            let mut rng = thread_rng();
            let mut token = generate_token(&mut rng)?;
            while registrations.contains_key(&token) {
                token = generate_token(&mut rng)?;
            }
            token
        };

        let user = User::new(self.room.clone(), id.clone(), token.clone());
        let mut users = self.room.users.write().await;
        if users.contains_key(&id) {
            return Err(ApiError::UserAlreadyExists(id));
        }

        users.insert(id.clone(), RwLock::new(user));
        drop(users);

        let mut registrations = self.room.registrations.write().await;
        registrations.insert(token, id.clone());
        drop(registrations);

        debug!("Created new user {} in room {}", &id, self.room.id());
        Ok(self.get(&id).await.unwrap())
    }

    pub async fn get(&'r self, id: &str) -> Option<UserGuard<'r>> {
        let inner = self.room.users.read().await;
        if !inner.contains_key(id) {
            return None;
        }

        Some(UserGuard {
            inner,
            id: id.to_string(),
        })
    }

    pub async fn register(&'r self, token: &str) -> Option<UserGuard<'r>> {
        let mut registrations = self.room.registrations.write().await;
        let registration = registrations.remove(token)?;
        drop(registrations);

        let users = self.room.users.read().await;
        let user = users.get(&registration)?;
        let id = {
            let mut user = user.write().await;
            user.register().await;
            user.id().to_string()
        };

        Some(UserGuard { inner: users, id })
    }

    pub async fn remove(&'r self, id: &str) -> Result<(), ()> {
        let mut users = self.room.users.write().await;
        match users.remove(id) {
            Some(_) => {
                debug!("Removed user {} from room {}", id, self.room.id());
                self.room.send_event(RoomEvent::UserLeft(id.to_string()));
                Ok(())
            }
            None => Err(()),
        }
    }

    // This is dumb
    pub async fn guard(&'r self) -> UserMapGuard<'r> {
        let inner = self.room.users.read().await;
        UserMapGuard { inner }
    }
}

pub struct UserGuard<'r> {
    inner: RwLockReadGuard<'r, RoomUserMap>,
    id: String,
}

impl Deref for UserGuard<'_> {
    type Target = RwLock<User>;

    fn deref(&self) -> &Self::Target {
        self.inner
            .get(&self.id)
            .expect("UserGuard deref failed, this should never happen")
    }
}

pub struct UserMapGuard<'r> {
    inner: RwLockReadGuard<'r, RoomUserMap>,
}

impl<'r> UserMapGuard<'r> {
    pub fn iter(&'r self) -> Values<'r, String, RwLock<User>> {
        self.inner.values()
    }
}
