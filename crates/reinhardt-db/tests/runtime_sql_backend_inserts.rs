//! Native INSERT builders retain source arguments and explicit conflict semantics.
#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]

use reinhardt_db::backends::query_builder::{InsertFromSelectBuilder, OnConflictClause};
use reinhardt_db::backends::{DatabaseConnection, DatabaseType, InsertBuilder, QueryValue};
use reinhardt_query::{Expr, Query};
use rstest::rstest;
use testcontainers::{ImageExt, runners::AsyncRunner};

async fn exercise_inserts(connection: DatabaseConnection) {
	// Arrange: this test owns each native executor and its complete schema.
	let backend = connection.backend();
	let dialect = backend.database_type();
	let (timestamp, array) = match dialect {
		DatabaseType::Postgres => ("TIMESTAMP", "TEXT[]"),
		DatabaseType::Mysql => ("DATETIME", "TEXT"),
		DatabaseType::Sqlite => ("TEXT", "TEXT"),
	};
	for table in ["insert_values", "insert_copy"] {
		connection.execute(&format!("CREATE TABLE {table} (id BIGINT PRIMARY KEY, name TEXT, touched {timestamp}, nullable TEXT, items {array})"), vec![]).await.unwrap();
	}
	let payload = "insert' ? $42";
	let items = vec!["element' ? $43".to_owned()];
	// Act: inline NULL and time expressions precede and follow bound inputs.
	let result = InsertBuilder::new(backend.clone(), "insert_values")
		.value("nullable", QueryValue::Null)
		.value("id", 1_i64)
		.value("touched", QueryValue::Now)
		.value("name", payload)
		.value("items", QueryValue::StringArray(items.clone()))
		.execute()
		.await
		.unwrap();
	// Assert
	assert_eq!(result.rows_affected, 1);
	let row = connection
		.fetch_one("SELECT * FROM insert_values WHERE id = 1", vec![])
		.await
		.unwrap();
	assert_eq!(row.get::<i64>("id").unwrap(), 1);
	assert_eq!(row.get::<String>("name").unwrap(), payload);
	assert_eq!(row.data.get("nullable"), Some(&QueryValue::Null));
	assert_ne!(row.data.get("touched"), Some(&QueryValue::Null));
	if dialect == DatabaseType::Postgres {
		assert_eq!(row.data.get("items"), Some(&QueryValue::StringArray(items)));
	} else {
		assert_eq!(
			serde_json::from_str::<Vec<String>>(&row.get::<String>("items").unwrap()).unwrap(),
			items
		);
	}
	// Act / Assert: RETURNING follows the conflict action on supported backends.
	let replacement = "updated' ? $44";
	let upsert = InsertBuilder::new(backend.clone(), "insert_values")
		.value("id", 1_i64)
		.value("name", replacement)
		.on_conflict_do_update(Some(vec!["id".into()]), vec!["name".into()])
		.returning(vec!["id", "name"]);
	if dialect == DatabaseType::Mysql {
		assert!(upsert.execute().await.unwrap().rows_affected > 0);
	} else {
		let returned = upsert.fetch_one().await.unwrap();
		assert_eq!(returned.get::<i64>("id").unwrap(), 1);
		assert_eq!(returned.get::<String>("name").unwrap(), replacement);
	}
	// Explicit DO NOTHING retains the backend's documented ignore behavior.
	let ignored = InsertBuilder::new(backend.clone(), "insert_values")
		.value("id", 1_i64)
		.value("name", "must stay ignored")
		.on_conflict_do_nothing(None)
		.execute()
		.await
		.unwrap();
	assert_eq!(ignored.rows_affected, 0);
	let condition = InsertBuilder::new(backend.clone(), "insert_values")
		.value("id", 1_i64)
		.value("name", "blocked")
		.on_conflict(
			OnConflictClause::columns(vec!["id"])
				.do_update(vec!["name"])
				.where_clause("1 = 0"),
		);
	if dialect == DatabaseType::Mysql {
		assert!(
			condition
				.execute()
				.await
				.unwrap_err()
				.to_string()
				.contains("MySQL does not support conditional")
		);
		let named = InsertBuilder::new(backend.clone(), "insert_values")
			.value("id", 1_i64)
			.on_conflict(
				OnConflictClause::constraint("insert_values_pkey").do_update(vec!["name"]),
			);
		assert!(
			named
				.execute()
				.await
				.unwrap_err()
				.to_string()
				.contains("ON CONFLICT ON CONSTRAINT is unsupported")
		);
	} else {
		assert_eq!(condition.execute().await.unwrap().rows_affected, 0);
	}
	let retained = connection
		.fetch_one("SELECT name FROM insert_values WHERE id = 1", vec![])
		.await
		.unwrap();
	assert_eq!(retained.get::<String>("name").unwrap(), replacement);
	if dialect == DatabaseType::Postgres {
		let row = InsertBuilder::new(backend.clone(), "insert_values")
			.value("id", 1_i64)
			.value("name", replacement)
			.on_conflict(OnConflictClause::constraint("insert_values_pkey").do_update(vec!["name"]))
			.returning(vec!["id"])
			.fetch_one()
			.await
			.unwrap();
		assert_eq!(row.get::<i64>("id").unwrap(), 1);
	}
	if dialect == DatabaseType::Sqlite {
		let missing = InsertBuilder::new(backend.clone(), "insert_values")
			.value("id", 1_i64)
			.on_conflict(OnConflictClause::any().do_update(vec!["name"]));
		assert!(
			missing
				.execute()
				.await
				.unwrap_err()
				.to_string()
				.contains("non-empty conflict_columns")
		);
	}
	// Bound SELECT expressions execute through the exact generated arguments.
	let source_payload = "source' ? $99";
	let insert = InsertFromSelectBuilder::new(
		backend.clone(),
		"insert_copy",
		vec!["id", "name"],
		Query::select()
			.expr(Expr::val(2_i64))
			.expr(Expr::val(source_payload))
			.to_owned(),
	);
	assert_eq!(insert.execute().await.unwrap().rows_affected, 1);
	let row = connection
		.fetch_one("SELECT id,name FROM insert_copy WHERE id = 2", vec![])
		.await
		.unwrap();
	assert_eq!(row.get::<String>("name").unwrap(), source_payload);
	// SQLite requires an unambiguous typed guard when FROM has no WHERE.
	let select = Query::select()
		.columns(["id", "name"])
		.from("insert_values")
		.to_owned();
	let insert =
		InsertFromSelectBuilder::new(backend.clone(), "insert_copy", vec!["id", "name"], select)
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["name".into()]);
	assert_eq!(insert.execute().await.unwrap().rows_affected, 1);
	let row = connection
		.fetch_one("SELECT name FROM insert_copy WHERE id = 1", vec![])
		.await
		.unwrap();
	assert_eq!(row.get::<String>("name").unwrap(), replacement);
	// Converting a configured builder retains its fluent conflict and RETURNING.
	let select = Query::select()
		.expr(Expr::val(2_i64))
		.expr(Expr::val("converted' ? $100"))
		.to_owned();
	let converted = InsertBuilder::new(backend, "insert_copy")
		.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]))
		.returning(vec!["id", "name"])
		.from_select(vec!["id", "name"], select);
	if dialect == DatabaseType::Mysql {
		assert!(converted.execute().await.unwrap().rows_affected > 0);
	} else {
		let returned = converted.fetch_one().await.unwrap();
		assert_eq!(returned.get::<String>("name").unwrap(), "converted' ? $100");
	}
	let row = connection
		.fetch_one("SELECT name FROM insert_copy WHERE id = 2", vec![])
		.await
		.unwrap();
	assert_eq!(row.get::<String>("name").unwrap(), "converted' ? $100");
}

