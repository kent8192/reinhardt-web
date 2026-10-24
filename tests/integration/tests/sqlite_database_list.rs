//! SQLite database-list result and durable-pool regressions for issue #6508.

use reinhardt_query::{Query, QueryStatementBuilder, SqliteQueryBuilder};
use reinhardt_tasks::durable::{DurableQueue, JobSpec, JobState, SqliteDurableJobStore};
use reinhardt_test::fixtures::temp_dir;
use rstest::*;
use sqlx::{Column, Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};
use tempfile::TempDir;

async fn database_list(connection: &mut SqliteConnection) -> Vec<(i64, String, String)> {
	let (sql, values) = Query::sqlite_database_list()
		.build_sqlite_checked()
		.unwrap();
	assert!(values.is_empty());
	let rows = sqlx::query(&sql).fetch_all(connection).await.unwrap();
	rows.into_iter()
		.map(|row| {
			assert_eq!(
				row.columns().iter().map(Column::name).collect::<Vec<_>>(),
				["seq", "name", "file"]
			);
			(
				row.try_get("seq").unwrap(),
				row.try_get("name").unwrap(),
				row.try_get("file").unwrap(),
			)
		})
		.collect()
}

#[rstest]
#[case::in_memory(true)]
#[case::file_backed(false)]
#[tokio::test]
async fn database_list_preserves_main_temp_and_attachment_results(
	temp_dir: TempDir,
	#[case] in_memory: bool,
) {
	// Arrange
	let directory = temp_dir.path().canonicalize().unwrap();
	let main_file = directory.join("main.sqlite3");
	let attached_file = directory.join("attached.sqlite3");
	let options = if in_memory {
		// ATTACH inherits SQLITE_OPEN_MEMORY, so use the special main filename
		// without that flag to keep the file attachment backed by a file.
		SqliteConnectOptions::new()
			.filename(":memory:")
			.create_if_missing(true)
	} else {
		SqliteConnectOptions::new()
			.filename(&main_file)
			.create_if_missing(true)
	};
	let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
	for (filename, name) in [
		(attached_file.to_str().unwrap(), "aux_file"),
		(":memory:", "aux_memory"),
	] {
		let (sql, values) = Query::attach_database()
			.file_path(filename)
			.as_name(name)
			.build(SqliteQueryBuilder);
		assert!(values.is_empty());
		sqlx::query(&sql).execute(&mut connection).await.unwrap();
	}
	sqlx::query("CREATE TEMP TABLE connection_probe (id INTEGER)")
		.execute(&mut connection)
		.await
		.unwrap();

	// Act
	let rows = database_list(&mut connection).await;

	// Assert
	let main_filename = if in_memory {
		String::new()
	} else {
		main_file.to_str().unwrap().to_owned()
	};
	assert_eq!(
		rows,
		vec![
			(0, "main".to_owned(), main_filename),
			(1, "temp".to_owned(), String::new()),
			(
				2,
				"aux_file".to_owned(),
				attached_file.to_str().unwrap().to_owned()
			),
			(3, "aux_memory".to_owned(), String::new()),
		]
	);
	assert!(attached_file.is_file());
}

#[rstest]
#[tokio::test]
async fn database_list_is_connection_local(temp_dir: TempDir) {
	// Arrange
	let main_file = temp_dir.path().canonicalize().unwrap().join("main.sqlite3");
	let options = SqliteConnectOptions::new()
		.filename(&main_file)
		.create_if_missing(true);
	let mut first = SqliteConnection::connect_with(&options).await.unwrap();
	let mut second = SqliteConnection::connect_with(&options).await.unwrap();
	let (sql, _) = Query::attach_database()
		.file_path(":memory:")
		.as_name("local_attachment")
		.build(SqliteQueryBuilder);
	sqlx::query(&sql).execute(&mut first).await.unwrap();

	// Act
	let first_rows = database_list(&mut first).await;
	let second_rows = database_list(&mut second).await;

	// Assert
	let main = (0, "main".to_owned(), main_file.to_str().unwrap().to_owned());
	assert_eq!(
		first_rows,
		vec![
			main.clone(),
			(2, "local_attachment".to_owned(), String::new())
		]
	);
	assert_eq!(second_rows, vec![main]);
}

#[rstest]
#[tokio::test]
async fn durable_pool_accepts_file_main_with_in_memory_attachment(temp_dir: TempDir) {
	// Arrange
	let main_file = temp_dir.path().canonicalize().unwrap().join("main.sqlite3");
	let pool = sqlx::sqlite::SqlitePoolOptions::new()
		.max_connections(1)
		.connect_with(
			SqliteConnectOptions::new()
				.filename(&main_file)
				.create_if_missing(true),
		)
		.await
		.unwrap();
	{
		let mut connection = pool.acquire().await.unwrap();
		let (sql, _) = Query::attach_database()
			.file_path(":memory:")
			.as_name("aux_memory")
			.build(SqliteQueryBuilder);
		sqlx::query(&sql).execute(&mut *connection).await.unwrap();
		let rows = database_list(&mut connection).await;
		assert_eq!(
			rows,
			vec![
				(0, "main".to_owned(), main_file.to_str().unwrap().to_owned()),
				(2, "aux_memory".to_owned(), String::new()),
			]
		);
	}

	// Act
	let store = SqliteDurableJobStore::from_pool(pool).await.unwrap();
	let queue = DurableQueue::new(store);
	let job = queue.enqueue(JobSpec::new("send_email")).await.unwrap();
	let claimed = queue.claim_next().await.unwrap().unwrap();

	// Assert
	assert_eq!(job.state, JobState::Queued);
	assert_eq!(claimed.id(), job.id);
}
