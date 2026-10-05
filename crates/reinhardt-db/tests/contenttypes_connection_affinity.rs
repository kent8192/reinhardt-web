//! Regression coverage for connection-local SQLite content type IDs.

#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use reinhardt_db::contenttypes::ContentType;
use reinhardt_db::contenttypes::orm_integration::ContentTypeTransaction;
use reinhardt_db::contenttypes::persistence::{
	ContentTypePersistence, ContentTypePersistenceBackend, PersistenceError,
};
use rstest::{fixture, rstest};
use sqlx::AnyPool;
use sqlx::any::AnyPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

struct SqliteContentTypes {
	persistence: ContentTypePersistence,
	pool: Arc<AnyPool>,
	_directory: TempDir,
}

#[fixture]
async fn sqlite_content_types() -> SqliteContentTypes {
	let temp_dir = tempfile::tempdir().expect("Failed to create temporary SQLite directory");
	sqlx::any::install_default_drivers();
	let database_path = temp_dir.path().join("contenttypes.sqlite");
	let url = format!("sqlite://{}?mode=rwc", database_path.display());
	let pool = Arc::new(
		AnyPoolOptions::new()
			.min_connections(2)
			.max_connections(2)
			.acquire_timeout(Duration::from_secs(5))
			.connect(&url)
			.await
			.expect("Failed to open two-connection SQLite pool"),
	);
	let persistence = ContentTypePersistence::from_pool(pool.clone(), &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create content type table");
	SqliteContentTypes {
		persistence,
		pool,
		_directory: temp_dir,
	}
}

#[derive(Clone, Copy)]
enum InsertPath {
	Persistence,
	Transaction,
}

impl InsertPath {
	async fn insert(
		self,
		database: &SqliteContentTypes,
		model: &str,
	) -> Result<ContentType, PersistenceError> {
		match self {
			Self::Persistence => {
				database
					.persistence
					.save(&ContentType::new("affinity", model))
					.await
			}
			Self::Transaction => {
				ContentTypeTransaction::new(database.pool.clone())
					.create("affinity", model)
					.await
			}
		}
	}
}

#[rstest]
#[case::persistence(InsertPath::Persistence)]
#[case::transaction(InsertPath::Transaction)]
#[tokio::test]
async fn inserted_ids_match_stored_rows(
	#[future] sqlite_content_types: SqliteContentTypes,
	#[case] insert_path: InsertPath,
) {
	// Arrange
	let database = sqlite_content_types.await;
	let mut saved = Vec::new();

	// Act
	for index in 0..8 {
		let model = format!("Probe{index}");
		saved.push(
			insert_path
				.insert(&database, &model)
				.await
				.expect("Failed to insert content type"),
		);
	}

	// Assert
	for (index, saved) in saved.iter().enumerate() {
		let stored = database
			.persistence
			.get(&saved.app_label, &saved.model)
			.await
			.expect("Failed to retrieve inserted content type")
			.expect("Inserted content type is missing");
		assert_eq!(saved.id, Some(index as i64 + 1));
		assert_eq!(saved, &stored);
		assert_eq!(
			database
				.persistence
				.get_by_id(saved.id.unwrap())
				.await
				.unwrap(),
			Some(stored),
		);
	}
}

#[rstest]
#[case::persistence(InsertPath::Persistence)]
#[case::transaction(InsertPath::Transaction)]
#[tokio::test]
async fn concurrent_inserts_return_their_own_ids(
	#[future] sqlite_content_types: SqliteContentTypes,
	#[case] insert_path: InsertPath,
) {
	// Arrange
	let database = sqlite_content_types.await;

	// Act
	let (first, second) = tokio::try_join!(
		insert_path.insert(&database, "First"),
		insert_path.insert(&database, "Second"),
	)
	.expect("Failed to insert concurrent content types");

	// Assert
	let mut ids = [first.id, second.id];
	ids.sort();
	assert_eq!(ids, [Some(1), Some(2)]);
	for saved in [first, second] {
		assert_eq!(
			database
				.persistence
				.get(&saved.app_label, &saved.model)
				.await
				.unwrap(),
			Some(saved),
		);
	}
}

#[rstest]
#[case::persistence(InsertPath::Persistence)]
#[case::transaction(InsertPath::Transaction)]
#[tokio::test]
async fn failed_insert_returns_connections_and_preserves_existing_row(
	#[future] sqlite_content_types: SqliteContentTypes,
	#[case] insert_path: InsertPath,
) {
	// Arrange
	let database = sqlite_content_types.await;
	let existing = insert_path
		.insert(&database, "Unique")
		.await
		.expect("Failed to insert original content type");

	// Act
	let error = insert_path
		.insert(&database, "Unique")
		.await
		.expect_err("Duplicate content type must fail");

	// Assert
	assert!(matches!(error, PersistenceError::DatabaseError(_)));
	{
		let _first_connection = database.pool.acquire().await.unwrap();
		let _second_connection = database.pool.acquire().await.unwrap();
	}
	assert_eq!(
		database
			.persistence
			.get("affinity", "Unique")
			.await
			.unwrap(),
		Some(existing),
	);
	let next = insert_path
		.insert(&database, "AfterFailure")
		.await
		.expect("Failed to insert after duplicate error");
	assert_eq!(next.id, Some(2));
	assert_eq!(database.persistence.load_all().await.unwrap().len(), 2);
}
