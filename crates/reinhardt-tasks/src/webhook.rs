//! Webhook notifications for task completion events
//!
//! This module provides webhook notification support for the Reinhardt tasks system.
//! Webhooks are HTTP callbacks that are triggered when tasks complete, fail, or are cancelled.
//!
//! # Features
//!
//! - HTTP webhook sender with configurable retry logic
//! - Exponential backoff with jitter for failed requests
//! - Configurable timeout and max retries
//! - Automatic serialization of task events to JSON
//!
//! # Example
//!
//! ```rust
//! use reinhardt_tasks::webhook::{
//!     WebhookConfig, RetryConfig, HttpWebhookSender, WebhookSender, WebhookEvent, TaskStatus
//! };
//! use std::time::Duration;
//! use std::collections::HashMap;
//! use chrono::Utc;
//! use reinhardt_tasks::TaskId;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure webhook with retry logic
//! let retry_config = RetryConfig {
//!     max_retries: 3,
//!     initial_backoff: Duration::from_millis(100),
//!     max_backoff: Duration::from_secs(10),
//!     backoff_multiplier: 2.0,
//! };
//!
//! let config = WebhookConfig {
//!     url: "https://example.com/webhook".to_string(),
//!     method: "POST".to_string(),
//!     headers: HashMap::new(),
//!     timeout: Duration::from_secs(5),
//!     retry_config,
//! };
//!
//! let sender = HttpWebhookSender::new(config);
//!
//! // Create and send event
//! let now = Utc::now();
//! let event = WebhookEvent {
//!     task_id: TaskId::new(),
//!     task_name: "example_task".to_string(),
//!     status: TaskStatus::Success,
//!     result: Some("Task completed".to_string()),
//!     error: None,
//!     started_at: now - chrono::Duration::seconds(10),
//!     completed_at: now,
//!     duration_ms: 10000,
//! };
//!
//! sender.send(&event).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

use crate::TaskId;

/// Webhook-related errors
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::WebhookError;
///
/// let error = WebhookError::RequestFailed("Network timeout".to_string());
/// assert_eq!(error.to_string(), "Webhook request failed: Network timeout");
/// ```
#[derive(Debug, Error)]
pub enum WebhookError {
	/// HTTP request failed
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::webhook::WebhookError;
	///
	/// let error = WebhookError::RequestFailed("Connection refused".to_string());
	/// ```
	#[error("Webhook request failed: {0}")]
	RequestFailed(String),

	/// Max retries exceeded
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::webhook::WebhookError;
	///
	/// let error = WebhookError::MaxRetriesExceeded;
	/// assert_eq!(error.to_string(), "Max retries exceeded for webhook");
	/// ```
	#[error("Max retries exceeded for webhook")]
	MaxRetriesExceeded,

	/// Serialization error
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::webhook::WebhookError;
	///
	/// let error = WebhookError::SerializationError("Invalid JSON".to_string());
	/// ```
	#[error("Webhook serialization error: {0}")]
	SerializationError(String),
}

/// Task status for webhook events
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::TaskStatus;
///
/// let status = TaskStatus::Success;
/// assert_eq!(status, TaskStatus::Success);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
	/// Task completed successfully
	Success,
	/// Task failed with an error
	Failed,
	/// Task was cancelled
	Cancelled,
}

/// Webhook event payload
///
/// Contains all information about a completed task for webhook notification.
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::{WebhookEvent, TaskStatus};
/// use reinhardt_tasks::TaskId;
/// use chrono::Utc;
///
/// let now = Utc::now();
/// let event = WebhookEvent {
///     task_id: TaskId::new(),
///     task_name: "send_email".to_string(),
///     status: TaskStatus::Success,
///     result: Some("Email sent".to_string()),
///     error: None,
///     started_at: now - chrono::Duration::seconds(5),
///     completed_at: now,
///     duration_ms: 5000,
/// };
///
/// assert_eq!(event.status, TaskStatus::Success);
/// assert_eq!(event.duration_ms, 5000);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
	/// Unique task identifier
	pub task_id: TaskId,
	/// Task name
	pub task_name: String,
	/// Task completion status
	pub status: TaskStatus,
	/// Task result (if successful)
	pub result: Option<String>,
	/// Error message (if failed)
	pub error: Option<String>,
	/// Task start time
	pub started_at: DateTime<Utc>,
	/// Task completion time
	pub completed_at: DateTime<Utc>,
	/// Task duration in milliseconds
	pub duration_ms: u64,
}

