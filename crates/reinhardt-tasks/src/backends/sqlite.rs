//! SQLite-based task backend implementation

use crate::{
	Task, TaskExecutionError, TaskId, TaskStatus,
	result::{ResultBackend, TaskResultMetadata},
};
use async_trait::async_trait;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

/// SQLite-based task backend
///
/// Stores tasks in a SQLite database with status tracking.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::SqliteBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = SqliteBackend::new("sqlite::memory:").await?;
/// # Ok(())
/// # }
/// ```
pub struct SqliteBackend {
	pool: SqlitePool,
}

impl SqliteBackend {
	/// Create a new SQLite backend
	///
	/// # Arguments
	///
	/// * `database_url` - SQLite database URL (e.g., "sqlite::memory:" or "sqlite://path/to/db.sqlite")
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::SqliteBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = SqliteBackend::new("sqlite::memory:").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
		use std::str::FromStr;

		let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);

		let pool = SqlitePool::connect_with(options).await?;

		let backend = Self { pool };

		backend.create_tables().await?;

		Ok(backend)
	}

	/// Create necessary database tables
	async fn create_tables(&self) -> Result<(), sqlx::Error> {
		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                task_data TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
        "#,
		)
		.execute(&self.pool)
		.await?;

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS task_results (
                task_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                result TEXT,
                error TEXT,
                created_at INTEGER NOT NULL
            )
        "#,
		)
		.execute(&self.pool)
		.await?;

		Ok(())
	}
}

#[async_trait]
impl crate::backend::TaskBackend for SqliteBackend {
	async fn enqueue(&self, task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		let task_id = task.id();
		let task_name = task.name().to_string();
		let now = chrono::Utc::now().timestamp();

		let id_str = task_id.to_string();
		let status_str = "pending";

		// Create SerializedTask with task name and placeholder data
		let serialized = crate::registry::SerializedTask::new(task_name.clone(), "{}".to_string());
		let task_data_json = serialized
			.to_json()
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		sqlx::query(
			"INSERT INTO tasks (id, name, status, task_data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
		)
		.bind(&id_str)
		.bind(&task_name)
		.bind(status_str)
		.bind(&task_data_json)
		.bind(now)
		.bind(now)
		.execute(&self.pool)
		.await
		.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(task_id)
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		// Get oldest pending task
		let record: Option<(String,)> = sqlx::query_as(
			"SELECT id FROM tasks WHERE status = 'pending' ORDER BY created_at ASC LIMIT 1",
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((id_str,)) => {
				let task_id = id_str
					.parse()
					.map_err(|e: uuid::Error| TaskExecutionError::BackendError(e.to_string()))?;

				// Mark as running
				let now = chrono::Utc::now().timestamp();
				sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
					.bind("running")
					.bind(now)
					.bind(&id_str)
					.execute(&self.pool)
					.await
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				Ok(Some(task_id))
			}
			None => Ok(None),
		}
	}

	async fn get_status(&self, task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		let id_str = task_id.to_string();

		let record: Option<(String,)> = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
			.bind(&id_str)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((status_str,)) => status_str
				.parse::<TaskStatus>()
				.map_err(|e| TaskExecutionError::BackendError(format!("{e} for task {task_id}"))),
			None => Err(TaskExecutionError::NotFound(task_id)),
		}
	}

	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		let id_str = task_id.to_string();
		let now = chrono::Utc::now().timestamp();

		let result = sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
			.bind(status.as_str())
			.bind(now)
			.bind(&id_str)
			.execute(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		if result.rows_affected() == 0 {
			Err(TaskExecutionError::NotFound(task_id))
		} else {
			Ok(())
		}
	}

	async fn get_task_data(
		&self,
		task_id: TaskId,
	) -> Result<Option<crate::registry::SerializedTask>, TaskExecutionError> {
		let id_str = task_id.to_string();

		let record: Option<(String,)> = sqlx::query_as("SELECT task_data FROM tasks WHERE id = ?")
			.bind(&id_str)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((task_data_json,)) => {
				let serialized = crate::registry::SerializedTask::from_json(&task_data_json)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;
				Ok(Some(serialized))
			}
			None => Ok(None),
		}
	}

	fn backend_name(&self) -> &str {
		"sqlite"
	}
}

/// SQLite-based result backend for task result persistence
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::{SqliteResultBackend, ResultBackend, TaskResultMetadata, TaskId, TaskStatus};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = SqliteResultBackend::new("sqlite::memory:").await?;
///
/// let metadata = TaskResultMetadata::new(
///     TaskId::new(),
///     TaskStatus::Success,
///     Some("Task completed".to_string()),
/// );
///
/// backend.store_result(metadata).await?;
/// # Ok(())
/// # }
/// ```
pub struct SqliteResultBackend {
	pool: SqlitePool,
}

impl SqliteResultBackend {
	/// Create a new SQLite result backend
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::SqliteResultBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = SqliteResultBackend::new("sqlite::memory:").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
		let pool = SqlitePool::connect(database_url).await?;

		let backend = Self { pool };
		backend.create_tables().await?;

