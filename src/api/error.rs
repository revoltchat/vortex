use serde::Serialize;
use std::convert::Infallible;
use std::fmt::{self, Display};
use strum::IntoStaticStr;

use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Rejection, Reply};

#[derive(Debug, IntoStaticStr)]
pub enum ApiError {
    Unauthorized,
    InternalServerError,

    RoomNotFound(String),
    RoomAlreadyExists(String),

    UserNotFound(String),
    UserAlreadyExists(String),
}

impl ApiError {
    pub fn code(&self) -> StatusCode {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,

            ApiError::RoomNotFound(_) | ApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
            ApiError::RoomAlreadyExists(_) | ApiError::UserAlreadyExists(_) => StatusCode::CONFLICT,
        }
    }
}

impl Reject for ApiError {}

impl Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Invalid management token"),
            ApiError::InternalServerError => write!(f, "Internal Server Error"),

            ApiError::RoomNotFound(id) => write!(f, "Room with ID {} not found", id),
            ApiError::RoomAlreadyExists(id) => write!(f, "Room with ID {} already exists", id),

            ApiError::UserNotFound(id) => write!(f, "User with ID {} not found", id),
            ApiError::UserAlreadyExists(id) => write!(f, "User with ID {} already exists", id),
        }
    }
}

#[derive(Serialize)]
struct ErrorMessage {
    error: &'static str,
    message: Option<String>,
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let error;
    let mut message = None;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        error = "NotFound";
    } else if let Some(api_error) = err.find::<ApiError>() {
        code = api_error.code();
        error = api_error.into();

        match api_error {
            ApiError::InternalServerError => (),
            _ => message = Some(api_error.to_string()),
        }
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        error = "InternalServerError";
    }

    let json = warp::reply::json(&ErrorMessage { error, message });

    Ok(warp::reply::with_status(json, code))
}
