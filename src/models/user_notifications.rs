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
impl ModelExt for UserNotification {
    async fn get_connection() -> &'static Database {
        database::connection().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, WitherModel, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserNotification {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub r#type: String,
    pub user_id: String,
    pub title: String,
    pub message: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_read: bool,
}
