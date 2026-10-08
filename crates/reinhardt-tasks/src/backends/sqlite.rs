//! SQLite-based task backend implementation

use crate::{
	Task, TaskExecutionError, TaskId, TaskStatus,
	result::{ResultBackend, TaskResultMetadata},
};
use async_trait::async_trait;
use reinhardt_query::prelude::{
	ColumnDef, Expr, ExprTrait, IntoValue, Query, QueryStatementBuilder, SqliteQueryBuilder,
};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

fn prepare(
	statement: impl QueryStatementBuilder,
) -> Result<(String, sqlx::sqlite::SqliteArguments<'static>), TaskExecutionError> {
	reinhardt_query_sqlx::prepare_sqlite(statement.build_any(&SqliteQueryBuilder))
		.map(|prepared| prepared.into_parts())
		.map_err(|error| TaskExecutionError::BackendError(error.to_string()))
}
fn task_results_schema() -> String {
	Query::create_table()
		.table("task_results")
		.if_not_exists()
		.col(ColumnDef::new("task_id").text().primary_key(true))
		.col(ColumnDef::new("status").text().not_null(true))
		.col(ColumnDef::new("result").text())
		.col(ColumnDef::new("error").text())
		.col(ColumnDef::new("created_at").integer().not_null(true))
		.to_string(SqliteQueryBuilder)
}

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
		let sql = Query::create_table()
			.table("tasks")
			.if_not_exists()
			.col(ColumnDef::new("id").text().primary_key(true))
			.col(ColumnDef::new("name").text().not_null(true))
			.col(ColumnDef::new("status").text().not_null(true))
			.col(ColumnDef::new("task_data").text())
			.col(ColumnDef::new("created_at").integer().not_null(true))
			.col(ColumnDef::new("updated_at").integer().not_null(true))
			.to_string(SqliteQueryBuilder);
		sqlx::query(&sql).execute(&self.pool).await?;
		sqlx::query(&task_results_schema())
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

		let (sql, arguments) = prepare(
			Query::insert()
				.into_table("tasks")
				.columns([
					"id",
					"name",
					"status",
					"task_data",
					"created_at",
					"updated_at",
				])
				.values_panic(vec![
					id_str.into_value(),
					task_name.into_value(),
					status_str.into_value(),
					task_data_json.into_value(),
					now.into_value(),
					now.into_value(),
				])
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(task_id)
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		use reinhardt_query::prelude::{
			Expr, ExprTrait, Order, Query, QueryStatementBuilder, SqliteQueryBuilder,
		};

		let now = chrono::Utc::now().timestamp();

		// Atomically claim the oldest pending task in a single statement:
		//
		//     UPDATE tasks SET status = 'running', updated_at = ?
		//     WHERE id IN (SELECT id FROM tasks WHERE status = 'pending'
		//                  ORDER BY created_at ASC LIMIT 1)
		//     RETURNING id
		//
		// A separate `SELECT ... LIMIT 1` followed by `UPDATE ... WHERE id = ?`
		// leaves a race window: with N consumers calling `dequeue()` at once, two
		// of them can SELECT the same pending row before either UPDATEs it, and
		// both then run the same task (issue #1). SQLite serializes writers, so
		// folding the selection into the `UPDATE`'s `WHERE` subquery makes the
		// claim atomic: the first writer flips the row to `running`, and every
		// subsequent writer re-evaluates the subquery and sees that row is no
		// longer pending, moving on to the next pending row (or claiming nothing).
		//
		// `RETURNING` (SQLite 3.35+, bundled with sqlx's SQLite driver) reports
		// exactly which row this call claimed.
		let (sql, values) = Query::update()
			.table("tasks")
			.value("status", "running")
			.value("updated_at", now)
			.and_where(
				Expr::col("id").in_subquery(
					Query::select()
						.column("id")
						.from("tasks")
						.and_where(Expr::col("status").eq("pending"))
						.order_by("created_at", Order::Asc)
						.limit(1)
						.take(),
				),
			)
			.returning(["id"])
			.build(SqliteQueryBuilder);

		let (sql, arguments) = reinhardt_query_sqlx::prepare_sqlite((sql, values))
			.map(|prepared| prepared.into_parts())
			.map_err(|error| TaskExecutionError::BackendError(error.to_string()))?;
		let record: Option<(String,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((id_str,)) => {
				let task_id = id_str
					.parse()
					.map_err(|e: uuid::Error| TaskExecutionError::BackendError(e.to_string()))?;
				Ok(Some(task_id))
			}
			None => Ok(None),
		}
	}

	async fn get_status(&self, task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		let id_str = task_id.to_string();

		let (sql, arguments) = prepare(
			Query::select()
				.column("status")
				.from("tasks")
				.and_where(Expr::col("id").eq(id_str))
				.take(),
		)?;
		let record: Option<(String,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((status_str,)) => {
				let status = match status_str.as_str() {
					"pending" => TaskStatus::Pending,
					"running" => TaskStatus::Running,
					"success" => TaskStatus::Success,
					"failure" => TaskStatus::Failure,
					"retry" => TaskStatus::Retry,
					_ => TaskStatus::Pending,
				};
				Ok(status)
			}
			None => Err(TaskExecutionError::NotFound(task_id)),
		}
	}

	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		let id_str = task_id.to_string();
		let status_str = match status {
			TaskStatus::Pending => "pending",
			TaskStatus::Running => "running",
			TaskStatus::Success => "success",
			TaskStatus::Failure => "failure",
			TaskStatus::Retry => "retry",
		};
		let now = chrono::Utc::now().timestamp();

		let (sql, arguments) = prepare(
			Query::update()
				.table("tasks")
				.value("status", status_str)
				.value("updated_at", now)
				.and_where(Expr::col("id").eq(id_str))
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
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

		let (sql, arguments) = prepare(
			Query::select()
				.column("task_data")
				.from("tasks")
				.and_where(Expr::col("id").eq(id_str))
				.take(),
		)?;
		let record: Option<(String,)> = sqlx::query_as_with(&sql, arguments)
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
		sqlx::query(&task_results_schema())
			.execute(&self.pool)
			.await?;

		Ok(())
	}
}

