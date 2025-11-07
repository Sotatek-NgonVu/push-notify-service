use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use wither::bson::{DateTime, doc, oid::ObjectId};

use crate::app_state::AppState;
use crate::core::jwt_auth::jwt_auth::JwtAuth;
use crate::errors::Error;
use crate::loading_preferences::update_user_notification_preferences;
use crate::models::user_notification_settings::UserNotificationSetting;
use crate::models::user_notifications::UserNotification;
use crate::routes::notification::dto::{
    EditNotifPreferenceRequestDto, MarkNotificationAsReadResponseDto, NotifPreferenceResponseDto,
    NotificationDto,
};
use crate::utils::models::ModelExt;
use crate::utils::pagination::{PaginationQuery, PaginationResponseDto};
use crate::utils::structs::{NotifType, NotificationPreferences};

pub fn create_route() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(mark_notification_as_read))
        .routes(routes!(get_latest_unread_transaction_notification))
        .routes(routes!(mark_all_transaction_notify_as_read))
        .routes(routes!(get_transaction_notification))
        .routes(routes!(get_latest_unread_account_notification))
        .routes(routes!(mark_all_account_notify_as_read))
        .routes(routes!(get_account_notification))
        .routes(routes!(update_notification_preference))
        .routes(routes!(get_notification_setting))
}

#[utoipa::path(
    patch,
    path = "/read/{id}",
    tag = "Notification APIs",
    params(
        ("id" = String, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read successfully", body = MarkNotificationAsReadResponseDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn mark_notification_as_read(
    JwtAuth(claims): JwtAuth,
    Path(notif_id): Path<String>,
) -> Result<Json<MarkNotificationAsReadResponseDto>, Error> {
    if notif_id.is_empty() {
        return Err(Error::bad_request("Notification ID cannot be empty"));
    }
    if notif_id.len() != 24 {
        return Err(Error::bad_request("Notification ID must be 24 characters"));
    }

    let oid = ObjectId::parse_str(&notif_id)
        .map_err(|_| Error::bad_request("Invalid notification ID format"))?;

    let filter = doc! {
        "_id": oid,
        "userId": &claims.user_id,
    };

    let update = doc! {
        "$set": {
            "isRead": true,
            "updatedAt": DateTime::now(),
        }
    };

    let updated = UserNotification::find_one_and_update(filter, update, false)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to update notification: {}", e)))?
        .ok_or_else(|| Error::not_found("Notification not found"))?;

    tracing::info!(
        "User {} marked notification {} as read.",
        claims.user_id,
        notif_id
    );

    Ok(Json(MarkNotificationAsReadResponseDto {
        notification_id: updated
            .id
            .ok_or_else(|| Error::internal_err("Notification ID is missing"))?
            .to_hex(),
        is_read: updated.is_read,
    }))
}
#[utoipa::path(
    patch,
    path = "/transaction",
    tag = "Notification APIs",
    responses(
        (status = 200, description = "All transaction notifications marked as read successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn mark_all_transaction_notify_as_read(
    JwtAuth(claims): JwtAuth,
) -> Result<Json<()>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Transaction.to_string(),
        "isRead": false,
    };

    let update = doc! {
        "$set": {
            "isRead": true,
            "updatedAt": DateTime::now(),
        }
    };

    UserNotification::update_many(filter, update, None)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to update notifications: {}", e)))?;

    tracing::info!(
        "User {} marked all transaction notifications as read.",
        claims.user_id
    );

    Ok(Json(()))
}