/// Retry configuration for webhook requests
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::RetryConfig;
/// use std::time::Duration;
///
/// let config = RetryConfig {
///     max_retries: 3,
///     initial_backoff: Duration::from_millis(100),
///     max_backoff: Duration::from_secs(10),
///     backoff_multiplier: 2.0,
/// };
///
/// assert_eq!(config.max_retries, 3);
/// assert_eq!(config.backoff_multiplier, 2.0);
/// ```
#[derive(Debug, Clone)]
pub struct RetryConfig {
	/// Maximum number of retry attempts
	pub max_retries: u32,
	/// Initial backoff duration
	pub initial_backoff: Duration,
	/// Maximum backoff duration
	pub max_backoff: Duration,
	/// Backoff multiplier for exponential backoff
	pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
	fn default() -> Self {
		Self {
			max_retries: 3,
			initial_backoff: Duration::from_millis(100),
			max_backoff: Duration::from_secs(30),
			backoff_multiplier: 2.0,
		}
	}
}

/// Webhook configuration
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::{WebhookConfig, RetryConfig};
/// use std::time::Duration;
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("Authorization".to_string(), "Bearer token123".to_string());
///
/// let config = WebhookConfig {
///     url: "https://api.example.com/webhooks".to_string(),
///     method: "POST".to_string(),
///     headers,
///     timeout: Duration::from_secs(5),
///     retry_config: RetryConfig::default(),
/// };
///
/// assert_eq!(config.url, "https://api.example.com/webhooks");
/// assert_eq!(config.timeout, Duration::from_secs(5));
/// ```
#[derive(Debug, Clone)]
pub struct WebhookConfig {
	/// Webhook URL
	pub url: String,
	/// HTTP method (e.g., "POST", "PUT")
	pub method: String,
	/// Additional HTTP headers
	pub headers: HashMap<String, String>,
	/// Request timeout
	pub timeout: Duration,
	/// Retry configuration
	pub retry_config: RetryConfig,
}

impl Default for WebhookConfig {
	fn default() -> Self {
		Self {
			url: String::new(),
			method: "POST".to_string(),
			headers: HashMap::new(),
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig::default(),
		}
	}
}

/// Trait for webhook senders
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_tasks::webhook::{WebhookSender, WebhookEvent, TaskStatus, HttpWebhookSender, WebhookConfig};
/// use reinhardt_tasks::TaskId;
/// use chrono::Utc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sender = HttpWebhookSender::new(WebhookConfig::default());
///
/// let now = Utc::now();
/// let event = WebhookEvent {
///     task_id: TaskId::new(),
///     task_name: "test_task".to_string(),
///     status: TaskStatus::Success,
///     result: Some("OK".to_string()),
///     error: None,
///     started_at: now,
///     completed_at: now,
///     duration_ms: 0,
/// };
///
/// sender.send(&event).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait WebhookSender: Send + Sync {
	/// Send a webhook event
	async fn send(&self, event: &WebhookEvent) -> Result<(), WebhookError>;
}

/// HTTP webhook sender with retry logic
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::webhook::{HttpWebhookSender, WebhookConfig, RetryConfig};
/// use std::time::Duration;
///
/// let config = WebhookConfig {
///     url: "https://example.com/webhook".to_string(),
///     method: "POST".to_string(),
///     headers: Default::default(),
///     timeout: Duration::from_secs(5),
///     retry_config: RetryConfig::default(),
/// };
///
/// let sender = HttpWebhookSender::new(config);
/// ```
pub struct HttpWebhookSender {
	client: reqwest::Client,
	config: WebhookConfig,
}

impl HttpWebhookSender {
	/// Create a new HTTP webhook sender
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::webhook::{HttpWebhookSender, WebhookConfig};
	///
	/// let sender = HttpWebhookSender::new(WebhookConfig::default());
	/// ```
	pub fn new(config: WebhookConfig) -> Self {
		let client = reqwest::Client::builder()
			.timeout(config.timeout)
			.build()
			.unwrap_or_else(|_| reqwest::Client::new());

		Self { client, config }
	}

