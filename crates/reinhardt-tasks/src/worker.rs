//! Task worker

use crate::{
	TaskBackend, TaskStatus,
	locking::TaskLock,
	registry::TaskRegistry,
	result::{ResultBackend, TaskResultMetadata},
	webhook::{HttpWebhookSender, WebhookConfig, WebhookEvent, WebhookSender},
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, broadcast};

/// Worker configuration
///
/// Controls worker behavior including name, concurrency, polling interval, and webhook notifications.
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::WorkerConfig;
/// use std::time::Duration;
///
/// let config = WorkerConfig::new("my-worker".to_string())
///     .with_concurrency(8)
///     .with_poll_interval(Duration::from_millis(100));
///
/// assert_eq!(config.name, "my-worker");
/// assert_eq!(config.concurrency, 8);
/// ```
#[derive(Debug, Clone)]
pub struct WorkerConfig {
	pub name: String,
	pub concurrency: usize,
	pub poll_interval: Duration,
	pub webhook_configs: Vec<WebhookConfig>,
}

impl WorkerConfig {
	/// Create a new worker configuration with default values
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::WorkerConfig;
	///
	/// let config = WorkerConfig::new("worker-1".to_string());
	/// assert_eq!(config.name, "worker-1");
	/// assert_eq!(config.concurrency, 4);
	/// ```
	pub fn new(name: String) -> Self {
		Self {
			name,
			concurrency: 4,
			poll_interval: Duration::from_secs(1),
			webhook_configs: Vec::new(),
		}
	}

	/// Set the concurrency level
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::WorkerConfig;
	///
	/// let config = WorkerConfig::new("worker".to_string()).with_concurrency(8);
	/// assert_eq!(config.concurrency, 8);
	/// ```
	pub fn with_concurrency(mut self, concurrency: usize) -> Self {
		self.concurrency = concurrency;
		self
	}

	/// Set the poll interval
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::WorkerConfig;
	/// use std::time::Duration;
	///
	/// let config = WorkerConfig::new("worker".to_string())
	///     .with_poll_interval(Duration::from_millis(500));
	/// assert_eq!(config.poll_interval, Duration::from_millis(500));
	/// ```
	pub fn with_poll_interval(mut self, interval: Duration) -> Self {
		self.poll_interval = interval;
		self
	}

	/// Add a webhook configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{WorkerConfig, webhook::WebhookConfig};
	/// use std::time::Duration;
	///
	/// let webhook_config = WebhookConfig {
	///     url: "https://example.com/webhook".to_string(),
	///     method: "POST".to_string(),
	///     headers: Default::default(),
	///     timeout: Duration::from_secs(5),
	///     retry_config: Default::default(),
	/// };
	///
	/// let config = WorkerConfig::new("worker".to_string())
	///     .with_webhook(webhook_config);
	/// assert_eq!(config.webhook_configs.len(), 1);
	/// ```
	pub fn with_webhook(mut self, webhook_config: WebhookConfig) -> Self {
		self.webhook_configs.push(webhook_config);
		self
	}

	/// Set multiple webhook configurations
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{WorkerConfig, webhook::WebhookConfig};
	///
	/// let webhooks = vec![
	///     WebhookConfig::default(),
	///     WebhookConfig::default(),
	/// ];
	///
	/// let config = WorkerConfig::new("worker".to_string())
	///     .with_webhooks(webhooks);
	/// assert_eq!(config.webhook_configs.len(), 2);
	/// ```
	pub fn with_webhooks(mut self, webhook_configs: Vec<WebhookConfig>) -> Self {
		self.webhook_configs = webhook_configs;
		self
	}
}

impl Default for WorkerConfig {
	fn default() -> Self {
		Self::new("worker".to_string())
	}
}

/// Task worker
///
/// Polls the backend for tasks and executes them concurrently.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_tasks::{Worker, WorkerConfig, DummyBackend};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = WorkerConfig::new("worker-1".to_string());
/// let worker = Worker::new(config);
/// let backend = Arc::new(DummyBackend::new());
///
/// // Start worker in background
/// let handle = tokio::spawn(async move {
///     worker.run(backend).await
/// });
///
/// // Later: stop the worker
/// handle.abort();
/// # Ok(())
/// # }
/// ```
pub struct Worker {
	config: WorkerConfig,
	shutdown_tx: broadcast::Sender<()>,
	registry: Option<Arc<TaskRegistry>>,
	task_lock: Option<Arc<dyn TaskLock>>,
	result_backend: Option<Arc<dyn ResultBackend>>,
	webhook_senders: Vec<Arc<dyn WebhookSender>>,
	/// Semaphore that enforces the configured concurrency limit
	concurrency_semaphore: Arc<Semaphore>,
}

