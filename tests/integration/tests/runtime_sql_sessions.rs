//! Native session persistence retains injected connections and bound values.
use reinhardt_auth::sessions::backends::{DatabaseSessionBackend, SessionBackend};
use reinhardt_auth::sessions::cleanup::CleanupableBackend;
use reinhardt_db::backends::DatabaseConnection;
use rstest::rstest;
#[cfg(feature = "mysql")]
use testcontainers::ImageExt;
#[cfg(any(feature = "postgres", feature = "mysql"))]
use testcontainers::runners::AsyncRunner;

async fn exercise_session_lifecycle(owner: DatabaseConnection) {
	// Arrange: the native owner and backend lease isolate each database.
	let backend = DatabaseSessionBackend::from_connection(owner.clone()).unwrap();
	backend.create_table().await.unwrap();
	backend.create_table().await.unwrap();
	let prefix = "tenant' ? $42:";
	let key = format!("{prefix}one");
	let first = serde_json::json!({"text": "quoted' ? $43", "absent": null});
	// Act / Assert: exact data and native UPSERT retain creation metadata.
	backend.save(&key, &first, Some(3600)).await.unwrap();
	assert_eq!(
		backend.load::<serde_json::Value>(&key).await.unwrap(),
		Some(first)
	);
	assert!(backend.exists(&key).await.unwrap());
	let before = backend.get_metadata(&key).await.unwrap().unwrap();
	let changed = serde_json::json!({"text": "changed' ? $44", "items": [1, null, 3]});
	backend.save(&key, &changed, Some(3600)).await.unwrap();
	assert_eq!(
		backend.load::<serde_json::Value>(&key).await.unwrap(),
		Some(changed)
	);
	assert_eq!(
		backend
			.get_metadata(&key)
			.await
			.unwrap()
			.unwrap()
			.created_at,
		before.created_at
	);
	let second = format!("{prefix}two");
	for key in [&second, "other", "expired"] {
		backend
			.save(key, &serde_json::json!({}), Some(3600))
			.await
			.unwrap();
	}
	// Test-owned fixture SQL makes expiration deterministic without a timer.
	owner
		.execute(
			"UPDATE sessions SET expire_date = 0 WHERE session_key = 'expired'",
			vec![],
		)
		.await
		.unwrap();
	assert_eq!(backend.get_all_keys().await.unwrap().len(), 4);
	assert_eq!(backend.cleanup_expired().await.unwrap(), 1);
	assert!(!backend.exists("expired").await.unwrap());
	let mut keys = backend.list_keys_with_prefix(prefix).await.unwrap();
	keys.sort();
	assert_eq!(keys, [key, second]);
	assert_eq!(backend.count_keys_with_prefix(prefix).await.unwrap(), 2);
	assert_eq!(backend.delete_keys_with_prefix(prefix).await.unwrap(), 2);
	assert_eq!(backend.count_keys_with_prefix(prefix).await.unwrap(), 0);
	assert!(backend.exists("other").await.unwrap());
	backend.delete("other").await.unwrap();
	assert!(backend.get_all_keys().await.unwrap().is_empty());
}

#[cfg(feature = "sqlite")]
#[rstest]
#[tokio::test]
async fn sqlite_generated_session_lifecycle() {
	let owner = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	exercise_session_lifecycle(owner).await;
}

#[cfg(feature = "postgres")]
#[rstest]
#[tokio::test]
async fn postgres_generated_session_lifecycle() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let owner = DatabaseConnection::connect_postgres(&format!(
		"postgres://postgres:postgres@127.0.0.1:{port}/postgres"
	))
	.await
	.unwrap();
	exercise_session_lifecycle(owner).await;
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_generated_session_lifecycle() {
	let container = testcontainers_modules::mysql::Mysql::default()
		// Initializing MySQL's data directory can exceed a minute on shared Docker hosts.
		.with_startup_timeout(std::time::Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let owner = DatabaseConnection::connect_mysql(&format!("mysql://root@127.0.0.1:{port}/test"))
		.await
		.unwrap();
	exercise_session_lifecycle(owner).await;
}
