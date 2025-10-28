use std::collections::HashMap;

use async_trait::async_trait;
use push_notify_service::common::{DeserializerType, MessageWithOffset};
use push_notify_service::config::{KafkaConfig, APP_CONFIG};
use push_notify_service::core::cache::redis_emitter::setup_redis_emitter;
use push_notify_service::core::kafka_service::consumers::streams::{KafkaStreamConsumer, KafkaStreamConsumerExt};
use push_notify_service::core::kafka_service::producer::setup_kafka_producer;
use push_notify_service::enums::KafkaTopic;
use push_notify_service::loading_preferences::load_user_notification_preferences;
use push_notify_service::models::user_notifications::UserNotification;
use push_notify_service::utils::structs::{NotifMessage, NotifType};
use push_notify_service::utils::tracing::init_standard_tracing;
use push_notify_service::utils::models::ModelExt;
use push_notify_service::utils::notification::{group_by_user_id, NotifKey};
use push_notify_service::errors::Error;
use wither::bson::DateTime;

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

    NotificationPersistConsumer::run_single_vec_message(&kafka_config, DeserializerType::RmpSerde)
        .await?;

    Ok(())
}

pub struct NotificationPersistConsumer;

#[async_trait]
impl KafkaStreamConsumer<NotifMessage> for NotificationPersistConsumer {
    fn topic() -> String {
        KafkaTopic::UserNotificationPersister.to_string()
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

        process(user_notifications).await?;

        Ok(())
    }
}

async fn process(grouped_notifications: HashMap<NotifKey, Vec<String>>) -> Result<(), Error> {
    let now = DateTime::now();

    for (key, notifications) in grouped_notifications {
        let r#type = key.r#type.to_string();
        let title = key.r#type.construct_title();

        match key.r#type {
            NotifType::Order => {
                // For Order notifications, only save the last message
                if let Some(last_message) = notifications.last() {
                    let notification = UserNotification {
                        id: None,
                        r#type: r#type.clone(),
                        user_id: key.user_id.clone(),
                        title,
                        message: last_message.clone(),
                        created_at: now,
                        updated_at: now,
                        is_read: false,
                    };

                    match UserNotification::create(notification).await {
                        Ok(_) => {
                            tracing::info!(
                                "Successfully persisted Order notification for user_id={}",
                                key.user_id
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to persist Order notification for user_id={}: {e}",
                                key.user_id
                            );
                            continue;
                        }
                    }
                }
            }
            NotifType::Transaction | NotifType::Account => {
                // For Transaction/Account, save all notifications
                for message in notifications {
                    let notification = UserNotification {
                        id: None,
                        r#type: r#type.clone(),
                        user_id: key.user_id.clone(),
                        title: title.clone(),
                        message,
                        created_at: now,
                        updated_at: now,
                        is_read: false,
                    };

                    if let Err(e) = UserNotification::create(notification).await {
                        tracing::error!(
                            "Failed to persist notification for user_id={}: {e}",
                            key.user_id
                        );
                        continue;
                    }
                }

                tracing::info!(
                    "Successfully persisted batch of notifications for user_id={}",
                    key.user_id
                );
            }
            _ => {
                tracing::warn!("Unsupported notification type: {}", key.r#type);
                continue;
            }
        };
    }

    Ok(())
}
