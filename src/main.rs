#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use futures::try_join;
use warp::Filter;

pub mod api;
pub mod util;

use util::variables::HTTP_HOST;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    info!("Starting Revolt Voso voice server");
    util::variables::preflight_checks();

    let root_route = warp::path::end().map(|| "test");
    let api_route = warp::path("api").and(api::route());

    let route = root_route.or(api_route);

    let warp_serve = warp::serve(route).run(HTTP_HOST.to_owned());
    let warp_future = tokio::spawn(warp_serve);

    try_join!(warp_future).unwrap();
}
