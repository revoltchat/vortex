use warp::{filters::BoxedFilter, reply::Reply};
use warp::{Filter, Rejection};

use crate::util::variables;

pub mod error;
pub use error::ApiError;

pub mod room;
pub mod user;

fn authorize() -> impl Filter<Extract = ((),), Error = Rejection> + Copy {
    warp::header::optional("Authorization").and_then(|authorization: Option<String>| async move {
        match authorization {
            Some(authorization) => {
                if authorization == *variables::MANAGE_TOKEN {
                    return Ok(());
                }

                Err(warp::reject::custom(ApiError::Unauthorized))
            }
            None => Err(warp::reject::custom(ApiError::Unauthorized)),
        }
    })
}

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let room_routes = warp::path("room").and(room::route());
    let user_routes = warp::path("room").and(user::route());

    let routes = room_routes.or(user_routes);
    let log = warp::filters::log::custom(|info| {
        info!("{} {}: {}",
            info.method(),
            info.path(),
            info.status(),
        );
    });

    authorize()
        .untuple_one()
        .and(routes)
        .recover(error::handle_rejection)
        .with(log)
        .boxed()
}
