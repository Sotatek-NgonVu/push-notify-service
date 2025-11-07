use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use push_notify_service::common::{DeserializerType, MessageWithOffset};
use push_notify_service::config::{APP_CONFIG, KafkaConfig};
use push_notify_service::core::cache::redis_service::RedisService;
use push_notify_service::core::kafka_service::consumers::streams::{
    KafkaStreamConsumer, KafkaStreamConsumerExt,
};
use push_notify_service::core::kafka_service::producer::setup_kafka_producer;
use push_notify_service::enums::KafkaTopic;
use push_notify_service::errors::Error;
use push_notify_service::loading_fcm_token::{get_user_fcm_tokens, preload_user_fcm_tokens};
use push_notify_service::loading_preferences::load_user_notification_preferences;
use push_notify_service::utils::notification::{
    NotifKey, NotificationWithTimestamp, group_by_user_id,
};
use push_notify_service::utils::structs::{NotifMessage, OrderNotifBuilder};
use push_notify_service::utils::tracing::init_standard_tracing;
use tokio_retry::{Retry, strategy::ExponentialBackoff};

const NOTIFICATION_KEY_PREFIX: &str = "raidenx:notification";
const RATE_LIMIT_DURATION: usize = 2; // 2 second
const UNSENT_COUNT_DURATION: usize = 60 * 60 * 24; // 24 hours

static FCM_CLIENT: LazyLock<fcm_notification::FcmNotification> = LazyLock::new(|| {
    fcm_notification::FcmNotification::new(APP_CONFIG.firebase_credentials_path.as_str())
        .unwrap_or_else(|e| {
            tracing::error!("Failed to create FCM client: {e}");
            std::process::exit(1);
        })
});

const FCM_RETRY_ATTEMPTS: usize = 3;
const FCM_RETRY_INITIAL_DELAY_MS: u64 = 100;
const FCM_SEND_CONCURRENCY: usize = 8;

struct SendJob {
    token: String,
    title: Arc<String>,
    body: Arc<String>,
    user_id: String,
}

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

    NotificationPublishConsumer::run_single_vec_message(&kafka_config, DeserializerType::RmpSerde)
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
        let user_notifications = group_by_user_id(messages).await?;

        let total_grouped = user_notifications.values().map(|v| v.len()).sum::<usize>();
        tracing::info!(
            "Processing {} grouped notifications (some may have been skipped due to user preferences)",
            total_grouped
        );

        process(user_notifications).await?;

        tracing::info!("Batch processing completed - offset will be committed");

        Ok(())
    }
}

async fn process(
    grouped_notifications: HashMap<NotifKey, Vec<NotificationWithTimestamp>>,
) -> Result<(), Error> {
    if grouped_notifications.is_empty() {
        tracing::info!(
            "No notifications to send (all were skipped due to user preferences), but offset will still be committed"
        );
        return Ok(());
    }

    for (key, notifications) in grouped_notifications {
        let title = key.r#type.construct_title();

        let notif_data = notifications
            .iter()
            .map(|notif| OrderNotifBuilder {
                user_id: key.user_id.clone(),
                message: notif.message.clone(),
            })
            .collect::<Vec<OrderNotifBuilder>>();

        if let Some(last_notif) = notif_data.last() {
            if let Err(e) = push_notification_to_firebase(title, last_notif).await {
                tracing::error!("Failed to push notification for user {}: {e}", key.user_id);
            }
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

    let mut send_jobs = Vec::new();

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

            send_jobs.push(SendJob {
                token: token.clone(),
                title: Arc::new(title_str),
                body: Arc::new(message_str),
                user_id: notif.user_id.clone(),
            });
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

    if send_jobs.is_empty() {
        return Ok(());
    }


    let results = stream::iter(send_jobs.into_iter().map(send_job))
        .buffer_unordered(FCM_SEND_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    for (user_id, result) in results {
        if let Err(e) = result {
            tracing::error!(
                "Failed to send notification to user ID {} after retries: {}",
                user_id,
                e
            );
        }
    }

    Ok(())
}

async fn send_job(job: SendJob) -> (String, Result<(), Error>) {
    let SendJob {
        token,
        title,
        body,
        user_id,
    } = job;

    let retry_strategy = ExponentialBackoff::from_millis(FCM_RETRY_INITIAL_DELAY_MS)
        .max_delay(Duration::from_secs(5))
        .take(FCM_RETRY_ATTEMPTS);

    let token_arc = Arc::new(token.clone());
    let title_arc = Arc::clone(&title);
    let body_arc = Arc::clone(&body);

    let send_result = Retry::spawn(retry_strategy, {
        let token_arc = Arc::clone(&token_arc);
        let title_arc = Arc::clone(&title_arc);
        let body_arc = Arc::clone(&body_arc);
        move || {
            let token = Arc::clone(&token_arc);
            let title = Arc::clone(&title_arc);
            let body = Arc::clone(&body_arc);
            async move {
                let notification = fcm_notification::NotificationPayload {
                    token: token.as_str(),
                    title: title.as_ref(),
                    body: body.as_ref(),
                    data: None,
                };
                FCM_CLIENT.send_notification(&notification).await
            }
        }
    })
    .await;

    match send_result {
        Ok(_) => {
            if let Err(e) = update_last_sent(token.clone()).await {
                tracing::warn!("Failed to update rate limit for user ID {}: {e}", user_id);
            }
            if let Err(e) = reset_unsent_count(token.clone()).await {
                tracing::warn!(
                    "Failed to reset unsent count notification for user ID {}: {e}",
                    user_id
                );
            }

            tracing::info!("Notification sent successfully for user ID {}", user_id);
            (user_id, Ok(()))
        }
        Err(e) => {
            (
                user_id,
                Err(Error::internal_err(&format!(
                    "FCM send failed after {} retries: {}",
                    FCM_RETRY_ATTEMPTS, e
                ))),
            )
        }
    }
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
