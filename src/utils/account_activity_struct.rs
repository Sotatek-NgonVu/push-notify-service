use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AccountNotifType {
    Kyc(KycAction),
    Whitelisting(WhitelistingAction),
    Account(AccountAction),
    Mfa(MfaAction),
    Password(PasswordAction),
}

impl Display for AccountNotifType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountNotifType::Kyc(action) => match action {
                KycAction::Approved => write!(f, "verify KYC"),
                KycAction::Upgraded => write!(f, "upgrade KYC"),
            },
            AccountNotifType::Whitelisting(action) => match action {
                WhitelistingAction::Enabled => write!(f, "enable withdrawal address whitelisting"),
                WhitelistingAction::Disabled => {
                    write!(f, "disable withdrawal address whitelisting")
                }
                WhitelistingAction::Added => write!(f, "add withdrawal address to whitelist"),
                WhitelistingAction::Removed => {
                    write!(f, "remove withdrawal address from whitelist")
                }
            },
            AccountNotifType::Account(action) => match action {
                AccountAction::Disabled => write!(f, "disable account"),
                AccountAction::Deleted => write!(f, "delete account"),
            },
            AccountNotifType::Mfa(action) => match action {
                MfaAction::Enabled => write!(f, "enable two-factor authentication"),
                MfaAction::Disabled => write!(f, "disable two-factor authentication"),
            },
            AccountNotifType::Password(action) => match action {
                PasswordAction::Initialized => write!(f, "initialize password"),
                PasswordAction::Change => write!(f, "change password"),
                PasswordAction::Reset => write!(f, "reset password"),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KycAction {
    Approved,
    Upgraded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WhitelistingAction {
    Enabled,
    Disabled,
    Added,
    Removed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AccountAction {
    Disabled,
    Deleted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MfaAction {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PasswordAction {
    Initialized,
    Change,
    Reset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActionStatus {
    Failed,
    Success,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccountNotifData {
    pub user_id: String,
    pub activity_type: AccountNotifType,
    pub action_status: ActionStatus,
}

impl AccountNotifData {
    pub fn construct_message(&self) -> String {
        let time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match self.action_status {
            ActionStatus::Failed => format!(
                "Your request to {} failed on {}. If you do not recognize this activity, please contact us immediately.",
                self.activity_type, time
            ),
            ActionStatus::Success => match self.activity_type {
                AccountNotifType::Kyc(action) => match action {
                    KycAction::Approved => {
                        format!("Your identity verification was approved on {time}.")
                    }
                    KycAction::Upgraded => {
                        format!("Your verification level was upgraded on {time}.")
                    }
                },
                AccountNotifType::Whitelisting(action) => match action {
                    WhitelistingAction::Enabled => {
                        format!("Withdrawal address whitelisting was enabled on {time}.")
                    }
                    WhitelistingAction::Disabled => {
                        format!("Withdrawal address whitelisting was disabled on {time}.")
                    }
                    WhitelistingAction::Added => {
                        format!("A new withdrawal address was added to your whitelist on {time}.")
                    }
                    WhitelistingAction::Removed => {
                        format!("A withdrawal address was removed from your whitelist on {time}.")
                    }
                },
                AccountNotifType::Account(action) => match action {
                    AccountAction::Disabled => format!(
                        "Your account was disabled on {time}. If you do not recognize this activity, please contact us immediately."
                    ),
                    AccountAction::Deleted => format!(
                        "Your account was permanently deleted on {time}. All data has been removed as requested."
                    ),
                },
                AccountNotifType::Mfa(action) => match action {
                    MfaAction::Enabled => {
                        format!("Two-factor authentication was enabled on {time}.")
                    }
                    MfaAction::Disabled => format!(
                        "Two-factor authentication was disabled on {time}. If you do not recognize this activity, please contact us immediately."
                    ),
                },
                AccountNotifType::Password(action) => match action {
                    PasswordAction::Initialized => format!(
                        "Your account password was set up on {time}. Your account is ready to use."
                    ),
                    PasswordAction::Change => format!(
                        "Your password was changed on {time}. If you do not recognize this activity, please contact us immediately."
                    ),
                    PasswordAction::Reset => format!(
                        "Your password was reset on {time}. If you do not recognize this activity, please contact us immediately."
                    ),
                },
            },
        }
    }
}
