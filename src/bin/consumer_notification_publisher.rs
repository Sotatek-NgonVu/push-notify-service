use std::collections::HashMap;
use std::sync::LazyLock;

use async_trait::async_trait;
use push_notify_service::common::{DeserializerType, MessageWithOffset};
use push_notify_service::config::{KafkaConfig, APP_CONFIG};
use push_notify_service::core::cache::redis_emitter::{get_redis_emitter, setup_redis_emitter};
use push_notify_service::core::cache::redis_service::RedisService;
use push_notify_service::core::kafka_service::consumers::streams::{KafkaStreamConsumer, KafkaStreamConsumerExt};
use push_notify_service::core::kafka_service::producer::setup_kafka_producer;
use push_notify_service::enums::KafkaTopic;
use push_notify_service::loading_fcm_token::{get_user_fcm_tokens, preload_user_fcm_tokens, update_fcm_token_in_memory, UpdateFcmToken, UPDATE_FCM_TOKEN_CHANNEL};
use push_notify_service::loading_preferences::load_user_notification_preferences;
use push_notify_service::utils::structs::{NotifMessage, OrderNotifBuilder};
use push_notify_service::utils::tracing::init_standard_tracing;
use push_notify_service::errors::Error;
use push_notify_service::utils::notification::{group_by_user_id, NotifKey};

const NOTIFICATION_KEY_PREFIX: &str = "raidenx:notification";
const RATE_LIMIT_DURATION: usize = 2; // 2 second
const UNSENT_COUNT_DURATION: usize = 60 * 60 * 24; // 24 hours