#[async_trait]
impl ResultBackend for SqliteResultBackend {
	async fn store_result(&self, metadata: TaskResultMetadata) -> Result<(), TaskExecutionError> {
		let task_id_str = metadata.task_id().to_string();
		let status_str = match metadata.status() {
			TaskStatus::Pending => "pending",
			TaskStatus::Running => "running",
			TaskStatus::Success => "success",
			TaskStatus::Failure => "failure",
			TaskStatus::Retry => "retry",
		};

		let built = {
			let statement = Query::insert()
				.into_table("task_results")
				.columns(["task_id", "status", "result", "error", "created_at"])
				.values_panic(vec![
					task_id_str.into_value(),
					status_str.into_value(),
					metadata.result().into_value(),
					metadata.error().into_value(),
					metadata.created_at().into_value(),
				])
				.sqlite_or_replace()
				.take();
			SqliteQueryBuilder
				.build_insert_checked(&statement)
				.map_err(|error| TaskExecutionError::BackendError(error.to_string()))?
		};
		let (sql, arguments) = reinhardt_query_sqlx::prepare_sqlite(built)
			.map(|prepared| prepared.into_parts())
			.map_err(|error| TaskExecutionError::BackendError(error.to_string()))?;
		sqlx::query_with(&sql, arguments)
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

		let (sql, arguments) = prepare(
			Query::select()
				.columns(["status", "result", "error", "created_at"])
				.from("task_results")
				.and_where(Expr::col("task_id").eq(task_id_str))
				.take(),
		)?;
		let record: Option<(String, Option<String>, Option<String>, i64)> =
			sqlx::query_as_with(&sql, arguments)
				.fetch_optional(&self.pool)
				.await
				.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		match record {
			Some((status_str, result, error, _created_at)) => {
				let status = match status_str.as_str() {
					"pending" => TaskStatus::Pending,
					"running" => TaskStatus::Running,
					"success" => TaskStatus::Success,
					"failure" => TaskStatus::Failure,
					"retry" => TaskStatus::Retry,
					_ => TaskStatus::Pending,
				};

				let mut metadata = TaskResultMetadata::new(task_id, status, result);
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

		let (sql, arguments) = prepare(
			Query::delete()
				.from_table("task_results")
				.and_where(Expr::col("task_id").eq(task_id_str))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
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
	use rstest::rstest;
	use std::collections::HashSet;
	use std::sync::Arc;

	#[rstest]
	#[tokio::test]
	async fn result_replacement_preserves_nulls_and_delete_insert_semantics() {
		// Arrange
		let backend = SqliteResultBackend::new("sqlite::memory:").await.unwrap();
		let task_id = TaskId::new();
		let mut first =
			TaskResultMetadata::new(task_id, TaskStatus::Failure, Some("first' ? $1".into()));
		first.set_error("old error".into());
		backend.store_result(first).await.unwrap();
		let (first_rowid,): (i64,) =
			sqlx::query_as("SELECT rowid FROM task_results WHERE task_id = ?")
				.bind(task_id.to_string())
				.fetch_one(&backend.pool)
				.await
				.unwrap();
		let next = TaskResultMetadata::new(task_id, TaskStatus::Success, None);
		let timestamp = next.created_at();
		// Act
		backend.store_result(next).await.unwrap();
		let row: (i64, String, Option<String>, Option<String>, i64) = sqlx::query_as(
			"SELECT rowid, status, result, error, created_at FROM task_results WHERE task_id = ?",
		)
		.bind(task_id.to_string())
		.fetch_one(&backend.pool)
		.await
		.unwrap();
		// Assert
		assert_eq!(
			row,
			(first_rowid + 1, "success".into(), None, None, timestamp)
		);
	}

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

	/// Regression for issue #1: when many consumers call `dequeue()` at once,
	/// each pending task must be claimed by exactly one caller. The previous
	/// non-atomic `SELECT` + `UPDATE` allowed two consumers to select the same
	/// pending row and both run it. The atomic `UPDATE ... RETURNING` claim must
	/// hand each task to a single caller, so the set of claimed ids equals the
	/// set of enqueued ids with no duplicates.
	#[rstest]
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn test_concurrent_dequeue_claims_each_task_once() {
		use tempfile::tempdir;

		// Arrange: a file-backed database so every pooled connection shares the
		// same tasks (an in-memory URL gives each connection its own database).
		let temp_dir = tempdir().expect("Failed to create temp directory");
		let db_path = temp_dir.path().join("concurrent.db");
		let db_url = format!("sqlite:///{}", db_path.display());
		let backend = Arc::new(
			SqliteBackend::new(&db_url)
				.await
				.expect("Failed to create backend"),
		);

		let task_count = 20usize;
		let mut enqueued: HashSet<TaskId> = HashSet::with_capacity(task_count);
		for i in 0..task_count {
			let task = Box::new(TestTask {
				id: TaskId::new(),
				name: format!("task_{i}"),
			});
			enqueued.insert(task.id());
			backend.enqueue(task).await.expect("Failed to enqueue");
		}

		// Act: race twice as many consumers as there are tasks.
		let mut handles = Vec::with_capacity(task_count * 2);
		for _ in 0..task_count * 2 {
			let backend = Arc::clone(&backend);
			handles.push(tokio::spawn(async move { backend.dequeue().await }));
		}

		let mut claimed: Vec<TaskId> = Vec::with_capacity(task_count);
		for handle in handles {
			if let Some(task_id) = handle
				.await
				.expect("dequeue task panicked")
				.expect("dequeue returned an error")
			{
				claimed.push(task_id);
			}
		}

		// Assert: every task claimed exactly once, and no more than were enqueued.
		let claimed_set: HashSet<TaskId> = claimed.iter().copied().collect();
		assert_eq!(
			claimed.len(),
			claimed_set.len(),
			"a task was claimed by more than one consumer"
		);
		assert_eq!(
			claimed_set, enqueued,
			"claimed tasks must match enqueued tasks"
		);
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
}