	/// Calculate backoff duration with exponential backoff and jitter
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::webhook::{HttpWebhookSender, WebhookConfig, RetryConfig};
	/// use std::time::Duration;
	///
	/// let config = WebhookConfig {
	///     url: "https://example.com".to_string(),
	///     method: "POST".to_string(),
	///     headers: Default::default(),
	///     timeout: Duration::from_secs(5),
	///     retry_config: RetryConfig::default(),
	/// };
	///
	/// let sender = HttpWebhookSender::new(config);
	/// let backoff = sender.calculate_backoff(2);
	/// assert!(backoff > Duration::from_millis(0));
	/// ```
	pub fn calculate_backoff(&self, retry_count: u32) -> Duration {
		let retry_config = &self.config.retry_config;

		// Calculate base backoff with exponential growth
		let backoff_ms = retry_config.initial_backoff.as_millis() as f64
			* retry_config.backoff_multiplier.powi(retry_count as i32);

		// Add jitter (±25%)
		let mut rng = rand::rng();
		let jitter = rng.random_range(-0.25..=0.25);
		let backoff_with_jitter = backoff_ms * (1.0 + jitter);

		// Cap at max backoff (AFTER jitter)
		let capped_backoff = backoff_with_jitter.min(retry_config.max_backoff.as_millis() as f64);

		Duration::from_millis(capped_backoff.max(0.0) as u64)
	}

	/// Send webhook request with retry logic
	async fn send_with_retry(&self, event: &WebhookEvent) -> Result<(), WebhookError> {
		let mut retry_count = 0;
		let max_retries = self.config.retry_config.max_retries;

		loop {
			match self.send_request(event).await {
				Ok(_) => return Ok(()),
				Err(e) => {
					if retry_count >= max_retries {
						return Err(WebhookError::MaxRetriesExceeded);
					}

					let backoff = self.calculate_backoff(retry_count);
					eprintln!(
						"Webhook request failed (attempt {}/{}): {}. Retrying in {:?}",
						retry_count + 1,
						max_retries + 1,
						e,
						backoff
					);

					retry_count += 1;
				}
			}
		}
	}

	/// Send a single webhook request
	async fn send_request(&self, event: &WebhookEvent) -> Result<(), WebhookError> {
		let json_body = serde_json::to_string(event)
			.map_err(|e| WebhookError::SerializationError(e.to_string()))?;

		let mut request = match self.config.method.to_uppercase().as_str() {
			"POST" => self.client.post(&self.config.url),
			"PUT" => self.client.put(&self.config.url),
			"PATCH" => self.client.patch(&self.config.url),
			_ => self.client.post(&self.config.url),
		};

		// Add headers
		for (key, value) in &self.config.headers {
			request = request.header(key, value);
		}

		// Send request
		let response = request
			.header("Content-Type", "application/json")
			.body(json_body)
			.send()
			.await
			.map_err(|e| WebhookError::RequestFailed(e.to_string()))?;

		// Check response status
		if !response.status().is_success() {
			return Err(WebhookError::RequestFailed(format!(
				"HTTP {}: {}",
				response.status(),
				response
					.text()
					.await
					.unwrap_or_else(|_| "No response body".to_string())
			)));
		}

		Ok(())
	}
}

