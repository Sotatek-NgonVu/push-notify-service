use crate::utils::structs::NotificationPreferences;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDto {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkNotificationAsReadResponseDto {
    pub notification_id: String,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotifPreferenceResponseDto {
    pub user_id: String,
    pub preferences: NotificationPreferences,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditNotifPreferenceRequestDto {
    pub preferences: NotificationPreferences,
}