impl Worker {
	/// Create a new worker
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{Worker, WorkerConfig};
	///
	/// let config = WorkerConfig::new("worker-1".to_string());
	/// let worker = Worker::new(config.clone());
	/// ```
	pub fn new(config: WorkerConfig) -> Self {
		let (shutdown_tx, _) = broadcast::channel(1);
		let concurrency_semaphore = Arc::new(Semaphore::new(config.concurrency));

		// Create webhook senders from configuration
		let webhook_senders: Vec<Arc<dyn WebhookSender>> = config
			.webhook_configs
			.iter()
			.map(|webhook_config| {
				Arc::new(HttpWebhookSender::new(webhook_config.clone())) as Arc<dyn WebhookSender>
			})
			.collect();

		Self {
			config,
			shutdown_tx,
			registry: None,
			task_lock: None,
			result_backend: None,
			webhook_senders,
			concurrency_semaphore,
		}
	}

	/// Set the task registry for dynamic task dispatch
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{Worker, WorkerConfig, TaskRegistry};
	/// use std::sync::Arc;
	///
	/// let worker = Worker::new(WorkerConfig::default())
	///     .with_registry(Arc::new(TaskRegistry::new()));
	/// ```
	pub fn with_registry(mut self, registry: Arc<TaskRegistry>) -> Self {
		self.registry = Some(registry);
		self
	}

	/// Set the task lock for distributed task execution
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{Worker, WorkerConfig, MemoryTaskLock};
	/// use std::sync::Arc;
	///
	/// let worker = Worker::new(WorkerConfig::default())
	///     .with_lock(Arc::new(MemoryTaskLock::new()));
	/// ```
	pub fn with_lock(mut self, task_lock: Arc<dyn TaskLock>) -> Self {
		self.task_lock = Some(task_lock);
		self
	}

	/// Set the result backend for storing task results
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{Worker, WorkerConfig, MemoryResultBackend};
	/// use std::sync::Arc;
	///
	/// let worker = Worker::new(WorkerConfig::default())
	///     .with_result_backend(Arc::new(MemoryResultBackend::new()));
	/// ```
	pub fn with_result_backend(mut self, result_backend: Arc<dyn ResultBackend>) -> Self {
		self.result_backend = Some(result_backend);
		self
	}

	/// Run the worker loop
	///
	/// This method blocks until the worker is stopped via `stop()`.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::{Worker, WorkerConfig, DummyBackend};
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	/// let worker = Worker::new(WorkerConfig::default());
	/// let backend = Arc::new(DummyBackend::new());
	///
	/// worker.run(backend).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn run(
		&self,
		backend: Arc<dyn TaskBackend>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		use tokio::time::interval;

		let mut shutdown_rx = self.shutdown_tx.subscribe();
		let mut poll_interval = interval(self.config.poll_interval);

		println!(
			"[{}] Worker started with concurrency {}",
			self.config.name, self.config.concurrency
		);

		loop {
			tokio::select! {
				_ = shutdown_rx.recv() => {
					println!("[{}] Shutdown signal received", self.config.name);
					break;
				}
				_ = poll_interval.tick() => {
					self.try_process_task(backend.clone()).await;
				}
			}
		}

