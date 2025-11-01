//! RabbitMQ-based task backend implementation
//!
//! This module provides a RabbitMQ-based task queue backend using the AMQP protocol.
//! It stores tasks in RabbitMQ queues with status tracking in message properties.

use crate::{
	Task, TaskId, TaskStatus,
	backend::{TaskBackend, TaskExecutionError},
	registry::SerializedTask,
};
use async_trait::async_trait;
use lapin::{
	BasicProperties, Channel, Connection, ConnectionProperties, Error as LapinError, options::*,
	types::FieldTable,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Task metadata for RabbitMQ storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskMetadata {
	id: TaskId,
	name: String,
	status: TaskStatus,
	created_at: i64,
	updated_at: i64,
	task_data: Option<SerializedTask>,
}

/// RabbitMQ configuration
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::RabbitMQConfig;
///
/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f");
/// ```
#[derive(Debug, Clone)]
pub struct RabbitMQConfig {
	/// AMQP connection URL (e.g., "amqp://localhost:5672/%2f")
	pub url: String,
	/// Queue name for tasks
	pub queue_name: String,
	/// Exchange name (empty string for default exchange)
	pub exchange_name: String,
	/// Routing key
	pub routing_key: String,
}

impl RabbitMQConfig {
	/// Create a new RabbitMQ configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RabbitMQConfig;
	///
	/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f");
	/// ```
	pub fn new(url: &str) -> Self {
		Self {
			url: url.to_string(),
			queue_name: "reinhardt_tasks".to_string(),
			exchange_name: String::new(),
			routing_key: "reinhardt_tasks".to_string(),
		}
	}

	/// Set custom queue name
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RabbitMQConfig;
	///
	/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f")
	///     .with_queue_name("my_tasks");
	/// ```
	pub fn with_queue_name(mut self, queue_name: &str) -> Self {
		self.queue_name = queue_name.to_string();
		self.routing_key = queue_name.to_string();
		self
	}

	/// Set custom exchange name
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RabbitMQConfig;
	///
	/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f")
	///     .with_exchange("my_exchange");
	/// ```
	pub fn with_exchange(mut self, exchange_name: &str) -> Self {
		self.exchange_name = exchange_name.to_string();
		self
	}

	/// Set custom routing key
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RabbitMQConfig;
	///
	/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f")
	///     .with_routing_key("my_routing_key");
	/// ```
	pub fn with_routing_key(mut self, routing_key: &str) -> Self {
		self.routing_key = routing_key.to_string();
		self
	}
}

impl Default for RabbitMQConfig {
	fn default() -> Self {
		Self::new("amqp://localhost:5672/%2f")
	}
}

/// RabbitMQ-based task backend
///
/// Uses RabbitMQ as a message queue for task distribution and stores task
/// metadata in message properties.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::{RabbitMQBackend, RabbitMQConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f");
/// let backend = RabbitMQBackend::new(config).await?;
/// # Ok(())
/// # }
/// ```
pub struct RabbitMQBackend {
	connection: Arc<Connection>,
	channel: Arc<RwLock<Channel>>,
	config: RabbitMQConfig,
}

impl RabbitMQBackend {
	/// Create a new RabbitMQ backend
	///
	/// # Arguments
	///
	/// * `config` - RabbitMQ configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::{RabbitMQBackend, RabbitMQConfig};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let config = RabbitMQConfig::new("amqp://localhost:5672/%2f")
	///     .with_queue_name("my_tasks");
	/// let backend = RabbitMQBackend::new(config).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(config: RabbitMQConfig) -> Result<Self, LapinError> {
		let connection = Connection::connect(&config.url, ConnectionProperties::default()).await?;
		let channel = connection.create_channel().await?;

		// Declare queue
		channel
			.queue_declare(
				&config.queue_name,
				QueueDeclareOptions {
					durable: true,
					..Default::default()
				},
				FieldTable::default(),
			)
			.await?;

		Ok(Self {
			connection: Arc::new(connection),
			channel: Arc::new(RwLock::new(channel)),
			config,
		})
	}

	/// Create task metadata
	fn create_metadata(
		&self,
		task_id: TaskId,
		task_name: String,
		task_data: Option<SerializedTask>,
	) -> TaskMetadata {
		TaskMetadata {
			id: task_id,
			name: task_name,
			status: TaskStatus::Pending,
			created_at: chrono::Utc::now().timestamp(),
			updated_at: chrono::Utc::now().timestamp(),
			task_data,
		}
	}

	/// Ensure connection is healthy, reconnect if necessary
	async fn ensure_connection(&self) -> Result<(), TaskExecutionError> {
		if !self.connection.status().connected() {
			return Err(TaskExecutionError::BackendError(
				"RabbitMQ connection lost".to_string(),
			));
		}
		Ok(())
	}

	/// Get or recreate channel
	async fn get_channel(&self) -> Result<Channel, TaskExecutionError> {
		self.ensure_connection().await?;

		let channel = self.channel.read().await;
		if channel.status().connected() {
			return Ok(channel.clone());
		}

		drop(channel);

		// Recreate channel
		let new_channel = self
			.connection
			.create_channel()
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		// Redeclare queue
		new_channel
			.queue_declare(
				&self.config.queue_name,
				QueueDeclareOptions {
					durable: true,
					..Default::default()
				},
				FieldTable::default(),
			)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		let mut channel_write = self.channel.write().await;
		*channel_write = new_channel.clone();

		Ok(new_channel)
	}
}

