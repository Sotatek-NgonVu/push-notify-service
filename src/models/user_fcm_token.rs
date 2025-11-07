//! main database

use crate::database;
use crate::errors::Error;
use crate::enums::UserFcmTokenStatus;
use crate::utils::models::ModelExt;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::Model as WitherModel;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb::Database;

#[async_trait]
impl ModelExt for UserFcmToken {
    async fn get_connection() -> &'static Database {
        database::connection().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, WitherModel, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserFcmToken {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub device_id: String,
    pub token: String,
    pub platform: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub status: String,
}

impl UserFcmToken {
    pub async fn create_or_update(
        user_id: String,
        device_id: Option<String>,
        token: String,
        platform: Option<String>,
    ) -> Result<Self, Error> {
        let now = DateTime::now();

        let query = doc! {
            "deviceId": &device_id,
        };

        let update = doc! {
            "$set": {
                "userId": &user_id,
                "deviceId": &device_id,
                "token": &token,
                "platform": platform.as_ref(),
                "status": UserFcmTokenStatus::Active.to_string(),
                "updatedAt": now,
            },
            "$setOnInsert": {
                "createdAt": now,
            }
        };

        let result = <Self as ModelExt>::find_one_and_update(query, update, true).await?;

        result.ok_or_else(|| {
            let msg = format!(
                "Failed to create or update FCM token for user_id={}, device_id={}",
                user_id,
                device_id.unwrap_or_default()
            );
            Error::internal_err(&msg)
        })
    }

    pub async fn deactivate_by_user_id_and_device_id(
        user_id: String,
        device_id: &String,
    ) -> Result<Self, Error> {
        let now = DateTime::now();

        let query = doc! {
            "deviceId": device_id,
            "userId": &user_id
        };

        let update = doc! {
            "$set": {
                "status": UserFcmTokenStatus::Inactive.to_string(),
                "updatedAt": now,
            }
        };

        let result = <Self as ModelExt>::find_one_and_update(query, update, false).await?;

        result.ok_or_else(|| {
            let msg = format!(
                "No FCM token found for user_id={} and device_id={}",
                user_id, device_id
            );
            Error::not_found(&msg)
        })
    }
}
