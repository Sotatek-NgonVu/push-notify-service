use axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response};
use std::net::SocketAddr;

use crate::{core::cache::redis_service::RedisService, errors::Error};

const RATE_LIMIT_PER_MINUTE: i32 = 600;

pub async fn rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Error> {
    let redis_service = RedisService::new().await;

    let ip = addr.ip().to_string();
    let count = redis_service.get_rate_limit(&ip).await.ok();

    if let Some(count) = count {
        if count >= RATE_LIMIT_PER_MINUTE {
            return Err(Error::bad_request(
                "Rate limit exceeded. Please try again later.",
            ));
        }
    }

    let new_count = count.unwrap_or(0) + 1;
    redis_service.set_rate_limit(&ip, new_count).await?;

    Ok(next.run(req).await)
}