#[async_trait]
impl TaskBackend for RabbitMQBackend {
	async fn enqueue(&self, task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		let task_id = task.id();
		let task_name = task.name().to_string();

		let metadata = self.create_metadata(task_id, task_name, None);

		let metadata_json = serde_json::to_string(&metadata)
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		let channel = self.get_channel().await?;

		// Publish task to queue
		channel
			.basic_publish(
				&self.config.exchange_name,
				&self.config.routing_key,
				BasicPublishOptions::default(),
				metadata_json.as_bytes(),
				BasicProperties::default().with_delivery_mode(2), // Persistent
			)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(task_id)
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		let channel = self.get_channel().await?;

		// Get message from queue
		let delivery = channel
			.basic_get(&self.config.queue_name, BasicGetOptions { no_ack: false })
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match delivery {
			Some(delivery) => {
				let metadata: TaskMetadata = serde_json::from_slice(&delivery.data)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				// Acknowledge message
				delivery
					.ack(BasicAckOptions::default())
					.await
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				Ok(Some(metadata.id))
			}
			None => Ok(None),
		}
	}

	async fn get_status(&self, _task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		// NOTE: RabbitMQ is a message queue, not a data store
		// Task status tracking requires additional storage (Redis, database, etc.)
		// For now, return Pending as tasks are consumed from queue
		Ok(TaskStatus::Pending)
	}

	async fn update_status(
		&self,
		_task_id: TaskId,
		_status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		// NOTE: RabbitMQ is a message queue, not a data store
		// Status updates require additional storage backend
		// This is intentionally a no-op for pure queue-based backend
		Ok(())
	}

	async fn get_task_data(
		&self,
		_task_id: TaskId,
	) -> Result<Option<SerializedTask>, TaskExecutionError> {
		// NOTE: RabbitMQ is a message queue, not a data store
		// Task data is retrieved during dequeue operation
		// Cannot retrieve by task_id without additional storage
		Ok(None)
	}

	fn backend_name(&self) -> &str {
		"rabbitmq"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Task, TaskPriority};
	use serial_test::serial;
	use testcontainers::{
		GenericImage,
		core::{ContainerPort, WaitFor},
		runners::AsyncRunner,
	};

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

	async fn setup_rabbitmq() -> testcontainers::ContainerAsync<GenericImage> {
		let rabbitmq_image = GenericImage::new("rabbitmq", "3-management-alpine")
			.with_exposed_port(ContainerPort::Tcp(5672))
			.with_wait_for(WaitFor::message_on_stdout("Server startup complete"));

		rabbitmq_image
			.start()
			.await
			.expect("Failed to start RabbitMQ container")
	}

