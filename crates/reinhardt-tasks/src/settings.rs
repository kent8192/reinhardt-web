//! Settings fragments for task queues, workers, webhooks, and broker backends.
//!
//! These fragments are the settings-first configuration entry points for the
//! task system. Each maps to a `[tasks_*]` TOML section and can be composed
//! into a project's settings with the `#[settings]` macro. Conversions into the
//! deprecated compatibility `XxxConfig` types are provided for the migration
//! window; new code should prefer the fragments and the
//! `create_*_from_settings` constructors.

#![allow(deprecated)] // Conversions target legacy config types during the compatibility window.

use std::collections::HashMap;
use std::time::Duration;

use reinhardt_core::macros::settings;
use serde::{Deserialize, Serialize};

use crate::queue::{QueueConfig, TaskQueue};
use crate::webhook::{HttpWebhookSender, RetryConfig, WebhookConfig};
use crate::worker::{Worker, WorkerConfig};

// --- defaults -------------------------------------------------------------

fn default_queue_name() -> String {
	"default".to_string()
}
fn default_max_retries() -> u32 {
	3
}
fn default_worker_name() -> String {
	"worker".to_string()
}
fn default_concurrency() -> usize {
	4
}
fn default_poll_interval_ms() -> u64 {
	1000
}
fn default_webhook_method() -> String {
	"POST".to_string()
}
fn default_webhook_timeout_secs() -> u64 {
	5
}
fn default_retry_max_retries() -> u32 {
	3
}
fn default_retry_initial_backoff_ms() -> u64 {
	100
}
fn default_retry_max_backoff_ms() -> u64 {
	30_000
}
fn default_retry_backoff_multiplier() -> f64 {
	2.0
}

// --- queue ----------------------------------------------------------------

/// Task queue settings fragment.
///
/// Maps to the `[tasks_queue]` section.
#[settings(fragment = true, section = "tasks_queue")]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueSettings {
	/// The name of the queue.
	#[serde(default = "default_queue_name")]
	pub name: String,
	/// Maximum number of retry attempts for failed tasks.
	#[serde(default = "default_max_retries")]
	pub max_retries: u32,
}

impl Default for QueueSettings {
	fn default() -> Self {
		Self {
			name: default_queue_name(),
			max_retries: default_max_retries(),
		}
	}
}

impl From<&QueueSettings> for QueueConfig {
	fn from(settings: &QueueSettings) -> Self {
		Self {
			name: settings.name.clone(),
			max_retries: settings.max_retries,
		}
	}
}

/// Build a [`TaskQueue`] from a [`QueueSettings`] fragment.
///
/// Note: [`TaskQueue`] is currently a zero-sized, stateless delegator, so the
/// queue-level settings (`name`, `max_retries`) are not yet retained or applied
/// at runtime. Tracked in reinhardt-web#5067.
pub fn create_queue_from_settings(settings: &QueueSettings) -> TaskQueue {
	TaskQueue::with_config(QueueConfig::from(settings))
}

// --- webhook --------------------------------------------------------------

/// Retry policy value object embedded in [`WebhookSettings`].
///
/// This is not an independently loadable section; it is nested under
/// `[tasks_webhook.retry]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookRetrySettings {
	/// Maximum number of retry attempts.
	#[serde(default = "default_retry_max_retries")]
	pub max_retries: u32,
	/// Initial backoff between retries, in milliseconds.
	#[serde(default = "default_retry_initial_backoff_ms")]
	pub initial_backoff_ms: u64,
	/// Maximum backoff between retries, in milliseconds.
	#[serde(default = "default_retry_max_backoff_ms")]
	pub max_backoff_ms: u64,
	/// Backoff multiplier for exponential backoff.
	#[serde(default = "default_retry_backoff_multiplier")]
	pub backoff_multiplier: f64,
}

impl Default for WebhookRetrySettings {
	fn default() -> Self {
		Self {
			max_retries: default_retry_max_retries(),
			initial_backoff_ms: default_retry_initial_backoff_ms(),
			max_backoff_ms: default_retry_max_backoff_ms(),
			backoff_multiplier: default_retry_backoff_multiplier(),
		}
	}
}

