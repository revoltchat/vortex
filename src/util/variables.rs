use std::convert::TryFrom;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use mediasoup::data_structures::TransportListenIp;
use mediasoup::prelude::TransportListenIps;

lazy_static! {
    // HTTP API
    pub static ref HTTP_HOST: SocketAddr = env::var("HTTP_HOST")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .expect("HTTP_HOST environment variable is not a valid IP:port");
    pub static ref WS_URL: String =
        env::var("WS_URL").expect("Missing WS_URL environment variable.");
    pub static ref MANAGE_TOKEN: String =
        env::var("MANAGE_TOKEN").expect("Missing MANAGE_TOKEN environment variable.");

    // RTC
    pub static ref RTC_IPS: TransportListenIps = {
        let ip_list = env::var("RTC_IPS").expect("Missing RTC_IPS environment variable.");
        let ip_list = ip_list.split(';');
        let mut ip_vec = Vec::new();
        for ip_pair in ip_list {
            let mut iter = ip_pair.split(',');
            if let Some(ip) = iter.next() {
                let ip = IpAddr::from_str(&ip).expect("Not a valid listen IP");
                let announced_ip = iter.next().map(|ip| IpAddr::from_str(&ip).expect("Not a valid announcement IP"));
                if ip.is_unspecified() && announced_ip.is_none() {
                    panic!("RTC announcement IP must be specified when listen IP is 0.0.0.0");
                }

                if let Some(announced_ip) = announced_ip {
                    if announced_ip.is_unspecified() {
                        panic!("RTC announcement IP must not be 0.0.0.0");
                    }
                }

                ip_vec.push(TransportListenIp {
                    ip, announced_ip,
                });
            }
        }

        TransportListenIps::try_from(ip_vec).unwrap()
    };

    pub static ref RTC_MIN_PORT: u16 = env::var("RTC_MIN_PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse()
        .expect("RTC_MIN_PORT is not a valid 16-bit number");
    pub static ref RTC_MAX_PORT: u16 = env::var("RTC_MAX_PORT")
        .unwrap_or_else(|_| "11000".to_string())
        .parse()
        .expect("RTC_MAX_PORT is not a valid 16-bit number");
    pub static ref DISABLE_RTP: bool = env::var("DISABLE_RTP").map_or(false, |v| v == "1");
}

pub fn preflight_checks() {
    format!("{}", *WS_URL);
    format!("{}", *MANAGE_TOKEN);

    format!("{}", RTC_IPS.len());
}