static FCM_CLIENT: LazyLock<fcm_notification::FcmNotification> = LazyLock::new(|| {
    fcm_notification::FcmNotification::new(APP_CONFIG.firebase_credentials_path.as_str())
        .expect("Failed to create FCM client.")
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_standard_tracing(env!("CARGO_CRATE_NAME"));

    let kafka_config = KafkaConfig {
        kafka_group_id: APP_CONFIG.kafka_group_id.clone(),
        kafka_brokers: APP_CONFIG.kafka_brokers.clone(),
        kafka_ssl_enabled: APP_CONFIG.kafka_ssl_enabled,
        kafka_sasl_username: APP_CONFIG.kafka_sasl_username.clone(),
        kafka_sasl_password: APP_CONFIG.kafka_sasl_password.clone(),
        enable_idempotence: APP_CONFIG.enable_idempotence,
    };

    setup_kafka_producer(&kafka_config).await?;
    setup_redis_emitter(&APP_CONFIG.redis_url).await?;

    if let Err(e) = load_user_notification_preferences().await {
        tracing::warn!(
            "Failed to load user notification preferences: {e}. Return empty preferences."
        );
    } else {
        tracing::info!("User notification preferences loaded successfully.");
    };

    if let Err(e) = preload_user_fcm_tokens().await {
        tracing::warn!("Failed to preload user FCM tokens: {e}");
    } else {
        tracing::info!("User FCM tokens preloaded successfully.");
    }

    // let redis_emitter = get_redis_emitter();
    // tokio::spawn(async move {
    //     if let Err(e) = redis_emitter
    //         .subscribe::<UpdateFcmToken>(&UPDATE_FCM_TOKEN_CHANNEL, move |message| {
    //             Box::pin(async move { update_fcm_token_in_memory(message).await })
    //         })
    //         .await {
    //         tracing::error!("Failed to subscribe Redis channel: {e}.");
    //         std::process::exit(1);
    //     }
    //
    //     tracing::info!("Redis channel subscribed successfully.");
    // });

    NotificationPublishConsumer::run_single_vec_message(
        &kafka_config,
        DeserializerType::RmpSerde,
    )
        .await?;

    Ok(())
}

pub struct NotificationPublishConsumer;

#[async_trait]
impl KafkaStreamConsumer<NotifMessage> for NotificationPublishConsumer {
    fn topic() -> String {
        KafkaTopic::UserNotificationPublisher.to_string()
    }

    async fn handle_single_vector_message(
        payload: MessageWithOffset<Vec<NotifMessage>>,
    ) -> anyhow::Result<()> {
        let messages = payload.message;

        if messages.is_empty() {
            tracing::warn!("Received empty notification batch, skipping processing.");
            return Ok(());
        }

        tracing::info!("Received {} notifications from Kafka topic", messages.len());

        // Group the notifications by user_id
        let user_notifications = group_by_user_id(messages).await?;

        tracing::info!("Processing {} grouped notifications", user_notifications.len());

        process(user_notifications).await?;

        tracing::info!("Batch processing completed");

        Ok(())
    }
}

async fn process(grouped_notifications: HashMap<NotifKey, Vec<String>>) -> Result<(), Error> {
    for (key, notifications) in grouped_notifications {
        let title = key.r#type.construct_title();

        let notif_data = notifications
            .iter()
            .map(|notif| OrderNotifBuilder {
                user_id: key.user_id.clone(),
                message: notif.to_string(),
            })
            .collect::<Vec<OrderNotifBuilder>>();

        if !notif_data.is_empty() {
            let last_notif = notif_data.last().unwrap();
            push_notification_to_firebase(title, last_notif).await?;
        }
    }

    Ok(())
}

async fn push_notification_to_firebase(
    title: String,
    notif: &OrderNotifBuilder,
) -> Result<(), Error> {
    let tokens = get_user_fcm_tokens(notif.user_id.clone()).await?;
    if tokens.is_empty() {
        tracing::warn!(
            "No FCM tokens found for user ID {}. Skipping notification.",
            notif.user_id
        );
        return Ok(());
    }

    for token in tokens {
        let unsent_count = get_unsent_notification_count(token.clone())
            .await
            .unwrap_or(0);

        if can_send_notification(token.clone()).await {
            let (title_str, message_str) = if unsent_count > 1 {
                let title = "You have many notifications".to_string();
                let message = format!(
                    "You have {unsent_count} unread notifications. Please check your app.",
                );
                (title, message)
            } else {
                (title.clone(), notif.message.clone())
            };

            let notification = fcm_notification::NotificationPayload {
                token: token.as_str(),
                title: title_str.as_str(),
                body: message_str.as_str(),
                data: None,
            };

            match FCM_CLIENT.send_notification(&notification).await {
                Ok(_) => {
                    if let Err(e) = update_last_sent(token.clone()).await {
                        tracing::warn!(
                            "Failed to update rate limit for user ID {}: {e}",
                            notif.user_id
                        );
                    }
                    if let Err(e) = reset_unsent_count(token.clone()).await {
                        tracing::warn!(
                            "Failed to reset unsent count notification for user ID {}: {e}",
                            notif.user_id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to send notification to user ID {}: {e}",
                        notif.user_id
                    );
                }
            }

            tracing::info!("Notification sent for user ID {}", notif.user_id);
        } else {
            if let Err(e) = increment_unsent_count(token.clone()).await {
                tracing::warn!(
                    "Failed to update unsent count notification for user ID {}: {e}",
                    notif.user_id
                );
            };
            tracing::warn!(
                "Skipping notification for user ID {} due to rate limiting.",
                notif.user_id
            );
        }
    }

    Ok(())
}

async fn can_send_notification(device_token: String) -> bool {
    let redis_service = RedisService::new().await;
    let last_sent = redis_service
        .get_cache::<Option<i64>>(
            format!("{}:{}:last_sent", NOTIFICATION_KEY_PREFIX, device_token).as_str(),
        )
        .await
        .unwrap_or_default();

    match last_sent {
        Some(timestamp) => {
            let current_time = chrono::Utc::now().timestamp_millis();
            let elapsed_time = current_time - timestamp;
            elapsed_time >= (RATE_LIMIT_DURATION as i64 * 1000)
        }
        None => true,
    }
}

async fn update_last_sent(device_token: String) -> Result<(), Error> {
    let redis_service = RedisService::new().await;

    let key = format!("{}:{}:last_sent", NOTIFICATION_KEY_PREFIX, device_token);
    let current_time = chrono::Utc::now().timestamp_millis();

    redis_service
        .set_ex_cache(key.as_str(), &current_time, RATE_LIMIT_DURATION)
        .await
        .map_err(|e| {
            Error::internal_err(&format!("Failed to update unsent notification count: {e}"))
        })?;

    Ok(())
}

async fn increment_unsent_count(device_token: String) -> Result<(), Error> {
    let current_count = get_unsent_notification_count(device_token.clone())
        .await
        .unwrap_or(0);
    let new_count = current_count + 1;

    let redis_service = RedisService::new().await;
    let key = format!("{}:{}:unsent_count", NOTIFICATION_KEY_PREFIX, device_token);

    redis_service
        .set_ex_cache(&key, &new_count, UNSENT_COUNT_DURATION)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to increment unsent count: {e}")))?;

    Ok(())
}

async fn reset_unsent_count(device_token: String) -> Result<(), Error> {
    let redis_service = RedisService::new().await;
    let key = format!("{}:{}:unsent_count", NOTIFICATION_KEY_PREFIX, device_token);

    redis_service
        .set_ex_cache(&key, &0i64, UNSENT_COUNT_DURATION)
        .await
        .map_err(|e| Error::internal_err(&format!("Failed to reset unsent count: {e}")))?;

    Ok(())
}

async fn get_unsent_notification_count(device_token: String) -> Option<i64> {
    let redis_service = RedisService::new().await;
    let key = format!("{}:{}:unsent_count", NOTIFICATION_KEY_PREFIX, device_token);

    redis_service
        .get_cache::<Option<i64>>(key.as_str())
        .await
        .unwrap_or_default()
}
