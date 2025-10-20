use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::convert::Infallible;
use tokio::task::JoinError;
use wither::WitherError;
use wither::bson;
use wither::mongodb::error::Error as MongoError;

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum Error {
    #[error("{0}")]
    Wither(#[from] WitherError),

    #[error("{0}")]
    Mongo(#[from] MongoError),

    #[error("Error parsing ObjectID {0}")]
    ParseObjectID(String),

    #[error("{0}")]
    SerializeMongoResponse(#[from] bson::de::Error),

    #[error("{0}")]
    Authenticate(#[from] AuthenticateError),

    #[error("{0}")]
    PathError(StatusCode, PathError),

    #[error("{0}")]
    BadRequest(#[from] BadRequest),

    #[error("{0}")]
    NotFound(#[from] NotFound),

    #[error("{0}")]
    Internal(#[from] Internal),

    #[error("{0}")]
    Unauthorized(#[from] Unauthorized),

    #[error("{0}")]
    Forbidden(#[from] Forbidden),

    #[error("{0}")]
    RunSyncTask(#[from] JoinError),

    #[error("{0}")]
    Kafka(#[from] rdkafka::error::KafkaError),

    #[error("Slow synchronization {0}")]
    SlowSynchronization(u64),

    #[error("{0}")]
    Eyre(#[from] eyre::Error),

    #[error("{0}")]
    Rewquest(#[from] reqwest::Error),

    #[error("{0}")]
    InfallibleError(#[from] Infallible),

    #[error("{0}")]
    SerdeJsonError(#[from] serde_json::Error),
}

impl Error {
    fn get_codes(&self) -> (StatusCode, u16) {
        match *self {
            // 4XX Errors
            Error::ParseObjectID(_) => (StatusCode::BAD_REQUEST, 40001),
            Error::BadRequest(_) => (StatusCode::BAD_REQUEST, 40002),
            Error::PathError(status_code, _) => (status_code, 40007),

            Error::NotFound(_) => (StatusCode::NOT_FOUND, 40003),
            Error::Unauthorized(_) => (StatusCode::UNAUTHORIZED, 40003),
            Error::Forbidden(_) => (StatusCode::FORBIDDEN, 40003),
            Error::Authenticate(AuthenticateError::WrongCredentials) => {
                (StatusCode::UNAUTHORIZED, 40004)
            }
            Error::Authenticate(AuthenticateError::InvalidToken) => {
                (StatusCode::UNAUTHORIZED, 40005)
            }
            Error::Authenticate(AuthenticateError::Locked) => (StatusCode::LOCKED, 40006),

            // 5XX Errors
            Error::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5000),
            Error::Authenticate(AuthenticateError::TokenCreation) => {
                (StatusCode::INTERNAL_SERVER_ERROR, 5001)
            }
            Error::Wither(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5002),
            Error::Mongo(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5003),
            Error::SerializeMongoResponse(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5004),
            Error::RunSyncTask(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5005),
            Error::Kafka(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5007),
            Error::SlowSynchronization(_) => (StatusCode::SERVICE_UNAVAILABLE, 5008),

            Error::Eyre(_) => (StatusCode::INTERNAL_SERVER_ERROR, 6001),
            Error::Rewquest(_) => (StatusCode::INTERNAL_SERVER_ERROR, 6002),
            Error::InfallibleError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 6003),
            Error::SerdeJsonError(_) => (StatusCode::INTERNAL_SERVER_ERROR, 6003),
        }
    }

    pub fn bad_request(message: &str) -> Self {
        Error::BadRequest(BadRequest {
            message: message.to_string(),
        })
    }

    pub fn not_found(message: &str) -> Self {
        Error::NotFound(NotFound {
            message: message.to_string(),
        })
    }

    pub fn internal_err(message: &str) -> Self {
        Error::Internal(Internal {
            message: message.to_string(),
        })
    }

    pub fn unauthorized(message: &str) -> Self {
        Error::Unauthorized(Unauthorized {
            message: message.to_string(),
        })
    }

    pub fn forbidden(message: &str) -> Self {
        Error::Forbidden(Forbidden {
            message: message.to_string(),
        })
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        tracing::error!("{self:?}");

        let (status_code, code) = self.get_codes();
        let message = self.to_string();

        let body = match &self {
            Error::PathError(_, PathError { message, location }) => {
                let mut json = json!({ "code": code, "message": message });
                if let Some(loc) = location {
                    json["location"] = json!(loc);
                }
                Json(json)
            }
            _ => Json(json!({ "code": code, "message": message })),
        };

        (status_code, body).into_response()
    }
}

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum AuthenticateError {
    #[error("Wrong authentication credentials")]
    WrongCredentials,
    #[error("Failed to create authentication token")]
    TokenCreation,
    #[error("Invalid authentication credentials")]
    InvalidToken,
    #[error("User is locked")]
    Locked,
}

#[derive(thiserror::Error, Debug)]
#[error("Bad Request: {message}")]
pub struct BadRequest {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Not found: {message}")]
pub struct NotFound {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Internal error: {message}")]
pub struct Internal {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Unauthorized error: {message}")]
pub struct Unauthorized {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Forbidden error: {message}")]
pub struct Forbidden {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Slow synchronization")]
pub struct SlowSynchronization {
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Path Error: {message}")]
pub struct PathError {
    pub message: String,
    pub location: Option<String>,
}
