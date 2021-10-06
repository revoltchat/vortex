use crate::util::variables;
use serde::Serialize;

pub static VORTEX_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
pub struct Info {
    vortex: &'static str,
    features: Features,
    ws: &'static str,
}

#[derive(Serialize)]
pub struct Features {
    rtp: bool,
}

pub fn get_info() -> Info {
    let features = Features {
        rtp: !*variables::DISABLE_RTP,
    };

    Info {
        vortex: VORTEX_VERSION,
        features,
        ws: &variables::WS_URL,
    }
}