#[async_trait]
impl WebhookSender for HttpWebhookSender {
	async fn send(&self, event: &WebhookEvent) -> Result<(), WebhookError> {
		self.send_with_retry(event).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_task_status_serialization() {
		let status = TaskStatus::Success;
		let json = serde_json::to_string(&status).unwrap();
		assert_eq!(json, r#""success""#);

		let status: TaskStatus = serde_json::from_str(r#""failed""#).unwrap();
		assert_eq!(status, TaskStatus::Failed);
	}

	#[test]
	fn test_webhook_event_serialization() {
		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("OK".to_string()),
			error: None,
			started_at: now,
			completed_at: now,
			duration_ms: 1000,
		};

		let json = serde_json::to_string(&event).unwrap();
		assert!(json.contains("test_task"));
		assert!(json.contains(r#""status":"success""#));

		let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.task_name, "test_task");
		assert_eq!(deserialized.status, TaskStatus::Success);
	}

	#[test]
	fn test_retry_config_default() {
		let config = RetryConfig::default();
		assert_eq!(config.max_retries, 3);
		assert_eq!(config.initial_backoff, Duration::from_millis(100));
		assert_eq!(config.max_backoff, Duration::from_secs(30));
		assert_eq!(config.backoff_multiplier, 2.0);
	}

	#[test]
	fn test_webhook_config_default() {
		let config = WebhookConfig::default();
		assert_eq!(config.url, "");
		assert_eq!(config.method, "POST");
		assert_eq!(config.timeout, Duration::from_secs(5));
		assert!(config.headers.is_empty());
	}

	#[test]
	fn test_calculate_backoff() {
		let config = WebhookConfig {
			url: "https://example.com".to_string(),
			method: "POST".to_string(),
			headers: HashMap::new(),
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig {
				max_retries: 3,
				initial_backoff: Duration::from_millis(100),
				max_backoff: Duration::from_secs(10),
				backoff_multiplier: 2.0,
			},
		};

		let sender = HttpWebhookSender::new(config);

		// Test exponential backoff
		let backoff0 = sender.calculate_backoff(0);
		let backoff1 = sender.calculate_backoff(1);
		let backoff2 = sender.calculate_backoff(2);

		// Verify exponential growth (accounting for jitter)
		assert!(backoff0.as_millis() >= 75 && backoff0.as_millis() <= 125); // ~100ms ±25%
		assert!(backoff1.as_millis() >= 150 && backoff1.as_millis() <= 250); // ~200ms ±25%
		assert!(backoff2.as_millis() >= 300 && backoff2.as_millis() <= 500); // ~400ms ±25%

		// Test max backoff cap
		let backoff_large = sender.calculate_backoff(100);
		assert!(backoff_large <= Duration::from_secs(10));
	}

	#[test]
	fn test_webhook_error_display() {
		let error = WebhookError::RequestFailed("Connection timeout".to_string());
		assert_eq!(
			error.to_string(),
			"Webhook request failed: Connection timeout"
		);

		let error = WebhookError::MaxRetriesExceeded;
		assert_eq!(error.to_string(), "Max retries exceeded for webhook");

		let error = WebhookError::SerializationError("Invalid JSON".to_string());
		assert_eq!(
			error.to_string(),
			"Webhook serialization error: Invalid JSON"
		);
	}

	#[tokio::test]
	async fn test_http_webhook_sender_creation() {
		let config = WebhookConfig::default();
		let sender = HttpWebhookSender::new(config);

		// Verify sender is created successfully
		assert_eq!(sender.config.method, "POST");
	}

	#[tokio::test]
	async fn test_webhook_event_creation() {
		let now = Utc::now();
		let started = now - chrono::Duration::seconds(5);

		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("Task completed successfully".to_string()),
			error: None,
			started_at: started,
			completed_at: now,
			duration_ms: 5000,
		};

		assert_eq!(event.task_name, "test_task");
		assert_eq!(event.status, TaskStatus::Success);
		assert!(event.result.is_some());
		assert!(event.error.is_none());
		assert_eq!(event.duration_ms, 5000);
	}

	#[tokio::test]
	async fn test_webhook_failed_event() {
		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "failed_task".to_string(),
			status: TaskStatus::Failed,
			result: None,
			error: Some("Database connection failed".to_string()),
			started_at: now,
			completed_at: now,
			duration_ms: 100,
		};

		assert_eq!(event.status, TaskStatus::Failed);
		assert!(event.result.is_none());
		assert!(event.error.is_some());
		assert_eq!(
			event.error.unwrap(),
			"Database connection failed".to_string()
		);
	}

	// Integration test with mock HTTP server
	#[tokio::test]
	async fn test_webhook_send_success() {
		let mut server = mockito::Server::new_async().await;
		let mock = server
			.mock("POST", "/webhook")
			.with_status(200)
			.with_header("content-type", "application/json")
			.with_body(r#"{"status":"ok"}"#)
			.create_async()
			.await;

		let config = WebhookConfig {
			url: format!("{}/webhook", server.url()),
			method: "POST".to_string(),
			headers: HashMap::new(),
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig {
				max_retries: 0,
				initial_backoff: Duration::from_millis(10),
				max_backoff: Duration::from_secs(1),
				backoff_multiplier: 2.0,
			},
		};

		let sender = HttpWebhookSender::new(config);

		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("OK".to_string()),
			error: None,
			started_at: now,
			completed_at: now,
			duration_ms: 100,
		};

		let result = sender.send(&event).await;
		assert!(result.is_ok());

		mock.assert_async().await;
	}

	#[tokio::test]
	async fn test_webhook_send_retry_then_success() {
		let mut server = mockito::Server::new_async().await;

		// First two requests fail, third succeeds
		let mock1 = server
			.mock("POST", "/webhook")
			.with_status(500)
			.expect(1)
			.create_async()
			.await;

		let mock2 = server
			.mock("POST", "/webhook")
			.with_status(503)
			.expect(1)
			.create_async()
			.await;

		let mock3 = server
			.mock("POST", "/webhook")
			.with_status(200)
			.expect(1)
			.create_async()
			.await;

		let config = WebhookConfig {
			url: format!("{}/webhook", server.url()),
			method: "POST".to_string(),
			headers: HashMap::new(),
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig {
				max_retries: 3,
				initial_backoff: Duration::from_millis(10),
				max_backoff: Duration::from_secs(1),
				backoff_multiplier: 2.0,
			},
		};

		let sender = HttpWebhookSender::new(config);

		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("OK".to_string()),
			error: None,
			started_at: now,
			completed_at: now,
			duration_ms: 100,
		};

		let result = sender.send(&event).await;
		assert!(result.is_ok());

		mock1.assert_async().await;
		mock2.assert_async().await;
		mock3.assert_async().await;
	}

