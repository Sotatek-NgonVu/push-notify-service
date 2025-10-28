use std::str::FromStr;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, VariantNames, Display};

#[derive(Debug, PartialEq, Clone)]
pub enum EAvailableScope {
    FullAccess,
    FullReadOnly,
    PositionsReadOnly,
    PositionsWrite,
    PresetSettingReadOnly,
    PresetSettingWrite,
    TransactionsReadOnly,
    HoldersReadOnly,
    FavoritesReadOnly,
    FavoritesWrite,
    ReferralsReadOnly,
    ReferralsWrite,
    TokenReadOnly,
    SettingsFullAccess,
}

impl FromStr for EAvailableScope {
    type Err = String;

    fn from_str(input: &str) -> Result<EAvailableScope, Self::Err> {
        match input.to_lowercase().as_str() {
            "full_access" => Ok(EAvailableScope::FullAccess),
            "full_read_only" => Ok(EAvailableScope::FullReadOnly),
            "common.positions.read_only" => Ok(EAvailableScope::PositionsReadOnly),
            "common.positions.write" => Ok(EAvailableScope::PositionsWrite),
            "common.preset_settings.read_only" => Ok(EAvailableScope::PresetSettingReadOnly),
            "common.preset_settings.write" => Ok(EAvailableScope::PresetSettingWrite),
            "common.transactions.read_only" => Ok(EAvailableScope::TransactionsReadOnly),
            "common.holders.read_only" => Ok(EAvailableScope::HoldersReadOnly),
            "common.favorites.read_only" => Ok(EAvailableScope::FavoritesReadOnly),
            "common.favorites.write" => Ok(EAvailableScope::FavoritesWrite),
            "common.referrals.read_only" => Ok(EAvailableScope::ReferralsReadOnly),
            "common.referrals.write" => Ok(EAvailableScope::ReferralsWrite),
            "common.token.read_only" => Ok(EAvailableScope::TokenReadOnly),
            "settings.full_access" => Ok(EAvailableScope::SettingsFullAccess),
            _ => Err(format!("Invalid scope: {input}")),
        }
    }
}


#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    EnumString,
    VariantNames,
    Display,
    PartialEq,
    Eq,
    Hash,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum KafkaTopic {
    #[strum(serialize = "raidenx.user.notify.persister")]
    UserNotificationPersister,
    #[strum(serialize = "raidenx.user.notify.publisher")]
    UserNotificationPublisher,
}