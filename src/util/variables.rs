use std::env;
use std::net::SocketAddr;

lazy_static! {
    pub static ref HTTP_HOST: SocketAddr = env::var("HTTP_HOST")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .expect("HTTP_HOST environment variable is not a valid IP:port");
    pub static ref WS_URL: String =
        env::var("WS_URL").expect("Missing WS_URL environment variable.");
    pub static ref MANAGE_TOKEN: String =
        env::var("MANAGE_TOKEN").expect("Missing MANAGE_TOKEN environment variable.");
    pub static ref RTC_IPS: String =
        env::var("RTC_IPS").expect("Missing RTC_IPS environment variable.");
    pub static ref RTC_MIN_PORT: String =
        env::var("RTC_MIN_PORT").unwrap_or_else(|_| "10000".to_string());
    pub static ref RTC_MAX_PORT: String =
        env::var("RTC_MAX_PORT").unwrap_or_else(|_| "11000".to_string());
    pub static ref DISABLE_RTP: bool = env::var("DISABLE_RTP").map_or(false, |v| v == "1");
}

pub fn preflight_checks() {
    format!("{}", *WS_URL);
    format!("{}", *MANAGE_TOKEN);

    format!("{}", *RTC_IPS);
}
