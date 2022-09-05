use std::{pin::Pin, sync::Arc};

use anyhow::Result;
use futures::{Future, StreamExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};

use super::{
    client::Client,
    packets::{PacketC2S, PacketS2C, ServerError},
    sender::{ReadWritePair, Sender},
};

/// User capabilities
#[derive(Default)]
pub struct UserCapabilities {
    pub audio: bool,
    pub video: bool,
    pub screenshare: bool,
}

/// User Information
pub struct UserInformation {
    pub id: String,
    pub capabilities: UserCapabilities,
}

/// Authentication function
type AuthFn = Box<
    dyn (Fn(
            String,
            String,
        ) -> Pin<Box<dyn Future<Output = Result<UserInformation>> + Send + 'static>>)
        + Send
        + Sync,
>;

/// Launch a new signaling server
pub async fn launch<A: ToSocketAddrs>(addr: A, auth: AuthFn) -> Result<()> {
    // Create TCP listener
    let try_socket = TcpListener::bind(addr).await;
    let listener = try_socket.expect("Failed to bind");

    // Accept new conncetions
    let auth = Arc::new(auth);
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, auth.clone()));
    }

    Ok(())
}

/// Accept a new TCP connection
async fn accept_connection(stream: TcpStream, auth: Arc<AuthFn>) {
    // Validate TCP connection
    stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    // Handshake WebSocket connection
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    // Prepare the connection for read / write
    let (write, read) = ws_stream.split();
    let write = Sender::new(write);

    // Handle any resulting errors
    if let Err(error) = handle_connection((read, write.clone()), auth).await {
        write
            .send(PacketS2C::Error {
                error: error.to_string(),
            })
            .await
            .ok();
    }
}

/// Wrap error handling around the connection and authenticate the client
async fn handle_connection((mut read, write): ReadWritePair, auth: Arc<AuthFn>) -> Result<()> {
    // Wait until valid packet is sent
    let mut client: Option<Client> = None;
    while let Some(msg) = read.next().await {
        if let Some(packet) = PacketC2S::from(msg?)? {
            if let PacketC2S::Connect { room_id, token } = packet {
                // Authenticate the client
                if let Ok(user) = (auth)(room_id.to_owned(), token).await {
                    info!("Authenticated user {} for room {room_id}", user.id);

                    // Create a new client
                    client = Some(Client::new(user, room_id));
                }
            }
        }
    }

    // Check if we are authenticated
    if let Some(client) = client {
        // Accept the new client
        client.run((read, write)).await
    } else {
        Err(ServerError::FailedToAuthenticate.into())
    }
}