	#[tokio::test]
	async fn test_webhook_send_max_retries_exceeded() {
		let mut server = mockito::Server::new_async().await;

		// All requests fail
		let mock = server
            .mock("POST", "/webhook")
            .with_status(500)
            .expect(4) // Initial + 3 retries
            .create_async()
            .await;

		let config = WebhookConfig {
			url: format!("{}/webhook", server.url()),
			method: "POST".to_string(),
			headers: HashMap::new(),
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig {
				max_retries: 3,
				initial_backoff: Duration::from_millis(10),
				max_backoff: Duration::from_secs(1),
				backoff_multiplier: 2.0,
			},
		};

		let sender = HttpWebhookSender::new(config);

		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("OK".to_string()),
			error: None,
			started_at: now,
			completed_at: now,
			duration_ms: 100,
		};

		let result = sender.send(&event).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			WebhookError::MaxRetriesExceeded
		));

		mock.assert_async().await;
	}

	#[tokio::test]
	async fn test_webhook_custom_headers() {
		let mut server = mockito::Server::new_async().await;

		let mock = server
			.mock("POST", "/webhook")
			.match_header("Authorization", "Bearer test-token")
			.match_header("X-Custom-Header", "custom-value")
			.with_status(200)
			.create_async()
			.await;

		let mut headers = HashMap::new();
		headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
		headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

		let config = WebhookConfig {
			url: format!("{}/webhook", server.url()),
			method: "POST".to_string(),
			headers,
			timeout: Duration::from_secs(5),
			retry_config: RetryConfig {
				max_retries: 0,
				initial_backoff: Duration::from_millis(10),
				max_backoff: Duration::from_secs(1),
				backoff_multiplier: 2.0,
			},
		};

		let sender = HttpWebhookSender::new(config);

		let now = Utc::now();
		let event = WebhookEvent {
			task_id: TaskId::new(),
			task_name: "test_task".to_string(),
			status: TaskStatus::Success,
			result: Some("OK".to_string()),
			error: None,
			started_at: now,
			completed_at: now,
			duration_ms: 100,
		};

		let result = sender.send(&event).await;
		assert!(result.is_ok());

		mock.assert_async().await;
	}
}