impl From<&WebhookRetrySettings> for RetryConfig {
	fn from(settings: &WebhookRetrySettings) -> Self {
		Self {
			max_retries: settings.max_retries,
			initial_backoff: Duration::from_millis(settings.initial_backoff_ms),
			max_backoff: Duration::from_millis(settings.max_backoff_ms),
			backoff_multiplier: settings.backoff_multiplier,
		}
	}
}

/// Webhook delivery settings fragment.
///
/// Maps to the `[tasks_webhook]` section.
#[settings(fragment = true, section = "tasks_webhook")]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookSettings {
	/// Target URL for webhook delivery.
	#[serde(default)]
	pub url: String,
	/// HTTP method to use.
	#[serde(default = "default_webhook_method")]
	pub method: String,
	/// Additional headers to include with each request.
	#[serde(default)]
	pub headers: HashMap<String, String>,
	/// Request timeout, in seconds.
	#[serde(default = "default_webhook_timeout_secs")]
	pub timeout_secs: u64,
	/// Retry policy.
	#[serde(default)]
	pub retry: WebhookRetrySettings,
}

impl Default for WebhookSettings {
	fn default() -> Self {
		Self {
			url: String::new(),
			method: default_webhook_method(),
			headers: HashMap::new(),
			timeout_secs: default_webhook_timeout_secs(),
			retry: WebhookRetrySettings::default(),
		}
	}
}

impl From<&WebhookSettings> for WebhookConfig {
	fn from(settings: &WebhookSettings) -> Self {
		Self {
			url: settings.url.clone(),
			method: settings.method.clone(),
			headers: settings.headers.clone(),
			timeout: Duration::from_secs(settings.timeout_secs),
			retry_config: RetryConfig::from(&settings.retry),
		}
	}
}

/// Build an [`HttpWebhookSender`] from a [`WebhookSettings`] fragment.
pub fn create_webhook_sender_from_settings(settings: &WebhookSettings) -> HttpWebhookSender {
	HttpWebhookSender::new(WebhookConfig::from(settings))
}

// --- worker ---------------------------------------------------------------

/// Task worker settings fragment.
///
/// Maps to the `[tasks_worker]` section.
#[settings(fragment = true, section = "tasks_worker")]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerSettings {
	/// Name of this worker instance.
	#[serde(default = "default_worker_name")]
	pub name: String,
	/// Number of concurrent task handlers.
	#[serde(default = "default_concurrency")]
	pub concurrency: usize,
	/// How long to wait between queue polls, in milliseconds.
	#[serde(default = "default_poll_interval_ms")]
	pub poll_interval_ms: u64,
	/// Webhook delivery targets for task completion notifications.
	#[serde(default)]
	pub webhooks: Vec<WebhookSettings>,
}

impl Default for WorkerSettings {
	fn default() -> Self {
		Self {
			name: default_worker_name(),
			concurrency: default_concurrency(),
			poll_interval_ms: default_poll_interval_ms(),
			webhooks: Vec::new(),
		}
	}
}

impl From<&WorkerSettings> for WorkerConfig {
	fn from(settings: &WorkerSettings) -> Self {
		Self {
			name: settings.name.clone(),
			concurrency: settings.concurrency,
			poll_interval: Duration::from_millis(settings.poll_interval_ms),
			webhook_configs: settings.webhooks.iter().map(WebhookConfig::from).collect(),
		}
	}
}

/// Build a [`Worker`] from a [`WorkerSettings`] fragment.
pub fn create_worker_from_settings(settings: &WorkerSettings) -> Worker {
	Worker::new(WorkerConfig::from(settings))
}

// --- sqs backend ----------------------------------------------------------

#[cfg(feature = "sqs-backend")]
fn default_sqs_visibility_timeout() -> i32 {
	30
}
#[cfg(feature = "sqs-backend")]
fn default_sqs_max_messages() -> i32 {
	1
}
#[cfg(feature = "sqs-backend")]
fn default_sqs_wait_time_seconds() -> i32 {
	0
}

