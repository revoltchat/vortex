use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use mediasoup::data_structures::TransportProtocol;
use mediasoup::prelude::*;
use mediasoup::sctp_parameters::SctpParameters;
use mediasoup::srtp_parameters::SrtpParameters;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationInput {
    pub(super) rtp_capabilities: RtpCapabilities,
    #[serde(flatten)]
    pub(super) mode: InitializationInputMode,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum InitializationInputMode {
    #[serde(rename = "SplitWebRTC")]
    SplitWebRtc,
    #[serde(rename = "CombinedWebRTC")]
    CombinedWebRtc,
    #[serde(rename = "CombinedRTP")]
    CombinedRtp,
}

#[derive(Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum TransportInitData {
    #[serde(rename_all = "camelCase")]
    SplitWebRtc {
        send_transport: WebRtcTransportInitData,
        recv_transport: WebRtcTransportInitData,
    },
    CombinedWebRtc {
        transport: WebRtcTransportInitData,
    },
    #[serde(rename_all = "camelCase")]
    CombinedRtp {
        ip: IpAddr,
        port: u16,
        protocol: TransportProtocol,
        id: TransportId,
        srtp_crypto_suite: SrtpCryptoSuite,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebRtcTransportInitData {
    pub id: TransportId,
    pub ice_parameters: IceParameters,
    pub ice_candidates: Vec<IceCandidate>,
    pub dtls_parameters: DtlsParameters,
    pub sctp_parameters: Option<SctpParameters>,
}

#[derive(Deserialize)]
pub struct ConnectTransportData {
    pub id: TransportId,
    #[serde(flatten)]
    pub params: ConnectTransportParams,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ConnectTransportParams {
    #[serde(rename_all = "camelCase")]
    WebRtc { dtls_parameters: DtlsParameters },
    #[serde(rename_all = "camelCase")]
    Rtp { srtp_parameters: SrtpParameters },
}