#[utoipa::path(
    patch,
    path = "/account",
    tag = "Notification APIs",
    responses(
        (status = 200, description = "All account notifications marked as read successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn mark_all_account_notify_as_read(JwtAuth(claims): JwtAuth) -> Result<Json<()>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Account.to_string(),
        "isRead": false,
    };

    let update = doc! {
        "$set": {
            "isRead": true,
            "updatedAt": DateTime::now(),
        }
    };

    UserNotification::update_many(filter, update, None)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to update notifications: {}", e)))?;

    tracing::info!(
        "User {} marked all account notifications as read.",
        claims.user_id
    );

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/transaction/latest",
    tag = "Notification APIs",
    responses(
        (status = 200, description = "Latest unread transaction notification retrieved successfully", body = Option<NotificationDto>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_latest_unread_transaction_notification(
    JwtAuth(claims): JwtAuth,
) -> Result<Json<Option<NotificationDto>>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Transaction.to_string(),
        "isRead": false,
    };

    let options = wither::mongodb::options::FindOneOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();

    let notification = UserNotification::find_one(filter, Some(options))
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to fetch notification: {}", e)))?;

    match notification {
        Some(notif) => {
            tracing::info!(
                "User {} has unread transaction notification",
                claims.user_id
            );
            Ok(Json(Some(NotificationDto {
                id: notif
                    .id
                    .ok_or_else(|| Error::internal_err("Notification ID is missing"))?
                    .to_hex(),
                user_id: notif.user_id,
                title: notif.title,
                content: notif.message,
                is_read: notif.is_read,
                created_at: notif.created_at.to_string(),
            })))
        }
        None => {
            tracing::info!(
                "User {} has no unread transaction notifications.",
                claims.user_id
            );
            Ok(Json(None))
        }
    }
}

#[utoipa::path(
    get,
    path = "/account/latest",
    tag = "Notification APIs",
    responses(
        (status = 200, description = "Latest unread account notification retrieved successfully", body = Option<NotificationDto>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_latest_unread_account_notification(
    JwtAuth(claims): JwtAuth,
) -> Result<Json<Option<NotificationDto>>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Account.to_string(),
        "isRead": false,
    };

    let options = wither::mongodb::options::FindOneOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();

    let notification = UserNotification::find_one(filter, Some(options))
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to fetch notification: {}", e)))?;

    match notification {
        Some(notif) => {
            tracing::info!("User {} has unread account notification", claims.user_id);
            Ok(Json(Some(NotificationDto {
                id: notif
                    .id
                    .ok_or_else(|| Error::internal_err("Notification ID is missing"))?
                    .to_hex(),
                user_id: notif.user_id,
                title: notif.title,
                content: notif.message,
                is_read: notif.is_read,
                created_at: notif.created_at.to_string(),
            })))
        }
        None => {
            tracing::info!(
                "User {} has no unread account notifications.",
                claims.user_id
            );
            Ok(Json(None))
        }
    }
}

#[utoipa::path(
    get,
    path = "/account",
    tag = "Notification APIs",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Ok", body = PaginationResponseDto<NotificationDto>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_account_notification(
    JwtAuth(claims): JwtAuth,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginationResponseDto<NotificationDto>>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Account.to_string(),
    };

    let options = wither::mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .skip(pagination.skip() as u64)
        .limit(pagination.limit() as i64)
        .build();

    let notifications = UserNotification::find(filter.clone(), Some(options))
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to fetch notifications: {}", e)))?;

    let total = UserNotification::count(filter)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to count notifications: {}", e)))?;

    let response: Result<Vec<NotificationDto>, Error> = notifications
        .iter()
        .map(|notif| {
            Ok(NotificationDto {
                id: notif
                    .id
                    .as_ref()
                    .ok_or_else(|| Error::internal_err("Notification ID is missing"))?
                    .to_hex(),
                user_id: notif.user_id.clone(),
                title: notif.title.clone(),
                content: notif.message.clone(),
                is_read: notif.is_read,
                created_at: notif.created_at.to_string(),
            })
        })
        .collect();
    let response = response?;

    let total_pages = (total as f64 / pagination.limit() as f64).ceil() as u32;

    Ok(Json(PaginationResponseDto {
        docs: response,
        page: pagination.page(),
        limit: pagination.limit(),
        total_docs: total as u32,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/transaction",
    tag = "Notification APIs",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Ok", body = PaginationResponseDto<NotificationDto>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_transaction_notification(
    JwtAuth(claims): JwtAuth,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginationResponseDto<NotificationDto>>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
        "type": NotifType::Transaction.to_string(),
    };

    let options = wither::mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .skip(pagination.skip() as u64)
        .limit(pagination.limit() as i64)
        .build();

    let notifications = UserNotification::find(filter.clone(), Some(options))
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to fetch notifications: {}", e)))?;

    let total = UserNotification::count(filter)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to count notifications: {}", e)))?;

    let response: Result<Vec<NotificationDto>, Error> = notifications
        .iter()
        .map(|notif| {
            Ok(NotificationDto {
                id: notif
                    .id
                    .as_ref()
                    .ok_or_else(|| Error::internal_err("Notification ID is missing"))?
                    .to_hex(),
                user_id: notif.user_id.clone(),
                title: notif.title.clone(),
                content: notif.message.clone(),
                is_read: notif.is_read,
                created_at: notif.created_at.to_string(),
            })
        })
        .collect();
    let response = response?;

    let total_pages = (total as f64 / pagination.limit() as f64).ceil() as u32;

    Ok(Json(PaginationResponseDto {
        docs: response,
        page: pagination.page(),
        limit: pagination.limit(),
        total_docs: total as u32,
        total_pages,
    }))
}

#[utoipa::path(
    patch,
    path = "/preference",
    tag = "Notification APIs",
    request_body(
        content = EditNotifPreferenceRequestDto,
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Ok", body = NotifPreferenceResponseDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_notification_preference(
    JwtAuth(claims): JwtAuth,
    Json(request): Json<EditNotifPreferenceRequestDto>,
) -> Result<Json<NotifPreferenceResponseDto>, Error> {
    // Input validation
    // Validate user_id is not empty
    if claims.user_id.is_empty() {
        return Err(Error::bad_request("User ID cannot be empty"));
    }

    let filter = doc! {
        "userId": &claims.user_id,
    };

    let update = doc! {
        "$set": {
            "account": request.preferences.account,
            "announcement": request.preferences.announcement,
            "campaign": request.preferences.campaign,
            "transaction": request.preferences.transaction,
            "updatedAt": DateTime::now(),
        },
        "$setOnInsert": {
            "userId": &claims.user_id,
            "createdAt": DateTime::now(),
        }
    };

    UserNotificationSetting::find_one_and_update(filter, update, true)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to update preferences: {}", e)))?;

    // Update in-memory cache
    update_user_notification_preferences(claims.user_id.clone(), request.preferences)
        .await
        .map_err(|e| Error::internal_err(&e.to_string()))?;

    Ok(Json(NotifPreferenceResponseDto {
        user_id: claims.user_id,
        preferences: request.preferences,
    }))
}

#[utoipa::path(
    get,
    path = "/preference",
    tag = "Notification APIs",
    responses(
        (status = 200, description = "Ok", body = NotifPreferenceResponseDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_notification_setting(
    JwtAuth(claims): JwtAuth,
) -> Result<Json<NotifPreferenceResponseDto>, Error> {
    let filter = doc! {
        "userId": &claims.user_id,
    };

    let setting = UserNotificationSetting::find_one(filter, None)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to fetch preferences: {}", e)))?;

    match setting {
        Some(setting) => Ok(Json(NotifPreferenceResponseDto {
            user_id: setting.user_id,
            preferences: NotificationPreferences {
                announcement: setting.announcement,
                account: setting.account,
                campaign: setting.campaign,
                transaction: setting.transaction,
            },
        })),
        None => Ok(Json(NotifPreferenceResponseDto {
            user_id: claims.user_id,
            preferences: NotificationPreferences {
                announcement: true,
                account: true,
                campaign: true,
                transaction: true,
            },
        })),
    }
}
