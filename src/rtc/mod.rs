use std::num::{NonZeroU8, NonZeroU32};

use mediasoup::rtp_parameters::{MimeTypeAudio, RtpCodecCapability, RtpCodecParametersParameters};

pub mod worker;

pub use worker::get_worker_pool;

pub fn create_opus_codec(channels: u8) -> RtpCodecCapability {
    RtpCodecCapability::Audio {
        mime_type: MimeTypeAudio::Opus,
        preferred_payload_type: None,
        clock_rate: NonZeroU32::new(48000).unwrap(),
        channels: NonZeroU8::new(channels).expect("Invalid number of audio channels provided"),
        parameters: RtpCodecParametersParameters::default(),
        rtcp_feedback: Vec::new()
    }
}
