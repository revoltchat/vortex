use warp::Filter;
use warp::filters::BoxedFilter;
use warp::reply::Reply;

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let route = warp::any().map(|| "test yes");

    route.boxed()
}