	#[tokio::test]
	#[serial(rabbitmq)]
	async fn test_rabbitmq_backend_enqueue() {
		let container = setup_rabbitmq().await;
		let port = container
			.get_host_port_ipv4(5672)
			.await
			.expect("Failed to get port");
		let amqp_url = format!("amqp://guest:guest@127.0.0.1:{}/%2f", port);

		let config = RabbitMQConfig::new(&amqp_url);
		let backend = RabbitMQBackend::new(config)
			.await
			.expect("Failed to connect to RabbitMQ");

		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});

		let task_id = task.id();
		let result = backend.enqueue(task).await;
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), task_id);
	}

	#[tokio::test]
	#[serial(rabbitmq)]
	async fn test_rabbitmq_backend_dequeue() {
		let container = setup_rabbitmq().await;
		let port = container
			.get_host_port_ipv4(5672)
			.await
			.expect("Failed to get port");
		let amqp_url = format!("amqp://guest:guest@127.0.0.1:{}/%2f", port);

		let config = RabbitMQConfig::new(&amqp_url).with_queue_name("test_queue");
		let backend = RabbitMQBackend::new(config)
			.await
			.expect("Failed to connect to RabbitMQ");

		// Enqueue a task first
		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "dequeue_test".to_string(),
		});
		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		// Wait a bit for message to be available
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		// Dequeue the task
		let dequeued = backend.dequeue().await.expect("Failed to dequeue");
		assert_eq!(dequeued, Some(task_id));

		// Second dequeue should return None
		let empty = backend.dequeue().await.expect("Failed to dequeue");
		assert_eq!(empty, None);
	}

	#[tokio::test]
	#[serial(rabbitmq)]
	async fn test_rabbitmq_backend_get_status() {
		let container = setup_rabbitmq().await;
		let port = container
			.get_host_port_ipv4(5672)
			.await
			.expect("Failed to get port");
		let amqp_url = format!("amqp://guest:guest@127.0.0.1:{}/%2f", port);

		let config = RabbitMQConfig::new(&amqp_url);
		let backend = RabbitMQBackend::new(config)
			.await
			.expect("Failed to connect to RabbitMQ");

		// get_status returns Pending for queue-based backend
		let status = backend
			.get_status(TaskId::new())
			.await
			.expect("Failed to get status");
		assert_eq!(status, TaskStatus::Pending);
	}

	#[tokio::test]
	#[serial(rabbitmq)]
	async fn test_rabbitmq_backend_multiple_tasks() {
		let container = setup_rabbitmq().await;
		let port = container
			.get_host_port_ipv4(5672)
			.await
			.expect("Failed to get port");
		let amqp_url = format!("amqp://guest:guest@127.0.0.1:{}/%2f", port);

		let config = RabbitMQConfig::new(&amqp_url).with_queue_name("multi_queue");
		let backend = RabbitMQBackend::new(config)
			.await
			.expect("Failed to connect to RabbitMQ");

		// Enqueue multiple tasks
		let task1_id = TaskId::new();
		let task2_id = TaskId::new();
		let task3_id = TaskId::new();

		backend
			.enqueue(Box::new(TestTask {
				id: task1_id,
				name: "task1".to_string(),
			}))
			.await
			.expect("Failed to enqueue task1");

		backend
			.enqueue(Box::new(TestTask {
				id: task2_id,
				name: "task2".to_string(),
			}))
			.await
			.expect("Failed to enqueue task2");

		backend
			.enqueue(Box::new(TestTask {
				id: task3_id,
				name: "task3".to_string(),
			}))
			.await
			.expect("Failed to enqueue task3");

		// Wait for messages
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		// Dequeue in FIFO order
		assert_eq!(
			backend.dequeue().await.expect("Failed to dequeue"),
			Some(task1_id)
		);
		assert_eq!(
			backend.dequeue().await.expect("Failed to dequeue"),
			Some(task2_id)
		);
		assert_eq!(
			backend.dequeue().await.expect("Failed to dequeue"),
			Some(task3_id)
		);
		assert_eq!(backend.dequeue().await.expect("Failed to dequeue"), None);
	}

	#[tokio::test]
	#[serial(rabbitmq)]
	async fn test_rabbitmq_config() {
		let config = RabbitMQConfig::new("amqp://localhost:5672/%2f")
			.with_queue_name("custom_queue")
			.with_exchange("custom_exchange")
			.with_routing_key("custom_key");

		assert_eq!(config.url, "amqp://localhost:5672/%2f");
		assert_eq!(config.queue_name, "custom_queue");
		assert_eq!(config.exchange_name, "custom_exchange");
		assert_eq!(config.routing_key, "custom_key");
	}
}
