use crate::errors::Error;
use crate::loading_preferences::get_user_notification_preferences_batch;
use crate::utils::structs::{NotifMessage, NotifType};
use std::collections::{HashMap, HashSet};

#[derive(Eq, Hash, PartialEq, Debug)]
pub struct NotifKey {
    pub user_id: String,
    pub second: i64,
    pub r#type: NotifType,
}

pub struct NotificationWithTimestamp {
    pub message: String,
    pub timestamp: i64,
}

pub async fn group_by_user_id(
    mut notif_message: Vec<NotifMessage>,
) -> Result<HashMap<NotifKey, Vec<NotificationWithTimestamp>>, Error> {
    notif_message.sort_by_key(|m| m.timestamp);

    let unique_user_ids: HashSet<String> =
        notif_message.iter().map(|n| n.user_id.clone()).collect();
    let user_ids_vec: Vec<String> = unique_user_ids.into_iter().collect();

    let preferences_map = get_user_notification_preferences_batch(user_ids_vec)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to batch load preferences: {}", e)))?;

    let mut grouped: HashMap<NotifKey, Vec<NotificationWithTimestamp>> = HashMap::new();
    for notif in notif_message {
        let user_id = notif.user_id;
        let notif_type = notif.notif_type;
        let timestamp = notif.timestamp;
        let key = NotifKey {
            user_id: user_id.clone(),
            second: timestamp / 1000,
            r#type: notif_type,
        };

        let preference = preferences_map.get(&user_id).copied().unwrap_or_else(|| {
            tracing::warn!("Preferences not found for user {}, using defaults", user_id);
            crate::utils::structs::NotificationPreferences {
                announcement: true,
                account: true,
                campaign: true,
                transaction: true,
            }
        });

        tracing::debug!(
            "User {} preferences: transaction={}, account={}, announcement={}, campaign={}, checking type={:?}",
            user_id,
            preference.transaction,
            preference.account,
            preference.announcement,
            preference.campaign,
            notif_type
        );

        let message = match notif.metadata.construct_message() {
            Ok(message) => message,
            Err(e) => {
                tracing::warn!("Skipping notification for user {user_id} with error: {e}");
                continue;
            }
        };

        if preference.contains(notif_type) {
            tracing::debug!(
                "Notification type {:?} is enabled for user {}, adding to grouped notifications",
                notif_type,
                user_id
            );
            grouped
                .entry(key)
                .or_default()
                .push(NotificationWithTimestamp { message, timestamp });
        } else {
            tracing::info!(
                "Notification type {:?} is DISABLED for user {}, skipping notification",
                notif_type,
                user_id
            );
        }
    }

    Ok(grouped)
}