		Ok(backend)
	}

	async fn create_tables(&self) -> Result<(), sqlx::Error> {
		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS task_results (
                task_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                result TEXT,
                error TEXT,
                created_at INTEGER NOT NULL
            )
        "#,
		)
		.execute(&self.pool)
		.await?;

		Ok(())
	}
}

#[async_trait]
impl ResultBackend for SqliteResultBackend {
	async fn store_result(&self, metadata: TaskResultMetadata) -> Result<(), TaskExecutionError> {
		let task_id_str = metadata.task_id().to_string();

		sqlx::query(
			r#"
            INSERT OR REPLACE INTO task_results
            (task_id, status, result, error, created_at)
            VALUES (?, ?, ?, ?, ?)
        "#,
		)
		.bind(&task_id_str)
		.bind(metadata.status().as_str())
		.bind(metadata.result())
		.bind(metadata.error())
		.bind(metadata.created_at())
		.execute(&self.pool)
		.await
		.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(())
	}

	async fn get_result(
		&self,
		task_id: TaskId,
	) -> Result<Option<TaskResultMetadata>, TaskExecutionError> {
		let task_id_str = task_id.to_string();

		let record: Option<(String, Option<String>, Option<String>, i64)> = sqlx::query_as(
			"SELECT status, result, error, created_at FROM task_results WHERE task_id = ?",
		)
		.bind(&task_id_str)
		.fetch_optional(&self.pool)
		.await
		.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((status_str, result, error, created_at)) => {
				let status = status_str.parse::<TaskStatus>().map_err(|e| {
					TaskExecutionError::BackendError(format!("{e} for task {task_id}"))
				})?;

				let mut metadata =
					TaskResultMetadata::with_created_at(task_id, status, result, created_at);
				if let Some(err) = error {
					metadata.set_error(err);
				}

				Ok(Some(metadata))
			}
			None => Ok(None),
		}
	}

	async fn delete_result(&self, task_id: TaskId) -> Result<(), TaskExecutionError> {
		let task_id_str = task_id.to_string();

		sqlx::query("DELETE FROM task_results WHERE task_id = ?")
			.bind(&task_id_str)
			.execute(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backend::TaskBackend;
	use crate::{TaskId, TaskPriority};
	use rstest::{fixture, rstest};

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

	#[tokio::test]
	async fn test_sqlite_backend_creation() {
		let backend = SqliteBackend::new("sqlite::memory:").await;
		assert!(backend.is_ok());
	}

	#[tokio::test]
	async fn test_sqlite_backend_enqueue() {
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

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
	async fn test_sqlite_backend_get_status() {
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});

		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		let status = backend
			.get_status(task_id)
			.await
			.expect("Failed to get status");
		assert_eq!(status, TaskStatus::Pending);
	}

	#[tokio::test]
	async fn test_sqlite_backend_not_found() {
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let result = backend.get_status(TaskId::new()).await;
		assert!(result.is_err());
		assert!(matches!(result, Err(TaskExecutionError::NotFound(_))));
	}

	#[tokio::test]
	async fn test_sqlite_result_backend_store_and_retrieve() {
		let backend = SqliteResultBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let task_id = TaskId::new();
		let metadata = TaskResultMetadata::new(
			task_id,
			TaskStatus::Success,
			Some("Test result".to_string()),
		);

		// Store result
		backend
			.store_result(metadata.clone())
			.await
			.expect("Failed to store result");

		// Retrieve result
		let retrieved = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result");
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().result(), Some("Test result"));
	}

	#[rstest]
	#[case::success_with_result(TaskStatus::Success, Some("Test result"), None)]
	#[case::failure_with_error(TaskStatus::Failure, None, Some("boom"))]
	#[case::retry_with_result_and_error(TaskStatus::Retry, Some("partial"), Some("transient"))]
	#[tokio::test]
	async fn test_sqlite_result_backend_get_result_preserves_created_at(
		#[case] status: TaskStatus,
		#[case] result: Option<&str>,
		#[case] error: Option<&str>,
	) {
		// Arrange
		const PERSISTED_CREATED_AT: i64 = 1_234;
		let backend = SqliteResultBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");
		let task_id = TaskId::new();
		let mut metadata = TaskResultMetadata::with_created_at(
			task_id,
			status,
			result.map(str::to_string),
			PERSISTED_CREATED_AT,
		);
		if let Some(error) = error {
			metadata.set_error(error.to_string());
		}
		backend
			.store_result(metadata)
			.await
			.expect("Failed to store result");

		// Act
		let first = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result")
			.expect("Result should exist");
		let second = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result")
			.expect("Result should exist");

		// Assert
		for retrieved in [&first, &second] {
			assert_eq!(retrieved.created_at(), PERSISTED_CREATED_AT);
			assert_eq!(retrieved.task_id(), task_id);
			assert_eq!(retrieved.status(), status);
			assert_eq!(retrieved.result(), result);
			assert_eq!(retrieved.error(), error);
		}
	}

	#[tokio::test]
	async fn test_sqlite_result_backend_delete() {
		let backend = SqliteResultBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let task_id = TaskId::new();
		let metadata = TaskResultMetadata::new(task_id, TaskStatus::Success, None);

		// Store and then delete
		backend
			.store_result(metadata)
			.await
			.expect("Failed to store result");
		backend
			.delete_result(task_id)
			.await
			.expect("Failed to delete result");

		// Verify deleted
		let retrieved = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result");
		assert!(retrieved.is_none());
	}

	#[tokio::test]
	async fn test_sqlite_backend_get_task_data_after_enqueue() {
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});

		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		// Retrieve task data
		let task_data = backend
			.get_task_data(task_id)
			.await
			.expect("Failed to get task data");

		let serialized = task_data.unwrap();
		assert_eq!(serialized.name(), "test_task");
		assert_eq!(serialized.data(), "{}");
	}

	#[tokio::test]
	async fn test_sqlite_backend_get_task_data_not_found() {
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");

		let task_id = TaskId::new();
		let task_data = backend
			.get_task_data(task_id)
			.await
			.expect("Failed to get task data");

		assert!(task_data.is_none());
	}

	#[tokio::test]
	async fn test_sqlite_backend_task_data_persistence() {
		use tempfile::tempdir;

		// Create temporary directory for database file
		let temp_dir = tempdir().expect("Failed to create temp directory");
		let db_path = temp_dir.path().join("test.db");
		let db_url = format!("sqlite:///{}", db_path.display());

		let task_id = TaskId::new();
		let task_name = "persistent_task".to_string();

		// Create backend and enqueue task
		{
			let backend = SqliteBackend::new(&db_url)
				.await
				.expect("Failed to create backend");

			let task = Box::new(TestTask {
				id: task_id,
				name: task_name.clone(),
			});

			backend.enqueue(task).await.expect("Failed to enqueue");
		}

		// Create new backend instance with same database
		{
			let backend = SqliteBackend::new(&db_url)
				.await
				.expect("Failed to create backend");

			// Verify task data persists
			let task_data = backend
				.get_task_data(task_id)
				.await
				.expect("Failed to get task data");

			let serialized = task_data.unwrap();
			assert_eq!(serialized.name(), task_name);
			assert_eq!(serialized.data(), "{}");
		}
	}

	#[fixture]
	async fn result_backend() -> SqliteResultBackend {
		SqliteResultBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create result backend")
	}

	#[rstest]
	#[case(TaskStatus::Pending, "pending")]
	#[case(TaskStatus::Running, "running")]
	#[case(TaskStatus::Success, "success")]
	#[case(TaskStatus::Failure, "failure")]
	#[case(TaskStatus::Retry, "retry")]
	#[tokio::test]
	async fn test_sqlite_result_backend_persists_status_text_and_round_trips(
		#[future] result_backend: SqliteResultBackend,
		#[case] status: TaskStatus,
		#[case] expected_text: &str,
	) {
		// Arrange
		let backend = result_backend.await;
		let task_id = TaskId::new();

		// Act
		backend
			.store_result(TaskResultMetadata::new(task_id, status, None))
			.await
			.expect("Failed to store result");
		let stored_text: String =
			sqlx::query_scalar("SELECT status FROM task_results WHERE task_id = ?")
				.bind(task_id.to_string())
				.fetch_one(&backend.pool)
				.await
				.expect("Failed to read raw status column");
		let retrieved = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result")
			.expect("Result should exist");

		// Assert
		assert_eq!(stored_text, expected_text);
		assert_eq!(retrieved.status(), status);
	}

	#[rstest]
	#[case("bogus")]
	#[case("")]
	#[case("Success")]
	#[tokio::test]
	async fn test_sqlite_result_backend_rejects_unknown_status(
		#[future] result_backend: SqliteResultBackend,
		#[case] corrupted_status: &str,
	) {
		// Arrange
		let backend = result_backend.await;
		let task_id = TaskId::new();
		backend
			.store_result(TaskResultMetadata::new(
				task_id,
				TaskStatus::Success,
				Some("done".to_string()),
			))
			.await
			.expect("Failed to store result");
		sqlx::query("UPDATE task_results SET status = ? WHERE task_id = ?")
			.bind(corrupted_status)
			.bind(task_id.to_string())
			.execute(&backend.pool)
			.await
			.expect("Failed to corrupt status column");

		// Act
		let result = backend.get_result(task_id).await;

		// Assert
		match result {
			Err(TaskExecutionError::BackendError(message)) => assert_eq!(
				message,
				format!("unknown task status '{corrupted_status}' for task {task_id}")
			),
			other => panic!("expected BackendError for unknown status, got {other:?}"),
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_sqlite_backend_get_status_rejects_unknown_status() {
		// Arrange
		let backend = SqliteBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create backend");
		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});
		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");
		sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
			.bind("bogus")
			.bind(task_id.to_string())
			.execute(&backend.pool)
			.await
			.expect("Failed to corrupt status column");

		// Act
		let result = backend.get_status(task_id).await;

		// Assert
		match result {
			Err(TaskExecutionError::BackendError(message)) => assert_eq!(
				message,
				format!("unknown task status 'bogus' for task {task_id}")
			),
			other => panic!("expected BackendError for unknown status, got {other:?}"),
		}
	}
}
