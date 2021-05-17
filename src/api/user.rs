use serde::Serialize;
use std::sync::Arc;

use warp::Filter;
use warp::{filters::BoxedFilter, http::StatusCode, reply::Reply};

use crate::api::ApiError;
use crate::state::room::Room;

#[derive(Serialize)]
struct CreateUserReply {
    token: String,
}

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let root = super::room::room_filter().and(warp::path("user"));

    let create_user = root
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::post())
        .and_then(|room: Arc<Room>, id: String| async move {
            let users = room.users();
            let user_lock = match users.new(id.clone()).await {
                Ok(user) => user,
                Err(ApiError::UserAlreadyExists(_)) => {
                    debug!(
                        "User {} in room {} already exists, kicking them",
                        &id,
                        room.id()
                    );
                    users.remove(&id).await.ok();
                    users.new(id).await?
                }
                Err(err) => return Err(warp::reject::custom(err)),
            };

            let user = user_lock.read().await;
            Ok(warp::reply::with_status(
                warp::reply::json(&CreateUserReply {
                    token: user.token().unwrap().to_string(),
                }),
                StatusCode::CREATED,
            ))
        });

    create_user.boxed()
}
