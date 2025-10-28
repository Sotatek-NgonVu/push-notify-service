use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::constants::TradingType;
use crate::utils::account_activity_struct::AccountNotifData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionNotifData {
    pub id: u64,
    pub user_id: String,
    pub asset: String,
    pub network_id: String,
    pub tx_hash: String,
    pub r#type: TradingType,
    pub amount: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderNotifData {
    pub order_id: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifMessage {
    pub user_id: String,
    pub notif_type: NotifType,
    pub timestamp: i64,
    pub metadata: NotifMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotifMetadata {
    Order(OrderNotifData),
    Transaction(TransactionNotifData),
    Account(AccountNotifData),
}

impl NotifMetadata {
    pub fn construct_message(&self) -> anyhow::Result<String> {
        match self {
            NotifMetadata::Order(order_data) => {
                let order_id = order_data.order_id.to_string();
                let status = order_data.status.clone();

                match status.as_str() {
                    "NEW" => Ok(format!("Order {order_id} placed successfully.")),
                    "FILLED" => Ok(format!("Order {order_id} matched.")),
                    "CANCELLED" => Ok(format!("Order {order_id} cancelled.")),
                    "REJECTED" => Ok(format!("Order {order_id} rejected.")),
                    _ => Err(anyhow::anyhow!(
                        "Skipping notification for order {order_id} with unsupported status: {status}"
                    )),
                }
            }
            NotifMetadata::Transaction(transaction_data) => {
                let r#type = transaction_data.r#type;
                let amount = transaction_data.amount.to_string();
                let asset = transaction_data.asset.clone();
                let time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

                match transaction_data.status.as_str() {
                    "COMPLETED" => match r#type {
                        TradingType::Add => Ok(format!(
                            "You have successfully deposit {amount} {asset} at {time}",
                        )),
                        TradingType::Remove => Ok(format!(
                            "You have successfully withdraw {amount} {asset} at {time}. If you do not recognize this activity, please contact us immediately."
                        )),
                        TradingType::Buy => Ok(format!(
                            "You have successfully withdraw {amount} {asset} at {time}. If you do not recognize this activity, please contact us immediately."
                        )),
                        TradingType::Sell => Ok(format!(
                            "You have successfully withdraw {amount} {asset} at {time}. If you do not recognize this activity, please contact us immediately."
                        ))
                    },
                    "FAILED" => Ok(format!("Your {} transaction of {amount} {asset} failed at {time}.", r#type)),
                    "REJECTED" => Ok(format!("Your {} transaction of {amount} {asset} failed at {time}.", r#type)),
                    _ => Err(anyhow::anyhow!(
                        "Skipping notification for transaction with unsupported status: {}",
                        transaction_data.status
                    )),
                }
            }
            NotifMetadata::Account(account_data) => Ok(account_data.construct_message()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum NotifType {
    Order,
    Transaction,
    Account,
    Announcement,
    Campaign,
}

impl Display for NotifType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifType::Transaction => write!(f, "TRANSACTION"),
            NotifType::Order => write!(f, "ORDER"),
            NotifType::Account => write!(f, "ACCOUNT"),
            NotifType::Announcement => write!(f, "ANNOUNCEMENT"),
            NotifType::Campaign => write!(f, "CAMPAIGN"),
        }
    }
}

impl FromStr for NotifType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ORDER" => Ok(NotifType::Order),
            "TRANSACTION" => Ok(NotifType::Transaction),
            "ACCOUNT" => Ok(NotifType::Account),
            "ANNOUNCEMENT" => Ok(NotifType::Announcement),
            "CAMPAIGN" => Ok(NotifType::Campaign),
            _ => Err(anyhow::anyhow!("Invalid notification type")),
        }
    }
}

impl NotifType {
    pub fn construct_title(&self) -> String {
        match self {
            NotifType::Transaction => "Transaction Notification".to_string(),
            NotifType::Order => "Order Notification".to_string(),
            NotifType::Account => "Account Notification".to_string(),
            NotifType::Announcement => "Announcement Notification".to_string(),
            NotifType::Campaign => "Campaign Notification".to_string(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NotificationPreferences {
    pub announcement: bool,
    pub account: bool,
    pub campaign: bool,
    pub transaction: bool,
}

impl NotificationPreferences {
    pub fn contains(&self, notif_type: NotifType) -> bool {
        match notif_type {
            NotifType::Transaction => self.transaction,
            NotifType::Order => self.transaction,
            NotifType::Account => self.account,
            NotifType::Announcement => self.announcement,
            NotifType::Campaign => self.campaign,
        }
    }
}

pub struct OrderNotifBuilder {
    pub user_id: String,
    pub message: String,
}
