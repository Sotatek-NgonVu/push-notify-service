use axum::http::request::Parts;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Bearer};
use bytes::Bytes;
use http::{Method, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::time::Instant;

use crate::config::APP_CONFIG;
use crate::core::jwt_auth::jwt_auth::decode_jwt;
use crate::core::jwt_auth::types::TokenClaims;

pub async fn http_logger(
    req: Request,
    next: Next,
) -> std::result::Result<impl IntoResponse, (StatusCode, String)> {
    let start_time = Instant::now();

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let version = req.version();
    let req_headers = req.headers().clone();
    let x_request_id = req_headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Check if request is a file upload
    let is_file_upload = req_headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map(|ct| ct.starts_with("multipart/form-data"))
        .unwrap_or(false);

    let (parts, body) = req.into_parts();

    let bearer_token_option = extract_bearer_token(&parts);

    let jwt_payload = if let Some(bearer_token) = bearer_token_option {
        bearer_token.to_string()
    } else {
        "".to_string()
    };

    let bytes = buffer_body("request", body).await?;
    let bytes_clone = bytes.clone();

    // Only log body if not a file upload
    let req_body = if !is_file_upload {
        let body_str = String::from_utf8_lossy(bytes_clone.as_ref());
        match serde_json::from_str::<Value>(&body_str) {
            Ok(json) => json,
            Err(_) => Value::Object(serde_json::Map::new()),
        }
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Reconstruct request with original body
    let req = Request::from_parts(parts, Body::from(bytes));

    let mut response = next.run(req).await;

    let latency = start_time.elapsed();

    let status = response.status();
    let res_headers = response.headers().clone();

    let should_log_body = matches!(method.as_str(), "POST" | "PUT" | "PATCH");
    let res_body = if should_log_body {
        let (parts, body) = response.into_parts();
        let bytes = buffer_body("response", body).await?;
        let body_str = String::from_utf8_lossy(&bytes);
        let json_body = match serde_json::from_str::<Value>(&body_str) {
            Ok(json) => json,
            Err(_) => Value::Object(serde_json::Map::new()),
        };
        response = Response::from_parts(parts, Body::from(bytes));
        json_body
    } else {
        Value::Object(serde_json::Map::new())
    };

    if method == Method::OPTIONS {
        // ignore OPTIONS requests
        return Ok(response);
    }

    // have to use span.in_scope in async fn
    let span = tracing::info_span!("http_request");
    span.in_scope(|| {
        tracing::info!(
          method = ?method,
          uri = ?uri,
          path = path,
          x_request_id = x_request_id,
          version = ?version,
          req_headers = ?req_headers,
          jwt_payload = jwt_payload,
          req_body = req_body.to_string(),
          status = ?status,
          latency = ?latency,
          latency_micros = latency.as_micros(),
          res_headers = ?res_headers,
          res_body = res_body.to_string(),
          app_env = APP_CONFIG.app_env.to_string()
        );
    });

    Ok(response)
}

pub async fn buffer_body<B>(
    direction: &str,
    body: B,
) -> std::result::Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    Ok(bytes)
}

fn extract_bearer_token(parts: &Parts) -> Option<TokenClaims> {
    // Extract the `Authorization` header from request parts
    if let Some(Authorization(bearer)) = parts.headers.typed_get::<Authorization<Bearer>>() {
        decode_jwt(bearer.token()).ok()
    } else {
        None
    }
}
