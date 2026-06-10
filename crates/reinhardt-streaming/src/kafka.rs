//! Kafka backend for reinhardt-streaming.
//!
//! Requires feature `kafka`.

/// Kafka connection configuration.
pub mod config;
/// Kafka consumer wrapper.
pub mod consumer;
/// Kafka producer wrapper.
pub mod producer;

pub use config::KafkaConfig;
pub use consumer::KafkaConsumer;
pub use producer::KafkaProducer;
