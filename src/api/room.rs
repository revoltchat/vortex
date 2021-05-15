use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;

use warp::{filters::BoxedFilter, http::StatusCode, reply::Reply};
use warp::{Filter, Rejection};

use crate::api::ApiError;
use crate::state::room::{Room, ROOMS};

#[derive(Serialize)]
struct RoomReply {
    #[serde(rename = "videoAllowed")]
    video_allowed: bool,
    users: Vec<()>,
}

pub fn room_filter() -> impl Filter<Extract = (Arc<Room>,), Error = Rejection> + Copy {
    warp::path::param::<String>().and_then(|id: String| async move {
        match Room::get(&id).await {
            Some(room) => Ok(room),
            None => Err(warp::reject::custom(ApiError::RoomNotFound(id))),
        }
    })
}

pub fn route() -> BoxedFilter<(impl Reply,)> {
    let get_rooms = warp::path::end().and(warp::get()).and_then(|| async move {
        let map = ROOMS.read().await;
        let rooms: Vec<&String> = map.keys().collect();
        Ok::<warp::reply::Json, Infallible>(warp::reply::json(&rooms))
    });

    let get_room = room_filter()
        .and(warp::path::end())
        .and(warp::get())
        .map(|_room: Arc<Room>| {
            warp::reply::json(&RoomReply {
                video_allowed: false,
                users: Vec::new(),
            })
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

    let delete_room = room_filter()
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(|room: Arc<Room>| async move {
            room.delete().await;
            Ok::<_, Infallible>(warp::reply::with_status(
                warp::reply::reply(),
                StatusCode::NO_CONTENT,
            ))
        });

    get_rooms
        .or(get_room)
        .or(create_room)
        .or(delete_room)
        .boxed()
}
