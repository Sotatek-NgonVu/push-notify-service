use crate::app_state::AppState;
use crate::core::jwt_auth::jwt_auth::JwtAuth;
use crate::errors::Error;
use crate::models::user_fcm_token::UserFcmToken;
use crate::routes::account::dtos::requests::{
    CreateUserFcmTokenRequestDto, DeactivateUserFcmTokenRequestDto,
};
use axum::Json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub fn create_route() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_user_fcm_token))
        .routes(routes!(deactivate_user_fcm_token))
}

#[utoipa::path(
    description = "Create or update a user's FCM token for push notifications.",
    summary = "Create or update FCM token",
    post,
    request_body(
        content = CreateUserFcmTokenRequestDto,
        content_type = "application/json",
    ),
    tag = "Account APIs",
    path = "/api/v1/user-fcm-token",
    responses(
        (status = 200, description = "FCM token created or updated successfully"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
    ("bearer_auth" = [])
    )
)]
pub async fn create_user_fcm_token(
    JwtAuth(claims): JwtAuth,
    Json(request): Json<CreateUserFcmTokenRequestDto>,
) -> Result<(), Error> {
    UserFcmToken::create_or_update(
        claims.user_id,
        request.device_id,
        request.token,
        request.platform,
    )
    .await
    .map_err(|e| Error::internal_err(&format!("Failed to create user fcm token: {}", e)))?;

    Ok(())
}

#[utoipa::path(
    summary = "Deactivate FCM token",
    patch,
    request_body(
        content = DeactivateUserFcmTokenRequestDto,
        content_type = "application/json",
    ),
    tag = "Account APIs",
    path = "/api/v1/user-fcm-token",
    responses(
        (status = 200, description = "FCM token deactivate successfully"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
    ("bearer_auth" = [])
    )
)]
pub async fn deactivate_user_fcm_token(
    JwtAuth(claims): JwtAuth,
    Json(request): Json<DeactivateUserFcmTokenRequestDto>,
) -> Result<(), Error> {
    UserFcmToken::deactivate_by_user_id_and_device_id(claims.user_id, &request.device_id)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to deactivate user fcm token: {}", e)))?;
    Ok(())
}