/// Amazon SQS backend settings fragment.
///
/// Maps to the `[tasks_sqs]` section. Available with the `sqs-backend` feature.
#[cfg(feature = "sqs-backend")]
#[settings(fragment = true, section = "tasks_sqs")]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SqsSettings {
	/// The SQS queue URL.
	#[serde(default)]
	pub queue_url: String,
	/// Message visibility timeout, in seconds.
	#[serde(default = "default_sqs_visibility_timeout")]
	pub visibility_timeout: i32,
	/// Maximum number of messages to receive per poll (capped at 10 by SQS).
	#[serde(default = "default_sqs_max_messages")]
	pub max_messages: i32,
	/// Wait time for long polling, in seconds.
	#[serde(default = "default_sqs_wait_time_seconds")]
	pub wait_time_seconds: i32,
}

#[cfg(feature = "sqs-backend")]
impl Default for SqsSettings {
	fn default() -> Self {
		Self {
			queue_url: String::new(),
			visibility_timeout: default_sqs_visibility_timeout(),
			max_messages: default_sqs_max_messages(),
			wait_time_seconds: default_sqs_wait_time_seconds(),
		}
	}
}

#[cfg(feature = "sqs-backend")]
impl From<&SqsSettings> for crate::backends::sqs::SqsConfig {
	fn from(settings: &SqsSettings) -> Self {
		// SqsConfig fields are private; rebuild through the builder API.
		crate::backends::sqs::SqsConfig::new(settings.queue_url.clone())
			.with_visibility_timeout(settings.visibility_timeout)
			.with_max_messages(settings.max_messages)
			.with_wait_time_seconds(settings.wait_time_seconds)
	}
}

/// Build an [`SqsBackend`](crate::backends::sqs::SqsBackend) from an
/// [`SqsSettings`] fragment.
#[cfg(feature = "sqs-backend")]
pub async fn create_sqs_backend_from_settings(
	settings: &SqsSettings,
) -> Result<crate::backends::sqs::SqsBackend, crate::TaskExecutionError> {
	crate::backends::sqs::SqsBackend::new(crate::backends::sqs::SqsConfig::from(settings)).await
}

// --- rabbitmq backend -----------------------------------------------------

#[cfg(feature = "rabbitmq-backend")]
fn default_rabbitmq_url() -> String {
	"amqp://localhost:5672/%2f".to_string()
}
#[cfg(feature = "rabbitmq-backend")]
fn default_rabbitmq_queue_name() -> String {
	"reinhardt_tasks".to_string()
}
#[cfg(feature = "rabbitmq-backend")]
fn default_rabbitmq_routing_key() -> String {
	"reinhardt_tasks".to_string()
}

/// RabbitMQ backend settings fragment.
///
/// Maps to the `[tasks_rabbitmq]` section. Available with the
/// `rabbitmq-backend` feature.
#[cfg(feature = "rabbitmq-backend")]
#[settings(fragment = true, section = "tasks_rabbitmq")]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RabbitMQSettings {
	/// The AMQP connection URL.
	#[serde(default = "default_rabbitmq_url")]
	pub url: String,
	/// The queue name to publish to.
	#[serde(default = "default_rabbitmq_queue_name")]
	pub queue_name: String,
	/// The exchange name (empty string for the default exchange).
	#[serde(default)]
	pub exchange_name: String,
	/// The routing key.
	#[serde(default = "default_rabbitmq_routing_key")]
	pub routing_key: String,
}

#[cfg(feature = "rabbitmq-backend")]
impl Default for RabbitMQSettings {
	fn default() -> Self {
		Self {
			url: default_rabbitmq_url(),
			queue_name: default_rabbitmq_queue_name(),
			exchange_name: String::new(),
			routing_key: default_rabbitmq_routing_key(),
		}
	}
}

#[cfg(feature = "rabbitmq-backend")]
impl From<&RabbitMQSettings> for crate::backends::rabbitmq::RabbitMQConfig {
	fn from(settings: &RabbitMQSettings) -> Self {
		Self {
			url: settings.url.clone(),
			queue_name: settings.queue_name.clone(),
			exchange_name: settings.exchange_name.clone(),
			routing_key: settings.routing_key.clone(),
		}
	}
}

