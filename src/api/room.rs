use std::convert::Infallible;
use serde::Serialize;

use warp::{filters::BoxedFilter, reply::Reply};
use warp::{Filter, Rejection};

use crate::api::ApiError;
use crate::state::room::{Room, ROOMS};

#[derive(Serialize)]
struct RoomReply {
    #[serde(rename = "videoAllowed")]
    video_allowed: bool,
    users: Vec<()>
}

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let get_rooms = warp::path::end().and_then(|| async move {
        let map = ROOMS.read().await;
        let rooms: Vec<&String> = map.keys().collect();
        Ok::<warp::reply::Json, Infallible>(warp::reply::json(&rooms))
    });

    let get_room = warp::path::param::<String>()
        .and(warp::path::end())
        .and_then(|id: String| async move {
            if let Some(_room) = Room::get(&id).await {
                Ok(warp::reply::json(&RoomReply {
                    video_allowed: false,
                    users: Vec::new()
                }))
            } else {
                Err(warp::reject::custom(ApiError::RoomNotFound(id)))
            }
        });

    get_rooms
        .or(get_room)
        .boxed()
}
