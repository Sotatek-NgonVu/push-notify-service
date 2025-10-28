use crate::database;
use crate::utils::models::ModelExt;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use validator::Validate;
use wither::Model as WitherModel;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb::Database;

#[async_trait]
impl ModelExt for UserNotificationSetting {
    async fn get_connection() -> &'static Database {
        database::connection().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, WitherModel, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserNotificationSetting {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub account: bool,
    pub announcement: bool,
    pub campaign: bool,
    pub transaction: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
