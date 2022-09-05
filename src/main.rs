#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate lazy_static;

use anyhow::Result;

pub mod rtc;
pub mod signaling;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    signaling::server::launch(
        "0.0.0.0:3000",
        Box::new(move |room_id, token| {
            Box::pin(async move {
                use signaling::packets::ServerError;
                if room_id != "1" {
                    return Err(ServerError::RoomNotFound.into());
                }

                let id = token.to_string();

                use signaling::server::{UserCapabilities, UserInformation};
                Ok(UserInformation {
                    id,
                    capabilities: UserCapabilities {
                        audio: true,
                        video: true,
                        screenshare: true,
                    },
                })
            })
        }),
    )
    .await
}
