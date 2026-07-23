//! MySQL JSON transaction integration tests.

use reinhardt_db::backends::types::QueryValue;
use reinhardt_db::{
	backends::DatabaseConnection as BackendsConnection,
	orm::{DatabaseConnectionLease, OrmExecutor},
};
use reinhardt_test::fixtures::testcontainers::mysql_container;
use rstest::rstest;
use sqlx::MySqlPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[rstest]
#[tokio::test]
async fn test_mysql_transaction_fetch_preserves_json_value(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, url) = mysql_container.await;
	let owner = BackendsConnection::connect(&url).await.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let connection = lease.handle();
	connection
		.execute(
			"CREATE TABLE json_transaction_rows (id BIGINT PRIMARY KEY, payload JSON NOT NULL)",
			vec![],
		)
		.await
		.unwrap();
	let expected = QueryValue::Json(Some(Box::new(serde_json::json!({
		"status": "draft"
	}))));
	connection
		.execute(
			"INSERT INTO json_transaction_rows (id, payload) VALUES (?, ?)",
			vec![QueryValue::Int(1), expected.clone()],
		)
		.await
		.unwrap();
	let row = connection
		.atomic(async |transaction| {
			transaction
				.fetch_one(
					"SELECT payload FROM json_transaction_rows WHERE id = ?",
					vec![QueryValue::Int(1)],
				)
				.await
		})
		.await
		.unwrap();

	// Assert
	assert_eq!(row.data.get("payload"), Some(&expected));
}
