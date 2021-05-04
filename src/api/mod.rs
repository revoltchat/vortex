use warp::{filters::BoxedFilter, reply::Reply};
use warp::{Filter, Rejection};

use crate::util::variables;

pub mod error;
pub use error::ApiError;

pub mod room;

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

    authorize()
        .untuple_one()
        .and(room_routes)
        .recover(error::handle_rejection)
        .boxed()
}