		println!("[{}] Worker stopped", self.config.name);
		Ok(())
	}

	/// Try to process a single task from the backend.
	///
	/// Acquires a concurrency permit before executing the task, ensuring the
	/// configured concurrency limit is enforced. The permit is released
	/// when the spawned task completes.
	async fn try_process_task(&self, backend: Arc<dyn TaskBackend>) {
		match backend.dequeue().await {
			Ok(Some(task_id)) => {
				// Acquire a concurrency permit before executing the task.
				// This enforces the configured concurrency limit.
				let permit = match self.concurrency_semaphore.clone().acquire_owned().await {
					Ok(permit) => permit,
					Err(_) => {
						eprintln!(
							"[{}] Concurrency semaphore closed unexpectedly",
							self.config.name
						);
						return;
					}
				};

				println!("[{}] Processing task: {}", self.config.name, task_id);

				// Execute task; permit is held for the duration
				match self.execute_task(task_id, backend.clone()).await {
					Ok(_) => {
						println!(
							"[{}] Task {} completed successfully",
							self.config.name, task_id
						);
						if let Err(e) = backend.update_status(task_id, TaskStatus::Success).await {
							eprintln!(
								"[{}] Failed to update task {} status: {}",
								self.config.name, task_id, e
							);
						}
					}
					Err(e) => {
						eprintln!("[{}] Task {} failed: {}", self.config.name, task_id, e);
						if let Err(e) = backend.update_status(task_id, TaskStatus::Failure).await {
							eprintln!(
								"[{}] Failed to update task {} status: {}",
								self.config.name, task_id, e
							);
						}
					}
				}

				// Permit is dropped here, releasing the concurrency slot
				drop(permit);
			}
			Ok(None) => {
				// No tasks available - interval will automatically wait before next poll
			}
			Err(e) => {
				eprintln!("[{}] Failed to dequeue task: {}", self.config.name, e);
				// Error occurred - interval will automatically wait before next poll
			}
		}
	}

	/// Execute a task
	async fn execute_task(
		&self,
		task_id: crate::TaskId,
		backend: Arc<dyn TaskBackend>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		println!("[{}] Executing task: {}", self.config.name, task_id);

		let started_at = Utc::now();

		// Try to acquire lock if available
		if let Some(ref lock) = self.task_lock {
			let acquired = lock.acquire(task_id, Duration::from_secs(300)).await?;
			if !acquired {
				println!(
					"[{}] Task {} already locked by another worker",
					self.config.name, task_id
				);
				return Ok(());
			}
		}

		// Get task name from backend
		let task_name = match backend.get_task_data(task_id).await? {
			Some(serialized_task) => serialized_task.name().to_string(),
			None => "unknown_task".to_string(),
		};

		// Execute task with registry if available
		let result: Result<(), Box<dyn std::error::Error + Send + Sync>> =
			if let Some(ref registry) = self.registry {
				// Get serialized task data from backend
				match backend.get_task_data(task_id).await? {
					Some(serialized_task) => {
						println!(
							"[{}] Executing task {} with registry",
							self.config.name, task_name
						);

						// Deserialize task using registry to get concrete task instance
						match registry
							.create(serialized_task.name(), serialized_task.data())
							.await
						{
							Ok(task_executor) => {
								// Execute the deserialized task with its arguments
								match task_executor.execute().await {
									Ok(_) => {
										println!(
											"[{}] Task {} completed successfully",
											self.config.name, task_name
										);
										Ok(())
									}
									Err(e) => {
										println!(
											"[{}] Task {} failed: {}",
											self.config.name, task_name, e
										);
										Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
									}
								}
							}
							Err(e) => {
								println!(
									"[{}] Failed to deserialize task {}: {}",
									self.config.name, task_name, e
								);
								Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
							}
						}
					}
					None => {
						println!(
							"[{}] Task {} not found in backend",
							self.config.name, task_id
						);
						Err(format!("Task {} not found", task_id).into())
					}
				}
			} else {
				println!(
					"[{}] Task execution without registry (basic mode)",
					self.config.name
				);
				Ok(())
			};

		let completed_at = Utc::now();
		// Use saturating conversion to prevent overflow on negative or very large durations
		let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

		// Determine final task status
		let (task_status, webhook_status) = match &result {
			Ok(_) => (TaskStatus::Success, crate::webhook::TaskStatus::Success),
			Err(_) => (TaskStatus::Failure, crate::webhook::TaskStatus::Failed),
		};

		// Store result if result backend is available
		if let Some(ref result_backend) = self.result_backend {
			let metadata = match result {
				Ok(_) => TaskResultMetadata::new(
					task_id,
					task_status,
					Some("Task completed successfully".to_string()),
				),
				Err(ref e) => {
					TaskResultMetadata::with_error(task_id, format!("Task failed: {}", e))
				}
			};

			result_backend.store_result(metadata).await?;
		}

		// Send webhook notifications
		if !self.webhook_senders.is_empty() {
			let webhook_event = WebhookEvent {
				task_id,
				task_name,
				status: webhook_status,
				result: match webhook_status {
					crate::webhook::TaskStatus::Success => {
						Some("Task completed successfully".to_string())
					}
					crate::webhook::TaskStatus::Failed => None,
					crate::webhook::TaskStatus::Cancelled => None,
				},
				error: match webhook_status {
					crate::webhook::TaskStatus::Failed => match &result {
						Err(e) => Some(e.to_string()),
						_ => Some("Unknown error".to_string()),
					},
					_ => None,
				},
				started_at,
				completed_at,
				duration_ms,
			};

			// Send to all configured webhooks (fire and forget)
			for sender in &self.webhook_senders {
				let sender_clone = Arc::clone(sender);
				let event_clone = webhook_event.clone();
				tokio::spawn(async move {
					if let Err(e) = sender_clone.send(&event_clone).await {
						eprintln!("Failed to send webhook notification: {}", e);
					}
				});
			}
		}

		// Release lock if acquired
		if let Some(ref lock) = self.task_lock {
			lock.release(task_id).await?;
		}

		result
	}

	/// Stop the worker
	///
	/// Sends a shutdown signal to all worker loops.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{Worker, WorkerConfig};
	///
	/// # async fn example() {
	/// let worker = Worker::new(WorkerConfig::default());
	/// worker.stop().await;
	/// # }
	/// ```
	pub async fn stop(&self) {
		let _ = self.shutdown_tx.send(());
	}
}

