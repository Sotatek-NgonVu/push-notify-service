use axum::response::IntoResponse;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::app_state::AppState;

pub fn create_route() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(health_check))
}

#[utoipa::path(
  get,
  path = "/",
  tag = "health",
  responses(
      (status = 200, description = "Health check successful"),
      (status = 500, description = "Health check failed")
  )
)]
async fn health_check() -> impl IntoResponse {
    "OK"
}
