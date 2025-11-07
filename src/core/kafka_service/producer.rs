use crate::config::KafkaConfig;
use anyhow::Error;
use futures::StreamExt;
use once_cell::sync::OnceCell;
use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use rmp_serde::Serializer;
use serde::Serialize;
use std::time::Duration;
use tokio::time::Instant;

pub static KAFKA_PRODUCER: OnceCell<FutureProducer> = OnceCell::new();

pub async fn setup_kafka_producer(kafka_config: &KafkaConfig) -> Result<(), Error> {
    let mut binding = ClientConfig::new();
    let config = binding
        .set("bootstrap.servers", kafka_config.kafka_brokers.clone())
        .set("message.timeout.ms", "5000")
        .set(
            "enable.idempotence",
            kafka_config.enable_idempotence.to_string().as_str(),
        )
        .set("acks", "all")
        .set("retries", "5");

    let is_ssl_enabled = kafka_config.kafka_ssl_enabled;
    // If SSL is enabled, set the SSL options
    if is_ssl_enabled {
        config.set("security.protocol", "SASL_SSL");

        let sasl_username = kafka_config.kafka_sasl_username.clone();
        let sasl_password = kafka_config.kafka_sasl_password.clone();
        if !sasl_username.is_empty() && !sasl_password.is_empty() {
            config
                .set("sasl.mechanism", "PLAIN")
                .set("sasl.username", &sasl_username)
                .set("sasl.password", &sasl_password);
        }
    }

    let producer = config.create().expect("Producer creation error");
    KAFKA_PRODUCER.set(producer).unwrap_or_else(|_| {
        panic!("Kafka producer already initialized");
    });

    Ok(())
}

pub fn get_kafka_producer() -> &'static FutureProducer {
    match KAFKA_PRODUCER.get() {
        Some(producer) => producer,
        None => {
            println!("Kafka producer not initialized");
            std::process::exit(1);
        }
    }
}

pub async fn publish_kafka_messages<T: Serialize + Clone>(
    topic: &str,
    kafka_records: Vec<T>,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let results = futures::stream::iter(kafka_records.clone())
        .map(|record| {
            let payload = serde_json::to_string(&record).expect("Failed to serialize record");
            async move {
                let record: FutureRecord<'_, String, String> =
                    FutureRecord::to(topic).payload(&payload);
                get_kafka_producer()
                    .send(record, Duration::from_secs(0))
                    .await
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in results {
        if let Err(err) = result {
            println!("Error sending to Kafka: {:?}", err);
            return Err(anyhow::anyhow!("Error sending to Kafka: {:?}", err));
        }
    }

    println!(
        "Published {} messages to topic {:?} took {:?}",
        kafka_records.len(),
        topic,
        start.elapsed()
    );

    Ok(())
}

pub async fn publish_kafka_messages_with_key<T: Serialize + Clone>(
    topic: &str,
    key: &str,
    kafka_records: Vec<T>,
) -> anyhow::Result<()> {
    let results = futures::stream::iter(kafka_records.clone())
        .map(|record| {
            let payload = serde_json::to_string(&record).expect("Failed to serialize record");
            async move {
                let key_str = key.to_string();
                let record: FutureRecord<'_, String, String> =
                    FutureRecord::to(topic).key(&key_str).payload(&payload);
                get_kafka_producer()
                    .send(record, Duration::from_secs(0))
                    .await
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in results {
        if let Err(err) = result {
            println!("Error sending to Kafka: {:?}", err);
            return Err(anyhow::anyhow!("Error sending to Kafka: {:?}", err));
        }
    }

    Ok(())
}

pub async fn publish_kafka_messages_to_partition<T: Serialize + Clone>(
    topic: &str,
    partition: i32,
    kafka_records: Vec<T>,
    custom_producer: Option<&FutureProducer>,
) -> anyhow::Result<()> {
    let start = Instant::now();
    let producer = match custom_producer {
        Some(p) => p,
        None => get_kafka_producer(),
    };

    let results = futures::stream::iter(kafka_records.clone())
        .map(|record| {
            let payload = serde_json::to_string(&record).expect("Failed to serialize record");
            async move {
                let record: FutureRecord<'_, String, String> = FutureRecord::to(topic)
                    .partition(partition)
                    .payload(&payload);
                producer.send(record, Duration::from_secs(100)).await
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in results {
        if let Err(err) = result {
            println!("Error sending to Kafka: {:?}", err);
            return Err(anyhow::anyhow!("Error sending to Kafka: {:?}", err));
        }
    }

    println!(
        "Published {} messages to topic {:?} took {:?}",
        kafka_records.len(),
        topic,
        start.elapsed()
    );

    Ok(())
}

pub async fn publish_kafka_rmp_messages<T: Serialize + Clone>(
    topic: &str,
    key: Option<&str>,
    kafka_records: Vec<T>,
    custom_producer: Option<&FutureProducer>,
) -> anyhow::Result<()> {
    let producer = match custom_producer {
        Some(p) => p,
        None => get_kafka_producer(),
    };

    let results = futures::stream::iter(kafka_records.clone())
        .map(|record| {
            let mut message = Vec::new();
            let _ = record.serialize(&mut Serializer::new(&mut message));
            let producer = producer.clone();
            async move {
                let record = FutureRecord::<'_, str, [u8]>::to(topic)
                    .key(key.unwrap_or(""))
                    .payload(&message);
                producer.send(record, Duration::from_secs(0)).await
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in results {
        if let Err(err) = result {
            println!("Error sending to Kafka: {:?}", err);
            return Err(anyhow::anyhow!("Error sending to Kafka: {:?}", err));
        }
    }

    Ok(())
}

pub async fn publish_kafka_rmp_messages_to_partition<T: Serialize + Clone>(
    topic: &str,
    partition: i32,
    kafka_records: Vec<T>,
    custom_producer: Option<&FutureProducer>,
) -> anyhow::Result<()> {
    let producer = match custom_producer {
        Some(p) => p,
        None => get_kafka_producer(),
    };

    let results = futures::stream::iter(kafka_records.clone())
        .map(|record| {
            let mut message = Vec::new();
            let _ = record.serialize(&mut Serializer::new(&mut message));
            let producer = producer.clone();
            async move {
                let record = FutureRecord::<'_, str, [u8]>::to(topic)
                    .partition(partition)
                    .payload(&message);
                producer.send(record, Duration::from_secs(100)).await
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in results {
        if let Err(err) = result {
            println!("Error sending to Kafka: {:?}", err);
            return Err(anyhow::anyhow!("Error sending to Kafka: {:?}", err));
        }
    }

    Ok(())
}
