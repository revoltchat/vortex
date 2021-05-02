use std::convert::Infallible;

use warp::reply::Reply;
use warp::Filter;

pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
    let route = warp::any().map(|| "test yes");

    route
}