/// Build a [`RabbitMQBackend`](crate::backends::rabbitmq::RabbitMQBackend) from
/// a [`RabbitMQSettings`] fragment.
#[cfg(feature = "rabbitmq-backend")]
pub async fn create_rabbitmq_backend_from_settings(
	settings: &RabbitMQSettings,
) -> Result<crate::backends::rabbitmq::RabbitMQBackend, lapin::Error> {
	crate::backends::rabbitmq::RabbitMQBackend::new(
		crate::backends::rabbitmq::RabbitMQConfig::from(settings),
	)
	.await
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_conf::settings::fragment::SettingsFragment;

	#[rstest::rstest]
	fn section_names_are_crate_prefixed() {
		// Arrange / Act / Assert
		assert_eq!(QueueSettings::section(), "tasks_queue");
		assert_eq!(WorkerSettings::section(), "tasks_worker");
		assert_eq!(WebhookSettings::section(), "tasks_webhook");
	}

	#[rstest::rstest]
	fn queue_settings_default_converts_to_config() {
		// Arrange
		let settings = QueueSettings::default();

		// Act
		let config = QueueConfig::from(&settings);

		// Assert
		assert_eq!(config.name, "default");
		assert_eq!(config.max_retries, 3);
	}

	#[rstest::rstest]
	fn worker_settings_convert_milliseconds_to_duration() {
		// Arrange
		let settings = WorkerSettings {
			name: "ingest".to_string(),
			concurrency: 8,
			poll_interval_ms: 2500,
			webhooks: Vec::new(),
		};

		// Act
		let config = WorkerConfig::from(&settings);

		// Assert
		assert_eq!(config.name, "ingest");
		assert_eq!(config.concurrency, 8);
		assert_eq!(config.poll_interval, Duration::from_millis(2500));
		assert!(config.webhook_configs.is_empty());
	}

	#[rstest::rstest]
	fn webhook_settings_convert_seconds_and_nested_retry() {
		// Arrange
		let settings = WebhookSettings::default();

		// Act
		let config = WebhookConfig::from(&settings);

		// Assert
		assert_eq!(config.method, "POST");
		assert_eq!(config.timeout, Duration::from_secs(5));
		assert_eq!(config.retry_config.max_retries, 3);
		assert_eq!(
			config.retry_config.initial_backoff,
			Duration::from_millis(100)
		);
		assert_eq!(config.retry_config.max_backoff, Duration::from_secs(30));
		assert_eq!(config.retry_config.backoff_multiplier, 2.0);
	}

	#[rstest::rstest]
	fn worker_settings_map_nested_webhooks() {
		// Arrange
		let settings = WorkerSettings {
			name: "w".to_string(),
			concurrency: 1,
			poll_interval_ms: 100,
			webhooks: vec![WebhookSettings {
				url: "https://example.com/hook".to_string(),
				..WebhookSettings::default()
			}],
		};

		// Act
		let config = WorkerConfig::from(&settings);

		// Assert
		assert_eq!(config.webhook_configs.len(), 1);
		assert_eq!(config.webhook_configs[0].url, "https://example.com/hook");
	}

	#[rstest::rstest]
	fn webhook_settings_deserialize_with_defaults() {
		// Arrange — only `url` is provided; everything else falls back to defaults.
		let json = r#"{ "url": "https://example.com/hook", "timeout_secs": 10 }"#;

		// Act
		let settings: WebhookSettings = serde_json::from_str(json).unwrap();
		let config = WebhookConfig::from(&settings);

		// Assert
		assert_eq!(config.url, "https://example.com/hook");
		assert_eq!(config.method, "POST");
		assert_eq!(config.timeout, Duration::from_secs(10));
		assert_eq!(config.retry_config.max_retries, 3);
	}

	#[cfg(feature = "sqs-backend")]
	#[rstest::rstest]
	fn sqs_settings_default_converts_to_config() {
		// Arrange
		let settings = SqsSettings {
			queue_url: "https://sqs.example.com/q".to_string(),
			..SqsSettings::default()
		};

		// Act
		let config = crate::backends::sqs::SqsConfig::from(&settings);

		// Assert — round-trips through the builder, which caps max_messages at 10.
		assert!(format!("{config:?}").contains("https://sqs.example.com/q"));
	}

	#[cfg(feature = "rabbitmq-backend")]
	#[rstest::rstest]
	fn rabbitmq_settings_default_converts_to_config() {
		// Arrange
		let settings = RabbitMQSettings::default();

		// Act
		let config = crate::backends::rabbitmq::RabbitMQConfig::from(&settings);

		// Assert
		assert_eq!(config.queue_name, "reinhardt_tasks");
		assert_eq!(config.routing_key, "reinhardt_tasks");
	}
}
