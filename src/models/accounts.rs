use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, VariantNames};
use utoipa::ToSchema;
use validator::Validate;
use wither::Model as WitherModel;
use wither::bson::DateTime;
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb::Database;

use crate::database;
use crate::errors::Error;
use crate::utils::models::ModelExt;

#[async_trait]
impl ModelExt for Account {
    async fn get_connection() -> &'static Database {
        database::connection().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, WitherModel, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub user_name: String,
    pub display_name: String,
    pub lang: ELanguage,
    pub joined_at: DateTime,
    pub is_system: bool,
    pub has_wallet: bool,
    pub has_order: bool,
    pub last_login_at: DateTime,
    pub referral_code: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, ToSchema, VariantNames, Display,
)]
#[strum(serialize_all = "lowercase")]
#[allow(non_camel_case_types)]
pub enum ELanguage {
    en,
    vi,
}

impl Account {
    pub async fn get_account_by_user_id(user_id: &str) -> Result<Option<Account>, Error> {
        <Self as ModelExt>::find_one(doc! { "userId": user_id }, None).await
    }
}
