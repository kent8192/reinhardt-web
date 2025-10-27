//! Task backend implementations

#[cfg(feature = "redis-backend")]
pub mod redis;

#[cfg(feature = "database-backend")]
pub mod sqlite;

#[cfg(feature = "sqs-backend")]
pub mod sqs;

#[cfg(feature = "rabbitmq-backend")]
pub mod rabbitmq;

#[cfg(feature = "redis-backend")]
pub use redis::RedisBackend;

#[cfg(feature = "database-backend")]
pub use sqlite::SqliteBackend;

#[cfg(feature = "sqs-backend")]
pub use sqs::{SqsBackend, SqsConfig};

#[cfg(feature = "rabbitmq-backend")]
pub use rabbitmq::{RabbitMQBackend, RabbitMQConfig};
