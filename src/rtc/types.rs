use std::net::IpAddr;
use serde::{Serialize, Deserialize};

use mediasoup::prelude::*;
use mediasoup::sctp_parameters::SctpParameters;
use mediasoup::srtp_parameters::SrtpParameters;
use mediasoup::data_structures::TransportProtocol;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationInput {
    pub(super) rtp_capabilities: RtpCapabilities,
    #[serde(flatten)]
    pub(super) mode: InitializationInputMode,
}

#[derive(Serialize, Deserialize)]
#[serde(tag="mode")]
pub enum InitializationInputMode {
    SplitWebRtc,
    CombinedWebRtc,
    CombinedRtp,
}

#[derive(Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum TransportInitData {
    #[serde(rename_all = "camelCase")]
    SplitWebRTC {
        send_transport: WebRTCTransportInitData,
        recv_transport: WebRTCTransportInitData,
    },
    CombinedWebRTC {
        transport: WebRTCTransportInitData,
    },
    #[serde(rename_all = "camelCase")]
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

#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ConnectTransportData {
    #[serde(rename_all = "camelCase")]
    WebRTC { dtls_parameters: DtlsParameters },
    #[serde(rename_all = "camelCase")]
    RTP { srtp_parameters: SrtpParameters },
}
