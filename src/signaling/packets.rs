use anyhow::Result;
use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

/// Available types of media tracks
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize)]
pub enum MediaType {
    /// Audio stream
    Audio,
    /// Video stream
    Video,
    /// Screenshare audio stream
    ScreenAudio,
    /// Screenshare video stream
    ScreenVideo,
}

/// Representation of an available track on the server
#[derive(Debug, Clone, Serialize)]
pub struct RemoteTrack {
    /// ID of the track
    pub id: String,
    /// User ID of whoever owns the track
    pub user_id: String,
    /// Type of media this track provides
    pub media_type: MediaType,
}

/// Packet sent from the client to the server
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PacketC2S {
    /// Connect to a given room
    Connect { room_id: String, token: String },
    /// Give the server track IDs of the type of media we want to start producing
    RequestTrack {
        audio: Option<String>,
        video: Option<String>,
        screen_audio: Option<String>,
        screen_video: Option<String>,
    },
    /// Tell the server to send tracks
    Continue { tracks: Vec<String> },
    /// Tell the server certain tracks are no longer available
    Remove { removed_tracks: Vec<String> },
    /// Negotiation
    Negotiation { sdp: Option<RTCSessionDescription> },
}

/// Packet sent from the server to the client
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum PacketS2C {
    /// Accept connection to room
    Accept {
        /// Currently available tracks
        available_tracks: Vec<RemoteTrack>,
    },
    /// Tell the client about a new available track
    Announce { track: RemoteTrack },
    /// Tell the client to send tracks
    Continue { tracks: Vec<String> },
    /// Tell the client certain tracks are no longer available
    Remove { removed_tracks: Vec<String> },
    /// Negotiation
    Negotiation { sdp: Option<RTCSessionDescription> },
    /// Disconnection error
    Error { error: String },
}

/// An error occurred on the server
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("This room ID does not exist.")]
    RoomNotFound,
    #[error("Something went wrong trying to authenticate you.")]
    FailedToAuthenticate,
    #[error("Already connected to a room!")]
    AlreadyConnected,
    #[error("Not connected to any room!")]
    NotConnected,
    #[error("Media type already has an existing track!")]
    MediaTypeSatisfied,
}

impl PacketC2S {
    /// Create a packet from incoming Message
    pub fn from(message: Message) -> Result<Option<Self>> {
        Ok(if let Message::Text(text) = message {
            Some(serde_json::from_str(&text)?)
        } else {
            None
        })
    }
}
