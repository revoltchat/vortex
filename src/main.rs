#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use futures::try_join;
use warp::Filter;

pub mod state;
pub mod util;

pub mod api;
pub mod info;
pub mod ws;

pub mod rtc;

use util::variables::HTTP_HOST;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    info!("Starting Revolt Vortex voice server");
    util::variables::preflight_checks();

    let worker_pool = rtc::worker::WorkerPool::new().await;
    rtc::worker::WORKER_POOL.set(worker_pool).unwrap();

    let info_route = warp::path::end()
        .and(warp::get())
        .map(|| warp::reply::json(&info::get_info()));

    let ws_route = warp::path::end().and(ws::route());

    let route = ws_route.or(info_route).or(api::route());

    let warp_serve = warp::serve(route).run(*HTTP_HOST);
    let warp_future = tokio::spawn(warp_serve);

    try_join!(warp_future).unwrap();
}
