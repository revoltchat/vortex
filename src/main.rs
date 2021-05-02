#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use futures::try_join;
use warp::Filter;

pub mod api;
pub mod info;
pub mod util;

use util::variables::HTTP_HOST;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    info!("Starting Revolt Voso voice server");
    util::variables::preflight_checks();

    let info_route = warp::path::end().map(|| warp::reply::json(&info::get_info()));

    let route = info_route.or(api::route());

    let warp_serve = warp::serve(route).run(*HTTP_HOST);
    let warp_future = tokio::spawn(warp_serve);

    try_join!(warp_future).unwrap();
}
