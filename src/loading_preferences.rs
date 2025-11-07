use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use crate::models::user_notification_settings::UserNotificationSetting;
use wither::bson::doc;
use crate::utils::models::ModelExt;
use crate::utils::structs::NotificationPreferences;
use crate::core::cache::redis_service::RedisService;

static USER_NOTIF_PREFERENCES: LazyLock<RwLock<HashMap<String, NotificationPreferences>>> =
    LazyLock::new(|| RwLock::new(HashMap::with_capacity(100_000)));

const REDIS_PREFERENCE_KEY_PREFIX: &str = "raidenx:user:notification:preferences";
const PREFERENCE_CACHE_TTL: usize = 3600; // 1 hour

fn get_redis_preference_key(user_id: &str) -> String {
    format!("{}:{}", REDIS_PREFERENCE_KEY_PREFIX, user_id)
}

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
    let redis_key = get_redis_preference_key(&user_id);
    let redis_service = RedisService::new().await;

    match redis_service.get_cache_opt::<NotificationPreferences>(&redis_key).await {
        Ok(Some(preferences)) => {
            tracing::debug!("Found preferences in Redis cache for user {}", user_id);
            let mut map = USER_NOTIF_PREFERENCES.write().await;
            map.insert(user_id.clone(), preferences);
            return Ok(*map.get(&user_id).unwrap());
        }
        Ok(None) => {
            tracing::debug!("Preferences not found in Redis cache for user {}, querying DB", user_id);
        }
        Err(e) => {
            tracing::warn!("Failed to get preferences from Redis for user {}: {e}, falling back to DB", user_id);
        }
    }

    {
        let map = USER_NOTIF_PREFERENCES.read().await;
        if let Some(preferences) = map.get(&user_id) {
            tracing::debug!("Found preferences in memory cache for user {}", user_id);
            let redis_key_clone = redis_key.clone();
            let preferences_clone = *preferences;
            tokio::spawn(async move {
                if let Err(e) = redis_service.set_ex_cache(&redis_key_clone, &preferences_clone, PREFERENCE_CACHE_TTL).await {
                    tracing::warn!("Failed to update Redis cache for user preferences: {e}");
                }
            });
            return Ok(*preferences);
        }
    }

    let query = doc! { "userId": user_id.clone() };
    let setting_opt = UserNotificationSetting::find_one(query, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user notification preference: {}", e))?;

    let preferences = if let Some(setting) = setting_opt {
        NotificationPreferences {
            announcement: setting.announcement,
            account: setting.account,
            campaign: setting.campaign,
            transaction: setting.transaction,
        }
    } else {
        NotificationPreferences {
            announcement: true,
            account: true,
            campaign: true,
            transaction: true,
        }
    };

    if let Err(e) = redis_service.set_ex_cache(&redis_key, &preferences, PREFERENCE_CACHE_TTL).await {
        tracing::warn!("Failed to update Redis cache for user {} preferences: {e}", user_id);
    }

    let mut map = USER_NOTIF_PREFERENCES.write().await;
    map.insert(user_id.clone(), preferences);
    let result = *map.get(&user_id).unwrap();

    tracing::debug!("Loaded preferences from DB for user {} and updated cache", user_id);
    Ok(result)
}

pub async fn update_user_notification_preferences(
    user_id: String,
    setting: NotificationPreferences,
) -> anyhow::Result<()> {
    let preferences = NotificationPreferences {
        announcement: setting.announcement,
        account: setting.account,
        campaign: setting.campaign,
        transaction: setting.transaction,
    };

    let redis_key = get_redis_preference_key(&user_id);
    let redis_service = RedisService::new().await;
    if let Err(e) = redis_service.set_ex_cache(&redis_key, &preferences, PREFERENCE_CACHE_TTL).await {
        tracing::warn!("Failed to update Redis cache for user {} preferences: {e}", user_id);
    }

    let mut map = USER_NOTIF_PREFERENCES.write().await;
    map.insert(user_id, preferences);

    Ok(())
}
