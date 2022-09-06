use anyhow::Result;
use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

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

/// Browser compliant ICE candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ICECandidate {
    pub candidate: String,
    #[serde(default)]
    pub sdp_mid: String,
    #[serde(default)]
    pub sdp_mline_index: u16,
    #[serde(default)]
    pub username_fragment: String,
}

/// Either description or ICE candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Negotiation {
    /// Session Description
    SDP { description: RTCSessionDescription },
    /// ICE Candidate
    ICE { candidate: ICECandidate },
}

/// Packet sent from the client to the server
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PacketC2S {
    /// Connect to a given room
    Connect {
        // Room ID
        room_id: String,
        /// Authentication token
        token: String,
    },
    /// Give the server track IDs of the type of media we want to start producing
    RequestTrack {
        /// Request a new audio track
        audio: Option<String>,
        /// Request a new video track
        video: Option<String>,
        /// Request a new screenshare audio track
        screen_audio: Option<String>,
        /// Request a new screenshare video track
        screen_video: Option<String>,
    },
    /// Tell the server to send tracks
    Continue {
        /// IDs of tracks the client wants
        tracks: Vec<String>,
    },
    /// Tell the server certain tracks are no longer available
    Remove {
        /// IDs of tracks the client is no longer producing
        removed_tracks: Vec<String>,
    },
    /// Negotiation
    Negotiation(Negotiation),
}

/// Packet sent from the server to the client
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum PacketS2C {
    /// Accept connection to room
    Accept {
        /// Currently available tracks
        available_tracks: Vec<RemoteTrack>,
        /// Users currently in the room
        user_ids: Vec<String>,
    },
    /// Tell the client about a new available track
    Announce {
        /// Newly created remote track
        track: RemoteTrack,
    },
    /// Tell the client to send tracks
    Continue {
        /// IDs of tracks the server wants
        tracks: Vec<String>,
    },
    /// Tell the client certain tracks are no longer available
    Remove {
        /// IDs of tracks that are no longer being produced
        removed_tracks: Vec<String>,
    },
    /// Negotiation
    Negotiation(Negotiation),
    /// User joined the room
    UserJoin {
        /// ID of new user
        user_id: String,
    },
    /// User left the room
    UserLeft {
        /// ID of leaving user
        user_id: String,
    },
    /// Disconnection error
    Error { error: String },
}

/// An error occurred on the server
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("This room ID does not exist.")]
    RoomNotFound,
    #[error("This track ID does not exist.")]
    TrackNotFound,
    #[error("Something went wrong trying to authenticate you.")]
    FailedToAuthenticate,
    #[error("Already connected to a room!")]
    AlreadyConnected,
    #[error("Not connected to any room!")]
    NotConnected,
    #[error("Media type already has an existing track!")]
    MediaTypeSatisfied,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MediaType::Audio => write!(f, "Audio"),
            MediaType::Video => write!(f, "Video"),
            MediaType::ScreenAudio => write!(f, "ScreenAudio"),
            MediaType::ScreenVideo => write!(f, "ScreenVideo"),
        }
    }
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

impl From<RTCIceCandidateInit> for ICECandidate {
    fn from(candidate: RTCIceCandidateInit) -> Self {
        let RTCIceCandidateInit {
            candidate,
            sdp_mid,
            sdp_mline_index,
            username_fragment,
        } = candidate;

        Self {
            candidate,
            sdp_mid,
            sdp_mline_index,
            username_fragment,
        }
    }
}

impl From<ICECandidate> for RTCIceCandidateInit {
    fn from(candidate: ICECandidate) -> Self {
        let ICECandidate {
            candidate,
            sdp_mid,
            sdp_mline_index,
            username_fragment,
        } = candidate;

        Self {
            candidate,
            sdp_mid,
            sdp_mline_index,
            username_fragment,
        }
    }
}
