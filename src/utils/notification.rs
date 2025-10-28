use std::collections::HashMap;
use crate::loading_preferences::{get_user_notification_preferences};
use crate::utils::structs::{NotifMessage, NotifType};
use crate::errors::Error;

#[derive(Eq, Hash, PartialEq, Debug)]
pub struct NotifKey {
    pub user_id: String,
    pub second: i64,
    pub r#type: NotifType,
}

pub async fn group_by_user_id(
    mut notif_message: Vec<NotifMessage>,
) -> Result<HashMap<NotifKey, Vec<String>>, Error> {
    notif_message.sort_by_key(|m| m.timestamp);

    let mut grouped: HashMap<NotifKey, Vec<String>> = HashMap::new();
    for notif in notif_message {
        let user_id = notif.user_id;
        let notif_type = notif.notif_type;
        let key = NotifKey {
            user_id: user_id.clone(),
            second: notif.timestamp / 1000,
            r#type: notif_type,
        };
        let preference = match get_user_notification_preferences(user_id.clone()).await {
            Ok(preference) => preference,
            Err(e) => {
                tracing::warn!("Failed to get notification preferences for user {user_id}: {e}");
                continue;
            }
        };

        let message = match notif.metadata.construct_message() {
            Ok(message) => message,
            Err(e) => {
                tracing::warn!("Skipping notification for user {user_id} with error: {e}");
                continue;
            }
        };

        if preference.contains(notif_type) {
            grouped.entry(key).or_default().push(message);
        }
    }

    Ok(grouped)
}