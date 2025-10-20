use axum::body::HttpBody;
use axum::{
    body::{Body, Bytes},
    http::{Request, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use http::Method;
use serde_json::Value;
use uuid::Uuid;

pub async fn log_request_response(
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let loggable_get_paths = [].to_vec();

    let should_log = match req.method() {
        &Method::POST | &Method::PUT => true,
        &Method::GET => {
            let path = req.uri().path();
            loggable_get_paths.contains(&path)
        }
        _ => false,
    };

    if should_log {
        let new_uuid = Uuid::new_v4();

        let (parts, body) = req.into_parts();
        let bytes = buffer_and_print(&format!("Request {new_uuid}"), body)
            .await
            .unwrap();
        let req = Request::from_parts(parts, Body::from(bytes));

        let res = next.run(req).await;

        let (parts, body) = res.into_parts();
        let bytes = buffer_and_print(&format!("Response {new_uuid}"), body)
            .await
            .unwrap();
        let res = Response::from_parts(parts, Body::from(bytes));

        return Ok(res);
    }

    // For other methods, just call the next middleware/handler without logging
    req.extensions_mut();
    Ok(next.run(req).await)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: HttpBody<Data = Bytes> + Send + 'static,
    B::Error: std::fmt::Display,
    axum::body::Body: std::convert::From<B>,
{
    // Convert the body to a `Body` type compatible with Axum
    let body = Body::from(body);

    // Use `axum::body::to_bytes` to collect the body into bytes
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction:#?} body: {err:#?}"),
            ));
        }
    };

    // Attempt to log the body as a UTF-8 string if possible
    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        if !body_str.is_empty() {
            let body_parsed_json: Value = serde_json::from_str(body_str).expect("Invalid JSON");
            tracing::debug!("\n{:#?} body = {:#?}", direction, body_parsed_json);
        } else {
            tracing::debug!("\n{:#?} body = {:#?}", direction, body_str);
        }
    }

    Ok(bytes)
}
