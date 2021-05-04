use serde::Serialize;
use std::convert::Infallible;

use warp::Filter;
use warp::{filters::BoxedFilter, http::StatusCode, reply::Reply};

use crate::api::ApiError;
use crate::state::room::{Room, ROOMS};

#[derive(Serialize)]
struct RoomReply {
    #[serde(rename = "videoAllowed")]
    video_allowed: bool,
    users: Vec<()>,
}

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let get_rooms = warp::path::end().and(warp::get()).and_then(|| async move {
        let map = ROOMS.read().await;
        let rooms: Vec<&String> = map.keys().collect();
        Ok::<warp::reply::Json, Infallible>(warp::reply::json(&rooms))
    });

    let get_room = warp::path::param::<String>()
        .and(warp::path::end())
        .and(warp::get())
        .and_then(|id: String| async move {
            if let Some(_room) = Room::get(&id).await {
                Ok(warp::reply::json(&RoomReply {
                    video_allowed: false,
                    users: Vec::new(),
                }))
            } else {
                Err(warp::reject::custom(ApiError::RoomNotFound(id)))
            }
        });

    let create_room = warp::path::param::<String>()
        .and(warp::path::end())
        .and(warp::post())
        .and_then(|id: String| async move {
            match Room::new(id).await {
                Ok(_) => Ok(warp::reply::with_status(
                    warp::reply::reply(),
                    StatusCode::CREATED,
                )),
                Err(err) => Err(warp::reject::custom(err)),
            }
        });

    let delete_room = warp::path::param::<String>()
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(|id: String| async move {
            match Room::get(&id).await {
                Some(room) => {
                    room.delete().await;
                    Ok(warp::reply::with_status(
                        warp::reply::reply(),
                        StatusCode::NO_CONTENT,
                    ))
                }
                None => Err(warp::reject::custom(ApiError::RoomNotFound(id))),
            }
        });

    get_rooms
        .or(get_room)
        .or(create_room)
        .or(delete_room)
        .boxed()
}
