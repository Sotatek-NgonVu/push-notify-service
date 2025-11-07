use bson::DateTime;
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use crate::core::cache::redis_emitter::get_redis_emitter;
use crate::models::user_notifications::UserNotification;

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct PubUserNotification {
    #[serde(rename = "i")]
    pub notification_id: Option<ObjectId>,
    #[serde(rename = "u")]
    pub user_id: String,
    #[serde(rename = "m")]
    pub message: String,
    #[serde(rename = "t")]
    pub r#type: String,
    #[serde(rename = "ca")]
    pub created_at: DateTime,
    #[serde(rename = "ua")]
    pub updated_at: DateTime,
}

#[allow(dead_code)]
pub async fn emit_user_notify(
    user_notification: UserNotification
) -> anyhow::Result<()> {
    let redis_emitter = get_redis_emitter();

    let pub_notify = PubUserNotification {
        notification_id: user_notification.id,
        user_id: user_notification.user_id.clone(),
        r#type: user_notification.r#type.clone(),
        message: user_notification.message.clone(),
        created_at: user_notification.created_at,
        updated_at: user_notification.updated_at,
    };
    let pub_data = serde_json::to_string(&pub_notify)?;

    let notification_room = format!("notification:user:{}", user_notification.user_id);
    let _ = redis_emitter.emit_room(&notification_room, "NewNotification", &pub_data);

    tracing::info!(
        "Successfully emitted user notification via WebSocket for user {}",
        user_notification.user_id
    );

    Ok(())
}
