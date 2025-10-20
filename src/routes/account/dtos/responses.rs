use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TxHistoryResponseDto {
    pub id: i64,
    pub asset: String,
    pub network_id: i64,
    pub tx_hash: Option<String>,
    pub r#type: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub from_user_id: Option<i64>,
    pub to_user_id: Option<i64>,
    pub amount: String,
    pub fee: String,
    pub timestamp: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositTxResponseDto {
    pub id: i64,
    pub user_id: i64,
    pub asset: String,
    pub network_id: i64,
    pub tx_hash: Option<String>,
    pub amount: String,
    pub address: Option<String>,
    pub from_user_id: Option<i64>,
    pub memo_tag: Option<String>,
    pub status: String,
    pub fee: String,
    pub is_external: Option<bool>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawTxResponseDto {
    pub id: i64,
    pub user_id: i64,
    pub asset: String,
    pub network_id: i64,
    pub tx_hash: Option<String>,
    pub amount: String,
    pub address: Option<String>,
    pub to_user_id: Option<i64>,
    pub memo_tag: Option<String>,
    pub status: String,
    pub fee: String,
    pub is_external: Option<bool>,
    pub requested_at: Option<i64>,
    pub sent_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponseDto {
    pub email: String,
    pub withdraw_id: i64,
    pub expires_at: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDepositAddressResponseDto {
    pub id: i64,
    pub user_id: i64,
    pub network_id: i64,
    pub asset: String,
    pub address: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResendWithdrawVerificationCodeResponseDto {
    pub email: String,
    pub withdraw_id: i64,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyWithdrawCodeResponseDto {
    pub withdraw_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponseDto {
    pub id: i64,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub email: Option<String>,
    pub referral_code: Option<String>,
    pub kyc_verified_at: Option<i64>,
    pub kyc_status: String,
    pub kyc_level: Option<String>,
    pub status: String,
    pub deposit_enabled: Option<bool>,
    pub withdraw_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReferrerCodeResponseDto {
    pub message: String,
    pub referrer_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistAddressResponseDto {
    pub id: i64,
    pub address: String,
    pub network_id: i64,
    pub asset: String,
    pub label: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawLimitResponseDto {
    pub id: i64,
    pub kyc_level_name: Option<String>,
    pub deposit_limit: String,
    pub current_withdraw: Option<String>,
    pub withdraw_limit: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecurityCheckupResponseDto {
    pub identity_verification: bool,
    pub two_factor_authentication: bool,
    pub withdrawal_whitelist: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserNicknameResponseDto {
    pub message: String,
    pub nickname: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserReferralCountResponseDto {
    pub total_referral: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferralDetailResponseDto {
    pub user_id: i64,
    pub email: Option<String>,
    pub register_date: i64,
}
