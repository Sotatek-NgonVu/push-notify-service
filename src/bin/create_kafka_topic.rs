use anyhow::Result;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use push_notify_service::config::APP_CONFIG;
use push_notify_service::enums::KafkaTopic;

fn get_number_of_user_partitions() -> i32 {
    std::env::var("KAFKA_NUMBER_OF_USER_PARTITIONS")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1)
}

fn get_kafka_replication_factor() -> i32 {
    std::env::var("KAFKA_REPLICATION_FACTOR")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    println!("========== Creating kafka topics for notification ==========");
    create_kafka_topics_for_notification().await?;

    Ok(())
}

async fn create_kafka_topics_for_notification() -> Result<()> {
    // Create admin client
    let admin_client: AdminClient<_> = ClientConfig::new()
        .set("bootstrap.servers", &APP_CONFIG.kafka_brokers)
        .create()?;

    let topics = [
        KafkaTopic::UserNotificationPersister.to_string(),
        KafkaTopic::UserNotificationPublisher.to_string(),
    ];

    let new_topics: Vec<NewTopic> = topics
        .iter()
        .map(|topic| {
            NewTopic::new(
                topic,
                get_number_of_user_partitions(),
                TopicReplication::Fixed(get_kafka_replication_factor()),
            )
                .set("retention.ms", "2592000000") // 30 days
                .set("cleanup.policy", "delete")
        })
        .collect();

    let results = admin_client
        .create_topics(&new_topics, &AdminOptions::new())
        .await?;

    for result in results {
        match result {
            Ok(topic) => println!("Successfully created topic: {topic}"),
            Err((topic, e)) => {
                if e.to_string().contains("already exists") {
                    println!("Topic {topic} already exists, skipping...");
                } else {
                    println!("Error creating topic {topic}: {e}");
                }
            }
        }
    }

    Ok(())
}
