//! Native maintenance statements retain quoted identifiers and SQL boundaries.
#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
use reinhardt_db::backends::{AnalyzeBuilder, DatabaseConnection, DatabaseType};
use reinhardt_query::{
	ColumnDef, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuilder, SqliteQueryBuilder,
};
use rstest::rstest;
use testcontainers::{ImageExt, runners::AsyncRunner};

async fn exercise_analyze(connection: DatabaseConnection) {
	// Arrange: quoted schema and all data live on this isolated native owner.
	let dialect = connection.backend().database_type();
	let table = "analysis\"target`";
	let column = "quoted\"column`";
	let statement = Query::create_table()
		.table(table)
		.col(ColumnDef::new(column).integer())
		.to_owned();
	let ddl = match dialect {
		DatabaseType::Postgres => PostgresQueryBuilder.build_create_table(&statement).0,
		DatabaseType::Mysql => MySqlQueryBuilder.build_create_table(&statement).0,
		DatabaseType::Sqlite => SqliteQueryBuilder.build_create_table(&statement).0,
	};
	connection.execute(&ddl, vec![]).await.unwrap();
	// Act
	let result = AnalyzeBuilder::new(connection.backend())
		.table(table)
		.columns(vec![column])
		.verbose(true)
		.execute()
		.await;
	// Assert: PostgreSQL analyzes the quoted column; other backends retain documented ignored options.
	assert!(result.is_ok(), "{result:?}");
	let global = AnalyzeBuilder::new(connection.backend()).execute().await;
	if dialect == DatabaseType::Mysql {
		let error = global.unwrap_err();
		assert_eq!(
			error.database_kind(),
			Some(reinhardt_core::exception::DatabaseErrorKind::Unsupported)
		);
		assert_eq!(
			error.database_error().unwrap().message(),
			"MySQL ANALYZE requires an explicit table; use AnalyzeBuilder::table()"
		);
	} else {
		assert!(global.is_ok());
	}
}

#[rstest]
#[tokio::test]
async fn sqlite_native_analyze_contracts() {
	exercise_analyze(
		DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap(),
	)
	.await;
}

#[rstest]
#[tokio::test]
async fn sqlite_analyze_identifier_cannot_execute_another_statement() {
	// Arrange
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	connection
		.execute("CREATE TABLE safe (id BIGINT PRIMARY KEY)", vec![])
		.await
		.unwrap();
	connection
		.execute("CREATE TABLE audit_probe (id BIGINT PRIMARY KEY)", vec![])
		.await
		.unwrap();
	let identifier = "safe\"; INSERT INTO audit_probe (id) VALUES (7); --";
	// Act
	let result = AnalyzeBuilder::new(connection.backend())
		.table(identifier)
		.execute()
		.await;
	let row = connection
		.fetch_one("SELECT COUNT(*) AS count FROM audit_probe", vec![])
		.await
		.unwrap();
	// Assert: the entire input stays one identifier and cannot write another table.
	assert!(result.is_err());
	assert_eq!(row.get::<i64>("count").unwrap(), 0);
}

#[rstest]
#[tokio::test]
async fn postgres_native_analyze_contracts() {
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
	exercise_analyze(connection).await;
}

#[rstest]
#[tokio::test]
async fn mysql_native_analyze_contracts() {
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
	exercise_analyze(connection).await;
}
