#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

pub mod util;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    info!("Starting Revolt Voso voice server");
    util::variables::preflight_checks();
}
