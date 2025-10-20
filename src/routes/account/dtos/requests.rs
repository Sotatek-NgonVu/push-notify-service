use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum UserFcmTokenStatus {
    Active,
    Inactive,
}

impl Display for UserFcmTokenStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserFcmTokenStatus::Active => write!(f, "ACTIVE"),
            UserFcmTokenStatus::Inactive => write!(f, "INACTIVE"),
        }
    }
}

#[derive(Debug, IntoParams, Deserialize, Clone, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserFcmTokenRequestDto {
    pub device_id: Option<String>,
    pub token: String,
    pub platform: Option<String>,
}

#[derive(Debug, IntoParams, Deserialize, Clone, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeactivateUserFcmTokenRequestDto {
    pub device_id: String,
}

#[derive(Debug, IntoParams, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserNicknameRequestDto {
    pub nickname: String,
}
