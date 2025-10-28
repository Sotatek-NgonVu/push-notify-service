use anyhow::{Context, Result};
use async_trait::async_trait;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message, Offset, TopicPartitionList,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::Debug,
    time::{Duration, Instant},
};
use tokio::time::timeout;
use tracing::{error, info};
use crate::common::{DeserializerType, MessageWithOffset};
use crate::config::KafkaConfig;
// ===== Configuration Types =====

#[derive(Debug, Clone, Default)]
pub struct BaseConsumerConfig {
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub deserializer: DeserializerType,
    pub session_timeout_ms: u64,
    pub auto_commit: bool,
    pub partition_eof: bool,
    pub enable_debug: bool,
}

impl BaseConsumerConfig {
    pub fn validate(&self) -> Result<()> {
        if self.batch_size == 0 {
            return Err(anyhow::anyhow!("batch_size must be greater than 0"));
        }
        if self.batch_timeout_ms == 0 {
            return Err(anyhow::anyhow!("batch_timeout_ms must be greater than 0"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConsumerOffset {
    pub partition: i32,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub enum AutoOffsetReset {
    Earliest,
    Latest,
    None,
}

impl Default for AutoOffsetReset {
    fn default() -> Self {
        AutoOffsetReset::Latest
    }
}

impl AutoOffsetReset {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutoOffsetReset::Earliest => "earliest",
            AutoOffsetReset::Latest => "latest",
            AutoOffsetReset::None => "none",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StreamConsumerConfig {
    pub base: BaseConsumerConfig,
    pub offset_config: Option<ConsumerOffset>,
    pub auto_offset_reset: AutoOffsetReset,
}

impl StreamConsumerConfig {
    pub fn validate(&self) -> Result<()> {
        self.base.validate()?;
        Ok(())
    }
}

// ===== Stream Consumer Builder =====

pub struct StreamConsumerBuilder {
    config: StreamConsumerConfig,
}

impl Default for StreamConsumerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamConsumerBuilder {
    pub fn new() -> Self {
        Self {
            config: StreamConsumerConfig {
                base: BaseConsumerConfig {
                    batch_size: 100,
                    batch_timeout_ms: 5000,
                    deserializer: DeserializerType::RmpSerde,
                    session_timeout_ms: 30000,
                    auto_commit: false,
                    partition_eof: false,
                    enable_debug: false,
                },
                offset_config: None,
                auto_offset_reset: AutoOffsetReset::Latest,
            },
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.base.batch_size = size;
        self
    }

    pub fn batch_timeout_ms(mut self, timeout: u64) -> Self {
        self.config.base.batch_timeout_ms = timeout;
        self
    }

    pub fn deserializer(mut self, deserializer: DeserializerType) -> Self {
        self.config.base.deserializer = deserializer;
        self
    }

    pub fn offset_config(mut self, offset: ConsumerOffset) -> Self {
        self.config.offset_config = Some(offset);
        self
    }

    pub fn session_timeout_ms(mut self, timeout: u64) -> Self {
        self.config.base.session_timeout_ms = timeout;
        self
    }

    pub fn auto_commit(mut self, enable: bool) -> Self {
        self.config.base.auto_commit = enable;
        self
    }

    pub fn auto_offset_reset(mut self, strategy: AutoOffsetReset) -> Self {
        self.config.auto_offset_reset = strategy;
        self
    }

    pub fn partition_eof(mut self, enable: bool) -> Self {
        self.config.base.partition_eof = enable;
        self
    }

    pub fn enable_debug(mut self, enable: bool) -> Self {
        self.config.base.enable_debug = enable;
        self
    }

    pub fn build(self) -> StreamConsumerConfig {
        self.config
    }
}

const DEFAULT_BATCH_SIZE: usize = 100;

// ===== Message Handler =====

pub struct MessageHandler {
    deserializer: DeserializerType,
}

impl MessageHandler {
    pub fn new(deserializer: DeserializerType) -> Self {
        Self { deserializer }
    }

    pub fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        match self.deserializer {
            DeserializerType::RmpSerde => {
                rmp_serde::from_slice(bytes).context("Failed to deserialize RmpSerde")
            }
            DeserializerType::SerdeJson => {
                serde_json::from_slice(bytes).context("Failed to deserialize JSON")
            }
            DeserializerType::RmpRead => {
                rmp_serde::from_slice(bytes).context("Failed to deserialize RmpRead")
            }
        }
    }
}

// ===== Helper Functions =====

pub fn handle_message_payload<T: DeserializeOwned + Debug>(
    msg: &rdkafka::message::BorrowedMessage,
    handler: &MessageHandler,
) -> Result<MessageWithOffset<T>> {
    let payload = msg
        .payload()
        .context("Message payload is empty")?;

    let message: T = handler.deserialize(payload)?;

    Ok(MessageWithOffset {
        message,
        offset: msg.offset(),
        partition: msg.partition(),
        topic: msg.topic().to_string(),
    })
}

pub fn handle_vector_message_payload<T: DeserializeOwned + Debug>(
    msg: &rdkafka::message::BorrowedMessage,
    handler: &MessageHandler,
) -> Result<MessageWithOffset<Vec<T>>> {
    let payload = msg
        .payload()
        .context("Message payload is empty")?;

    let messages: Vec<T> = handler.deserialize(payload)?;

    Ok(MessageWithOffset {
        message: messages,
        offset: msg.offset(),
        partition: msg.partition(),
        topic: msg.topic().to_string(),
    })
}

pub async fn process_batch_with_early_return<T, F, Fut>(
    batch: Vec<MessageWithOffset<T>>,
    handler: F,
) -> Result<()>
where
    F: Fn(Vec<MessageWithOffset<T>>) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    handler(batch).await
}

// ===== Main Consumer Trait =====

#[async_trait]
pub trait KafkaStreamConsumer<T>
where
    T: Debug + Serialize + DeserializeOwned + Send + 'static,
{
    /// Returns the topic name to consume from
    fn topic() -> String;

    /// Returns the default batch size for this consumer
    fn default_batch_size() -> usize {
        DEFAULT_BATCH_SIZE
    }

    /// Creates a Kafka consumer with the given configuration
    fn create_consumer(
        kafka_config: &KafkaConfig,
        consumer_config: &StreamConsumerConfig,
    ) -> Result<StreamConsumer> {
        let mut config = ClientConfig::new();

        // Basic configuration
        config
            .set("group.id", kafka_config.kafka_group_id.clone())
            .set("bootstrap.servers", kafka_config.kafka_brokers.clone())
            .set(
                "enable.partition.eof",
                consumer_config.base.partition_eof.to_string(),
            )
            .set(
                "session.timeout.ms",
                consumer_config.base.session_timeout_ms.to_string(),
            )
            .set(
                "enable.auto.commit",
                consumer_config.base.auto_commit.to_string(),
            )
            .set(
                "auto.offset.reset",
                consumer_config.auto_offset_reset.as_str(),
            );

        if consumer_config.base.enable_debug {
            tracing::debug!("Enabling Kafka debug mode");
            config.set("debug", "consumer,cgrp,topic,fetch");
        }

        // SSL configuration
        if kafka_config.kafka_ssl_enabled {
            Self::configure_ssl(&mut config, kafka_config)?;
        }

        let consumer: StreamConsumer =
            config.create().context("Failed to create Kafka consumer")?;

        // Topic subscription or partition assignment
        Self::configure_topic_subscription(&consumer, consumer_config.offset_config.clone())?;

        Ok(consumer)
    }

    /// Configures SSL settings for the consumer
    fn configure_ssl(config: &mut ClientConfig, kafka_config: &KafkaConfig) -> Result<()> {
        config.set("security.protocol", "SASL_SSL");

        let sasl_username = kafka_config.kafka_sasl_username.clone();
        let sasl_password = kafka_config.kafka_sasl_password.clone();

        if !sasl_username.is_empty() && !sasl_password.is_empty() {
            config
                .set("sasl.mechanism", "PLAIN")
                .set("sasl.username", &sasl_username)
                .set("sasl.password", &sasl_password);
        }

        Ok(())
    }

    /// Configures topic subscription or partition assignment
    fn configure_topic_subscription(
        consumer: &StreamConsumer,
        offset_config: Option<ConsumerOffset>,
    ) -> Result<()> {
        if let Some(offset_config) = offset_config {
            let mut tpl = TopicPartitionList::new();

            match offset_config.offset {
                Some(offset) => {
                    tpl.add_partition_offset(
                        &Self::topic(),
                        offset_config.partition,
                        Offset::Offset(offset),
                    )
                    .context("Failed to add partition offset")?;
                }
                None => {
                    tpl.add_partition(&Self::topic(), offset_config.partition);
                }
            }

            consumer
                .assign(&tpl)
                .context("Failed to assign partitions with offsets")?;

            info!(
                "Assigned topic {} with custom offset: {:?}",
                Self::topic(),
                offset_config
            );
            return Ok(());
        }

        consumer
            .subscribe(&[&Self::topic()])
            .context("Failed to subscribe to topic")?;

        Ok(())
    }

    fn print_received_message(msg: &rdkafka::message::BorrowedMessage) {
        info!(
            topic = %msg.topic(),
            partition = msg.partition(),
            offset = msg.offset(),
            "Received message"
        );
    }

    /// Commits offset for a single message
    fn commit_single_offset(
        consumer: &StreamConsumer,
        msg: &rdkafka::message::BorrowedMessage,
    ) -> Result<()> {
        consumer
            .commit_message(msg, rdkafka::consumer::CommitMode::Async)
            .context("Failed to commit single message offset")?;

        info!(
            "Committed single message offset: {} for partition {}",
            msg.offset(),
            msg.partition()
        );

        Ok(())
    }

    /// Commits offset for a specific message with offset information
    fn commit_specific_offset(
        consumer: &StreamConsumer,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Result<()> {
        use rdkafka::consumer::CommitMode;
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(offset + 1))?;
        consumer
            .commit(&tpl, CommitMode::Async)
            .context("Failed to commit specific offset")?;

        info!(
            "Committed specific offset: {} for partition {}",
            offset + 1,
            partition
        );

        Ok(())
    }

    /// Starts consuming messages one by one
    async fn start_single_consumer(
        kafka_config: &KafkaConfig,
        config: StreamConsumerConfig,
    ) -> Result<()> {
        config.validate()?;
        let consumer = Self::create_consumer(kafka_config, &config)?;
        let handler = MessageHandler::new(config.base.deserializer);

        info!("Started single consumer for topic: {}", Self::topic());

        loop {
            let msg = consumer.recv().await.context("Failed to receive message")?;
            Self::print_received_message(&msg);

            let message_with_offset =
                handle_message_payload(&msg, &handler).context("Failed to deserialize message")?;

            match Self::handle_single_message(message_with_offset).await {
                Ok(_) => {
                    Self::commit_single_offset(&consumer, &msg)?;
                }
                Err(e) => {
                    error!(error = ?e, "Error processing single message, not committing offset");
                    return Err(e.context("Failed to process message"));
                }
            }
        }
    }

    /// Starts consuming messages in batches
    async fn start_batch_consumer(
        kafka_config: &KafkaConfig,
        config: StreamConsumerConfig,
    ) -> Result<()> {
        config.validate()?;
        let consumer = Self::create_consumer(kafka_config, &config)?;
        let handler = MessageHandler::new(config.base.deserializer);

        info!("Started batch consumer for topic: {}", Self::topic());

        let batch_timeout = Duration::from_millis(config.base.batch_timeout_ms);
        let batch_size = config.base.batch_size;
        let mut batch = Vec::with_capacity(batch_size);

        loop {
            let batch_start_time = Instant::now();

            while batch.len() < batch_size {
                let elapsed = batch_start_time.elapsed();

                if elapsed >= batch_timeout {
                    break;
                }

                let remaining_time = batch_timeout - elapsed;

                match timeout(remaining_time, consumer.recv()).await {
                    Ok(Ok(msg)) => {
                        Self::print_received_message(&msg);

                        let message_with_offset = handle_message_payload(&msg, &handler)
                            .context("Failed to deserialize message")?;

                        batch.push(message_with_offset);
                    }
                    Ok(Err(e)) => {
                        error!(error = ?e, "Error receiving message");
                        return Err(e.into());
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            if !batch.is_empty() {
                let batch_size = batch.len();
                let total_time = batch_start_time.elapsed();

                let last_message_info = batch
                    .last()
                    .map(|msg| (msg.topic.clone(), msg.partition, msg.offset));

                match Self::process_batch(&mut batch).await {
                    Ok(_) => {
                        if let Some((topic, partition, offset)) = last_message_info {
                            Self::commit_specific_offset(&consumer, &topic, partition, offset)?;
                        }
                        info!(
                            "Processed batch of {} messages in {:?}",
                            batch_size, total_time
                        );
                        batch.clear();
                    }
                    Err(e) => {
                        error!(error = ?e, "Error processing batch");
                        return Err(e.context("Failed to process batch"));
                    }
                }
            }
        }
    }

    /// Starts consuming vector messages in batches
    async fn start_batch_vec_consumer(
        kafka_config: &KafkaConfig,
        config: StreamConsumerConfig,
    ) -> Result<()> {
        config.validate()?;
        let consumer = Self::create_consumer(kafka_config, &config)?;
        let handler = MessageHandler::new(config.base.deserializer);

        info!("Started batch vec consumer for topic: {}", Self::topic());

        let batch_timeout = Duration::from_millis(config.base.batch_timeout_ms);
        let batch_size = config.base.batch_size;
        let mut batch = Vec::with_capacity(batch_size);

        loop {
            let batch_start_time = Instant::now();

            while batch.len() < batch_size {
                let elapsed = batch_start_time.elapsed();

                if elapsed >= batch_timeout {
                    break;
                }

                let remaining_time = batch_timeout - elapsed;

                match timeout(remaining_time, consumer.recv()).await {
                    Ok(Ok(msg)) => {
                        Self::print_received_message(&msg);

                        let message_with_offset = handle_vector_message_payload(&msg, &handler)
                            .context("Failed to deserialize vector message")?;

                        batch.push(message_with_offset);
                    }
                    Ok(Err(e)) => {
                        error!(error = ?e, "Error receiving message");
                        return Err(e.into());
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            if !batch.is_empty() {
                let batch_size = batch.len();
                let total_time = batch_start_time.elapsed();

                let last_message_info = batch
                    .last()
                    .map(|msg| (msg.topic.clone(), msg.partition, msg.offset));

                match Self::process_batch_vec(&mut batch).await {
                    Ok(_) => {
                        if let Some((topic, partition, offset)) = last_message_info {
                            Self::commit_specific_offset(&consumer, &topic, partition, offset)?;
                        }
                        info!(
                            "Processed batch of {} vec messages in {:?}",
                            batch_size, total_time
                        );
                        batch.clear();
                    }
                    Err(e) => {
                        error!(error = ?e, "Error processing batch vec");
                        return Err(e.context("Failed to process batch vec"));
                    }
                }
            }
        }
    }

    /// Starts consuming vector messages (when payload contains Vec<T>)
    async fn start_vector_consumer(
        kafka_config: &KafkaConfig,
        config: StreamConsumerConfig,
    ) -> Result<()> {
        config.validate()?;
        let consumer = Self::create_consumer(kafka_config, &config)?;
        let handler = MessageHandler::new(config.base.deserializer);

        info!("Started vector consumer for topic: {}", Self::topic());

        loop {
            let msg = consumer.recv().await.context("Failed to receive message")?;
            Self::print_received_message(&msg);

            let message_with_offset = handle_vector_message_payload(&msg, &handler)
                .context("Failed to deserialize vector message")?;

            match Self::handle_single_vector_message(message_with_offset).await {
                Ok(_) => {
                    Self::commit_single_offset(&consumer, &msg)?;
                }
                Err(e) => {
                    error!(error = ?e, "Error processing vector message, not committing offset");
                    return Err(e.context("Failed to process vector message"));
                }
            }
        }
    }

    /// Processes a batch of messages
    async fn process_batch(batch: &mut Vec<MessageWithOffset<T>>) -> Result<()> {
        process_batch_with_early_return(std::mem::take(batch), |batch_with_offset| {
            Box::pin(Self::handle_batch(batch_with_offset))
        })
        .await
    }

    /// Processes a batch of vector messages
    async fn process_batch_vec(batch: &mut Vec<MessageWithOffset<Vec<T>>>) -> Result<()> {
        process_batch_with_early_return(std::mem::take(batch), |batch_with_offset| {
            Box::pin(Self::handle_batch_vec(batch_with_offset))
        })
        .await
    }

    // ===== Handler methods to be implemented by consumers =====

    /// Handles a single message
    async fn handle_single_message(_message: MessageWithOffset<T>) -> Result<()> {
        Err(anyhow::anyhow!("handle_single_message not implemented"))
    }

    /// Handles a single vector message (when payload contains Vec<T>)
    async fn handle_single_vector_message(_messages: MessageWithOffset<Vec<T>>) -> Result<()> {
        Err(anyhow::anyhow!(
            "handle_single_vector_message not implemented"
        ))
    }

    /// Handles a batch of messages
    async fn handle_batch(_batch: Vec<MessageWithOffset<T>>) -> Result<()> {
        Err(anyhow::anyhow!("handle_batch not implemented"))
    }

    /// Handles a batch of vector messages
    async fn handle_batch_vec(_batch: Vec<MessageWithOffset<Vec<T>>>) -> Result<()> {
        Err(anyhow::anyhow!("handle_batch_vec not implemented"))
    }
}

// ===== Extension Trait =====

#[async_trait]
pub trait KafkaStreamConsumerExt<T>: KafkaStreamConsumer<T>
where
    T: Debug + Serialize + DeserializeOwned + Send + 'static,
{
    fn create_consumer_config(deserializer: DeserializerType) -> StreamConsumerConfig {
        StreamConsumerBuilder::new()
            .deserializer(deserializer)
            .build()
    }

    async fn run_single_message(
        kafka_config: &KafkaConfig,
        deserializer: DeserializerType,
    ) -> Result<()> {
        let config = Self::create_consumer_config(deserializer);
        Self::start_single_consumer(kafka_config, config).await
    }

    async fn run_single_vec_message(
        kafka_config: &KafkaConfig,
        deserializer: DeserializerType,
    ) -> Result<()> {
        let config = Self::create_consumer_config(deserializer);
        Self::start_vector_consumer(kafka_config, config).await
    }

    async fn run_single_vec_message_with_config(
        kafka_config: &KafkaConfig,
        deserializer: DeserializerType,
        auto_offset_reset: AutoOffsetReset,
    ) -> Result<()> {
        let config = StreamConsumerBuilder::new()
            .deserializer(deserializer)
            .auto_offset_reset(auto_offset_reset)
            .build();
        Self::start_vector_consumer(kafka_config, config).await
    }

    async fn run_batch_message(
        kafka_config: &KafkaConfig,
        deserializer: DeserializerType,
    ) -> Result<()> {
        let config = Self::create_consumer_config(deserializer);
        Self::start_batch_consumer(kafka_config, config).await
    }

    async fn run_batch_vec_message(
        kafka_config: &KafkaConfig,
        deserializer: DeserializerType,
    ) -> Result<()> {
        let config = Self::create_consumer_config(deserializer);
        Self::start_batch_vec_consumer(kafka_config, config).await
    }
}

// ===== Blanket Implementation =====

impl<T, U> KafkaStreamConsumerExt<T> for U
where
    T: Debug + Serialize + DeserializeOwned + Send + 'static,
    U: KafkaStreamConsumer<T>,
{
}
