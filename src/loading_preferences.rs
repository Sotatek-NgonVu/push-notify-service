use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use crate::models::user_notification_settings::UserNotificationSetting;
use wither::bson::doc;
use crate::utils::models::ModelExt;
use crate::utils::structs::NotificationPreferences;

static USER_NOTIF_PREFERENCES: LazyLock<RwLock<HashMap<String, NotificationPreferences>>> =
    LazyLock::new(|| RwLock::new(HashMap::with_capacity(100_000)));

pub async fn load_user_notification_preferences() -> anyhow::Result<()> {
    let mut map = USER_NOTIF_PREFERENCES.write().await;
    map.clear();

    let query = doc! {};

    let user_settings = UserNotificationSetting::find(query, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user notification settings: {}", e))?;

    for setting in user_settings {
        let preferences = NotificationPreferences {
            announcement: setting.announcement,
            account: setting.account,
            campaign: setting.campaign,
            transaction: setting.transaction,
        };

        map.insert(setting.user_id.clone(), preferences);
    }

    if map.is_empty() {
        tracing::warn!("No user notification preferences found in the database");
        return Err(anyhow::anyhow!("No user notification preferences found"));
    }

    tracing::info!("Successfully loaded user notification preferences");
    Ok(())
}

pub async fn get_user_notification_preferences(
    user_id: String,
) -> anyhow::Result<NotificationPreferences> {
    {
        let map = USER_NOTIF_PREFERENCES.read().await;

        if let Some(preferences) = map.get(&user_id) {
            return Ok(*preferences);
        }
    }

    // Fetch from DB if not present in cache
    let query = doc! { "userId": user_id.to_string() };
    let setting_opt = UserNotificationSetting::find_one(query, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user notification preference: {}", e))?;

    if let Some(setting) = setting_opt {
        let mut map = USER_NOTIF_PREFERENCES.write().await;

        let preferences = NotificationPreferences {
            announcement: setting.announcement,
            account: setting.account,
            campaign: setting.campaign,
            transaction: setting.transaction,
        };

        map.insert(user_id, preferences);
        Ok(preferences)
    } else {
        let mut map = USER_NOTIF_PREFERENCES.write().await;

        let preferences = NotificationPreferences {
            announcement: true,
            account: true,
            campaign: true,
            transaction: true,
        };

        map.insert(user_id, preferences);
        Ok(preferences)
    }
}

pub async fn update_user_notification_preferences(
    user_id: String,
    setting: NotificationPreferences,
) -> anyhow::Result<()> {
    let mut map = USER_NOTIF_PREFERENCES.write().await;

    let preferences = NotificationPreferences {
        announcement: setting.announcement,
        account: setting.account,
        campaign: setting.campaign,
        transaction: setting.transaction,
    };
    map.insert(user_id, preferences);

    Ok(())
}