impl Default for Worker {
	fn default() -> Self {
		let config = WorkerConfig::default();
		let concurrency_semaphore = Arc::new(Semaphore::new(config.concurrency));
		Self {
			config,
			shutdown_tx: broadcast::channel(1).0,
			registry: None,
			task_lock: None,
			result_backend: None,
			webhook_senders: Vec::new(),
			concurrency_semaphore,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{DummyBackend, Task, TaskId, TaskPriority};
	use rstest::rstest;
	use std::time::Duration;
	use tokio::time::sleep;

	// Test helper: unused fields are intentional for trait implementation
	#[allow(dead_code)]
	struct TestTask {
		id: TaskId,
		name: String,
	}

	impl Task for TestTask {
		fn id(&self) -> TaskId {
			self.id
		}

		fn name(&self) -> &str {
			&self.name
		}

		fn priority(&self) -> TaskPriority {
			TaskPriority::new(5)
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_creation() {
		// Arrange
		let config = WorkerConfig::new("test-worker".to_string());

		// Act
		let worker = Worker::new(config);

		// Assert
		assert_eq!(worker.config.name, "test-worker");
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_config_builder() {
		// Arrange & Act
		let config = WorkerConfig::new("test".to_string())
			.with_concurrency(8)
			.with_poll_interval(Duration::from_millis(100));

		// Assert
		assert_eq!(config.concurrency, 8);
		assert_eq!(config.poll_interval, Duration::from_millis(100));
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_start_and_stop() {
		// Arrange
		let worker = Worker::new(WorkerConfig::default());
		let backend = Arc::new(DummyBackend::new());
		let worker_clone = Worker {
			config: worker.config.clone(),
			shutdown_tx: worker.shutdown_tx.clone(),
			registry: None,
			task_lock: None,
			result_backend: None,
			webhook_senders: Vec::new(),
			concurrency_semaphore: worker.concurrency_semaphore.clone(),
		};

		let handle = tokio::spawn(async move { worker.run(backend).await });

		// Give worker time to start
		sleep(Duration::from_millis(100)).await;

		// Act
		worker_clone.stop().await;

		// Assert - worker should finish within timeout
		let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_with_registry() {
		// Arrange
		use crate::registry::TaskRegistry;
		let registry = Arc::new(TaskRegistry::new());

		// Act
		let worker = Worker::new(WorkerConfig::default()).with_registry(registry);

		// Assert
		assert!(worker.registry.is_some());
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_with_lock() {
		// Arrange
		use crate::locking::MemoryTaskLock;
		let lock = Arc::new(MemoryTaskLock::new());

		// Act
		let worker = Worker::new(WorkerConfig::default()).with_lock(lock);

		// Assert
		assert!(worker.task_lock.is_some());
	}

	#[rstest]
	#[tokio::test]
	async fn test_worker_with_result_backend() {
		// Arrange
		use crate::result::MemoryResultBackend;
		let backend = Arc::new(MemoryResultBackend::new());

		// Act
		let worker = Worker::new(WorkerConfig::default()).with_result_backend(backend);

		// Assert
		assert!(worker.result_backend.is_some());
	}
}