#[rstest]
#[tokio::test]
async fn sqlite_compound_source_guard_preserves_rows() {
	// Arrange
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	connection
		.execute(
			"CREATE TABLE source (id BIGINT PRIMARY KEY, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	connection
		.execute("INSERT INTO source VALUES (1,'one')", vec![])
		.await
		.unwrap();
	connection
		.execute(
			"CREATE TABLE target (id BIGINT PRIMARY KEY, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	let source = Query::select()
		.columns(["id", "name"])
		.from("source")
		.union_all(
			Query::select()
				.expr(Expr::val(2_i64))
				.expr(Expr::val("compound' ? $19"))
				.to_owned(),
		)
		.to_owned();
	let insert =
		InsertFromSelectBuilder::new(connection.backend(), "target", vec!["id", "name"], source)
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["name".into()]);
	// Act
	let result = insert.execute().await.unwrap();
	let rows = connection
		.fetch_all("SELECT id,name FROM target ORDER BY id", vec![])
		.await
		.unwrap();
	// Assert
	assert_eq!(result.rows_affected, 2);
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 1);
	assert_eq!(rows[1].get::<i64>("id").unwrap(), 2);
	assert_eq!(rows[1].get::<String>("name").unwrap(), "compound' ? $19");
}

#[rstest]
#[tokio::test]
async fn sqlite_native_insert_contracts() {
	exercise_inserts(
		DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap(),
	)
	.await;
}

#[rstest]
#[tokio::test]
async fn postgres_native_insert_contracts() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let connection = DatabaseConnection::connect_postgres(&format!(
		"postgres://postgres:postgres@127.0.0.1:{port}/postgres"
	))
	.await
	.unwrap();
	exercise_inserts(connection).await;
}

#[rstest]
#[tokio::test]
async fn mysql_native_insert_contracts() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.with_startup_timeout(std::time::Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let connection =
		DatabaseConnection::connect_mysql(&format!("mysql://root@127.0.0.1:{port}/test"))
			.await
			.unwrap();
	exercise_inserts(connection).await;
}
