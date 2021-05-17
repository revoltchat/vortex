use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use mediasoup::{
    data_structures::{DtlsParameters, IceCandidate, IceParameters, TransportProtocol},
    rtp_parameters::{MediaKind, RtpCapabilities, RtpParameters},
    sctp_parameters::SctpParameters,
    srtp_parameters::{SrtpCryptoSuite, SrtpParameters},
};

use crate::state::user::UserInfo;

#[derive(Serialize, Deserialize, Clone, Copy)]
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

#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum WSCommandType {
    Authenticate {
        room_id: String,
        token: String,
    },

    InitializeTransports {
        data: InitializeTransportsData,
    },
    ConnectTransport {
        data: ConnectTransportData,
    },

    RoomInfo,

    StartProduce {
        produce_type: ProduceType,
        rtp_parameters: RtpParameters,
    },
    StopProduce {
        produce_type: ProduceType,
    },

    StartConsume {
        produce_type: ProduceType,
        user_id: String,
    },
    StopConsume {
        /// Consumer ID
        id: String,
    },
    SetConsumerPause {
        /// Consumer ID
        id: String,
        paused: bool,
    },
}

#[derive(Deserialize)]
pub struct WSCommand {
    pub id: String,
    #[serde(flatten)]
    pub command_type: WSCommandType,
}

#[derive(Deserialize)]
pub struct InitializeTransportsData {
    pub id: String,
    #[serde(flatten)]
    pub variant: InitializeTransportsVariant,
}

#[derive(Deserialize)]
#[serde(tag = "mode")]
pub enum InitializeTransportsVariant {
    SplitWebRTC,
    CombinedWebRTC,
    CombinedRTP,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ConnectTransportData {
    WebRTC { dtls_parameters: DtlsParameters },
    RTP { srtp_parameters: SrtpParameters },
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum WSReplyType {
    Authenticate {
        user_id: String,
        room_id: String,
        rtp_capabilities: RtpCapabilities,
    },

    InitializeTransports {
        #[serde(flatten)]
        reply_data: InitializeTransportsReply,
    },
    ConnectTransport,

    RoomInfo {
        id: String,
        video_allowed: bool,
        users: Vec<UserInfo>,
    },

    StartProduce {
        producer_id: String
    },
    StopProduce,

    StartConsume {
        id: String,
        producer_id: String,
        kind: MediaKind,
        rtp_parameters: RtpParameters,
    },
    StopConsume,
    SetConsumerPause,
}

#[derive(Serialize)]
pub struct WSReply {
    pub id: String,
    #[serde(flatten)]
    pub reply_type: WSReplyType,
}

#[derive(Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum InitializeTransportsReply {
    SplitWebRTC {
        send_transport: WebRTCTransportInitData,
        recv_transport: WebRTCTransportInitData,
    },
    CombinedWebRTC {
        transport: WebRTCTransportInitData,
    },
    CombinedRTP {
        ip: IpAddr,
        port: u16,
        protocol: TransportProtocol,
        id: String,
        srtp_crypto_suite: SrtpCryptoSuite,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebRTCTransportInitData {
    id: String,
    ice_parameters: IceParameters,
    ice_candidates: Vec<IceCandidate>,
    dtls_arameters: DtlsParameters,
    sctp_parameters: SctpParameters,
}
