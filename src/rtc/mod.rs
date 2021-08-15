use std::num::{NonZeroU32, NonZeroU8};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use futures::join;
use mediasoup::prelude::*;
use crate::util::variables::{RTC_IPS, DISABLE_RTP};

pub mod types;
pub mod worker;

pub use worker::get_worker_pool;

use types::{InitializationInput, InitializationInputMode};

pub fn create_opus_codec(channels: u8) -> RtpCodecCapability {
    RtpCodecCapability::Audio {
        mime_type: MimeTypeAudio::Opus,
        preferred_payload_type: None,
        clock_rate: NonZeroU32::new(48000).unwrap(),
        channels: NonZeroU8::new(channels).expect("Invalid number of audio channels provided"),
        parameters: RtpCodecParametersParameters::default(),
        rtcp_feedback: Vec::new(),
    }
}

pub struct RtcState {
    rtp_capabilities: RtpCapabilities,
    transport_mode: TransportMode,
    consumers: HashMap<String, Consumer>
}

impl RtcState {
    pub async fn initialize(router: Router, init_data: InitializationInput) -> Result<Self, ()> {
        let mut webrtc_options = WebRtcTransportOptions::new(RTC_IPS.clone());
        webrtc_options.enable_udp = true;
        webrtc_options.enable_tcp = true;
        webrtc_options.prefer_udp = true;

        let transport_mode = match init_data.mode {
            InitializationInputMode::SplitWebRtc => {
                let (send, recv) = join!(
                    router.create_webrtc_transport(webrtc_options.clone()),
                    router.create_webrtc_transport(webrtc_options));
                TransportMode::SplitWebRtc(send.map_err(|_| ())?, recv.map_err(|_| ())?)
            },
            InitializationInputMode::CombinedWebRtc => {
                let transport = router.create_webrtc_transport(webrtc_options).await;
                TransportMode::CombinedWebRtc(transport.map_err(|_| ())?)
            },
            InitializationInputMode::CombinedRtp => {
                // TODO: make it return an error struct instead of ()
                if *DISABLE_RTP {
                    return Err(())
                }

                let mut options = PlainTransportOptions::new(RTC_IPS[0]);
                options.rtcp_mux = true;
                options.comedia = true;
                options.enable_srtp = true;
                options.srtp_crypto_suite = SrtpCryptoSuite::AesCm128HmacSha180;
                let transport = router.create_plain_transport(options).await;
                TransportMode::CombinedRtp(transport.map_err(|_| ())?)
            }
        };

        Ok(RtcState {
            rtp_capabilities: init_data.rtp_capabilities,
            transport_mode,
            consumers: HashMap::new(),
        })
    }

    pub fn combined(&self) -> bool { self.transport_mode.combined() }
}

enum TransportMode {
    SplitWebRtc(WebRtcTransport, WebRtcTransport),
    CombinedWebRtc(WebRtcTransport),
    CombinedRtp(PlainTransport),
}

impl TransportMode {
    pub fn combined(&self) -> bool {
        match *self {
            TransportMode::SplitWebRtc(..) => false,
            TransportMode::CombinedWebRtc(..) => true,
            TransportMode::CombinedRtp(..) => true,
        }
    }
}
