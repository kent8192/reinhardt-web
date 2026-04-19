//! Kafka backend for reinhardt-streaming.
//!
//! Requires feature `kafka`.

pub mod config;
pub mod consumer;
pub mod producer;

#[cfg(feature = "task-backend")]
pub mod task_backend;

pub use config::KafkaConfig;
pub use consumer::KafkaConsumer;
pub use producer::KafkaProducer;

#[cfg(feature = "task-backend")]
pub use task_backend::KafkaTaskBackend;
