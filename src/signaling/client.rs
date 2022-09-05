use anyhow::Result;

use super::{sender::ReadWritePair, server::UserInformation};

pub struct Client {
    user: UserInformation,
    room_id: String,
    // room: Arc<Mutex<Arc<Room>>>,
    // peer: Option<crate::peer::Peer>,
    // write: Sender,
}

impl Client {
    pub fn new(user: UserInformation, room_id: String) -> Self {
        Self { user, room_id }
    }

    pub async fn run(self, stream: ReadWritePair) -> Result<()> {
        // at this point we are in the room

        Ok(())
    }
}
