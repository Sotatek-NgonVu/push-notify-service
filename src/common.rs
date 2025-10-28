#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DeserializerType {
    /// MessagePack serialization using serde
    #[default]
    RmpSerde,
    /// JSON serialization using serde
    SerdeJson,
    /// MessagePack serialization using rmp_read
    RmpRead,
}

#[derive(Debug, Clone)]
pub struct MessageWithOffset<T> {
    /// The deserialized message
    pub message: T,
    /// Kafka message offset
    pub offset: i64,
    /// Kafka partition number
    pub partition: i32,
    /// Kafka topic name
    pub topic: String,
}