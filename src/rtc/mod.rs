use std::collections::HashMap;
use std::num::{NonZeroU32, NonZeroU8};

use crate::state::user::ProduceType;
use crate::util::variables::{DISABLE_RTP, RTC_IPS};
use futures::join;
use mediasoup::prelude::*;

pub mod types;
pub mod worker;

pub use worker::get_worker_pool;

pub const SRTP_CRYPTO_SUITE: SrtpCryptoSuite = SrtpCryptoSuite::AesCm128HmacSha180;

use types::{
    ConnectTransportData, ConnectTransportParams, InitializationInput, InitializationInputMode,
    TransportInitData, WebRtcTransportInitData,
};

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
    consumers: HashMap<ConsumerId, Consumer>,
}

impl RtcState {
    pub async fn initialize(router: &Router, init_data: InitializationInput) -> Result<Self, ()> {
        let mut webrtc_options = WebRtcTransportOptions::new(RTC_IPS.clone());
        webrtc_options.enable_udp = true;
        webrtc_options.enable_tcp = true;
        webrtc_options.prefer_udp = true;

        let transport_mode = match init_data.mode {
            InitializationInputMode::SplitWebRtc => {
                let (send, recv) = join!(
                    router.create_webrtc_transport(webrtc_options.clone()),
                    router.create_webrtc_transport(webrtc_options)
                );
                TransportMode::SplitWebRtc(send.map_err(|_| ())?, recv.map_err(|_| ())?)
            }
            InitializationInputMode::CombinedWebRtc => {
                let transport = router.create_webrtc_transport(webrtc_options).await;
                TransportMode::CombinedWebRtc(transport.map_err(|_| ())?)
            }
            InitializationInputMode::CombinedRtp => {
                // TODO: make it return an error struct instead of ()
                if *DISABLE_RTP {
                    return Err(());
                }

                let mut options = PlainTransportOptions::new(RTC_IPS[0]);
                options.rtcp_mux = true;
                options.comedia = true;
                options.enable_srtp = true;
                options.srtp_crypto_suite = SRTP_CRYPTO_SUITE;
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

    pub fn rtp_capabilities(&self) -> &RtpCapabilities {
        &self.rtp_capabilities
    }

    pub fn get_init_data(&self) -> TransportInitData {
        match &self.transport_mode {
            TransportMode::SplitWebRtc(send, recv) => TransportInitData::SplitWebRtc {
                send_transport: RtcState::get_webrtc_init_data(&send),
                recv_transport: RtcState::get_webrtc_init_data(&recv),
            },
            TransportMode::CombinedWebRtc(transport) => TransportInitData::CombinedWebRtc {
                transport: RtcState::get_webrtc_init_data(&transport),
            },
            TransportMode::CombinedRtp(transport) => {
                let tuple = transport.tuple();
                let ip = RTC_IPS[0];
                let ip = ip.announced_ip.unwrap_or(ip.ip);
                TransportInitData::CombinedRtp {
                    ip,
                    port: tuple.local_port(),
                    protocol: tuple.protocol(),
                    id: transport.id(),
                    srtp_crypto_suite: SRTP_CRYPTO_SUITE,
                }
            }
        }
    }

    fn get_webrtc_init_data(transport: &WebRtcTransport) -> WebRtcTransportInitData {
        WebRtcTransportInitData {
            id: transport.id(),
            ice_parameters: transport.ice_parameters().clone(),
            ice_candidates: transport.ice_candidates().clone(),
            dtls_parameters: transport.dtls_parameters(),
            sctp_parameters: transport.sctp_parameters(),
        }
    }

    pub fn combined(&self) -> bool {
        self.transport_mode.combined()
    }

    pub fn get_webrtc_transport_by_id(&self, id: TransportId) -> Option<&WebRtcTransport> {
        match self.transport_mode {
            TransportMode::SplitWebRtc(ref send, ref recv) => Some(send)
                .filter(|t| t.id() == id)
                .or_else(|| Some(recv).filter(|t| t.id() == id)),
            TransportMode::CombinedWebRtc(ref transport) => {
                Some(transport).filter(|t| t.id() == id)
            }
            _ => None,
        }
    }

    pub fn get_rtp_transport_by_id(&self, id: TransportId) -> Option<&PlainTransport> {
        match self.transport_mode {
            TransportMode::CombinedRtp(ref transport) => Some(transport).filter(|t| t.id() == id),
            _ => None,
        }
    }

    pub async fn connect_transport(&self, connect_data: &ConnectTransportData) -> Result<(), ()> {
        match self.transport_mode {
            TransportMode::SplitWebRtc(..) | TransportMode::CombinedWebRtc(..) => {
                if let ConnectTransportParams::WebRtc { dtls_parameters } = &connect_data.params {
                    let transport = self
                        .get_webrtc_transport_by_id(connect_data.id)
                        .ok_or_else(|| ())?;

                    transport
                        .connect(WebRtcTransportRemoteParameters {
                            dtls_parameters: dtls_parameters.clone(),
                        })
                        .await
                        .map_err(|_| ())?;

                    Ok(())
                } else {
                    Err(())
                }
            }
            TransportMode::CombinedRtp(..) => {
                if let ConnectTransportParams::Rtp { srtp_parameters } = &connect_data.params {
                    let transport = self
                        .get_rtp_transport_by_id(connect_data.id)
                        .ok_or_else(|| ())?;
                    transport
                        .connect(PlainTransportRemoteParameters {
                            ip: None,
                            port: None,
                            rtcp_port: None,
                            srtp_parameters: Some(srtp_parameters.clone()),
                        })
                        .await
                        .map_err(|_| ())?;

                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    }

    pub async fn start_produce(
        &self,
        produce_type: &ProduceType,
        rtp_parameters: RtpParameters,
    ) -> Result<Producer, ProduceError> {
        let transport = self.transport_mode.send();
        transport
            .produce(ProducerOptions::new(
                produce_type.into_kind(),
                rtp_parameters,
            ))
            .await
    }

    pub async fn start_consume(
        &mut self,
        producer_id: ProducerId,
    ) -> Result<Consumer, ConsumeError> {
        let transport = self.transport_mode.recv();
        let mut options = ConsumerOptions::new(producer_id, self.rtp_capabilities.clone());
        options.paused = true;
        let consumer = transport.consume(options).await?;
        self.consumers.insert(consumer.id(), consumer.clone());
        Ok(consumer)
    }

    pub fn stop_consume(&mut self, consumer_id: &ConsumerId) -> Result<(), ()> {
        let _ = self.consumers.remove(consumer_id).ok_or_else(|| ())?;
        Ok(())
    }

    pub async fn set_consumer_pause(
        &mut self,
        consumer_id: &ConsumerId,
        paused: bool,
    ) -> Result<(), ()> {
        let consumer = self.consumers.get_mut(consumer_id).ok_or_else(|| ())?;
        match paused {
            true => consumer.pause().await.map_err(|_| ()),
            false => consumer.resume().await.map_err(|_| ()),
        }
    }
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

    pub fn send(&self) -> &dyn Transport {
        match self {
            TransportMode::SplitWebRtc(ref send, _) => send,
            TransportMode::CombinedWebRtc(ref transport) => transport,
            TransportMode::CombinedRtp(ref transport) => transport,
        }
    }

    pub fn recv(&self) -> &dyn Transport {
        match self {
            TransportMode::SplitWebRtc(_, ref recv) => recv,
            TransportMode::CombinedWebRtc(ref transport) => transport,
            TransportMode::CombinedRtp(ref transport) => transport,
        }
    }
}
