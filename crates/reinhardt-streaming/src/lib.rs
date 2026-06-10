//! Backend-agnostic streaming abstraction for reinhardt.
//!
//! # Features
//!
//! - `kafka` — Kafka backend using rskafka (pure Rust, no C bindings)
//!
//! The Kafka-backed `TaskBackend` lives in `reinhardt-tasks` behind its
//! `kafka-backend` feature (not here).
//!
//! # Direct API
//!
//! ```rust,ignore
//! use reinhardt_streaming::kafka::{KafkaConfig, KafkaProducer};
//!
//! let config = KafkaConfig::new(vec!["localhost:9092"]);
//! let producer = KafkaProducer::connect(&config).await?;
//! producer.send("orders", &order).await?;
//! ```

#![warn(missing_docs)]

/// Backend trait abstraction for streaming providers.
pub mod backend;
/// Error types returned by streaming operations.
pub mod error;
/// In-memory streaming backend for tests and local execution.
pub mod in_memory;
/// Message envelope types used by streaming backends.
pub mod message;

#[cfg(feature = "kafka")]
/// Kafka producer and consumer integration.
pub mod kafka;

#[cfg(feature = "kafka")]
/// Dependency-injection bindings for Kafka streaming.
pub mod di;

#[cfg(feature = "kafka")]
/// Process-global Kafka producer registration.
pub mod global;

#[cfg(feature = "kafka")]
pub use global::{global_producer, set_global_producer};

/// Streaming DSL helper macros.
pub mod macros;
/// Router registration types for streaming handlers.
pub mod router;
pub use router::{
	ConsumerFactory, StreamingHandlerKind, StreamingHandlerRegistration, StreamingRouter,
};

/// Type-safe streaming topic resolver trait.
///
/// Mirrored from `reinhardt_urls::StreamingTopicResolver` to avoid requiring
/// reinhardt-urls as a direct dependency of reinhardt-streaming.
pub trait StreamingTopicResolver {
	/// Resolve a registered logical handler name to its backend topic name.
	fn resolve_topic(&self, name: &str) -> &'static str;
}

pub use backend::StreamingBackend;
pub use error::StreamingError;
pub use in_memory::InMemoryStreamingBackend;
pub use message::Message;

/// Metadata about a streaming handler, submitted to inventory by `#[producer]`/`#[consumer]`.
///
/// Used by `resolve_streaming_topic()` to look up topic names at runtime.
#[derive(Debug)]
pub struct StreamingHandlerMetadata {
	/// Logical handler name used for runtime lookup.
	pub name: &'static str,
	/// Backend topic name associated with the handler.
	pub topic: &'static str,
	/// Whether the handler produces or consumes messages.
	pub kind: StreamingHandlerKind,
	/// Consumer group name for consumers.
	pub group: Option<&'static str>,
	/// Rust module path where the handler was registered.
	pub module_path: &'static str,
}

inventory::collect!(StreamingHandlerMetadata);

/// Resolve the Kafka topic name for a streaming handler by its registered `name`.
///
/// Scans the inventory of `StreamingHandlerMetadata` entries submitted by
/// `#[producer]`/`#[consumer]` macros.
///
/// # Panics
///
/// Panics if no handler with `name` is registered.
pub fn resolve_streaming_topic(name: &str) -> &'static str {
	for meta in inventory::iter::<StreamingHandlerMetadata> {
		if meta.name == name {
			return meta.topic;
		}
	}
	panic!(
		"Streaming handler `{name}` not registered. Ensure the function is annotated with `#[producer]` or `#[consumer]`."
	);
}
