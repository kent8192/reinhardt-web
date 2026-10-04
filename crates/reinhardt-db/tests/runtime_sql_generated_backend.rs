//! Native generated execution preserves values without the legacy ORM converter.
#![cfg(all(
	feature = "backends",
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite"
))]

use reinhardt_db::backends::{DatabaseConnection, DatabaseErrorKind, DatabaseType};
use reinhardt_query::{
	Alias, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryStatementBuilder,
	SqliteQueryBuilder, Value, Values,
};
use rstest::rstest;
use sqlx::Row;
use testcontainers::runners::AsyncRunner;

fn build<T: QueryStatementBuilder>(database: DatabaseType, statement: T) -> (String, Values) {
	match database {
		DatabaseType::Postgres => statement.build(PostgresQueryBuilder),
		DatabaseType::Mysql => statement.build(MySqlQueryBuilder),
		DatabaseType::Sqlite => statement.build(SqliteQueryBuilder),
	}
}

async fn exercise_crud(connection: &DatabaseConnection) {
	let database = connection.database_type();
	// Arrange: fixture SQL is test-owned; generated runtime DML uses the public path.
	let schema = match database {
		DatabaseType::Postgres => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid UUID, amount NUMERIC(30,12))"
		}
		DatabaseType::Mysql => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid CHAR(36), amount DECIMAL(30,12))"
		}
		DatabaseType::Sqlite => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid TEXT, amount REAL)"
		}
	};
	connection.execute(schema, vec![]).await.unwrap();
	let text = "quoted' ? $1 payload";
	let uuid = uuid::Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap();
	let precise = rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap();
	let amount = if database == DatabaseType::Sqlite {
		Value::Double(Some(1.25))
	} else {
		Value::Decimal(Some(Box::new(precise)))
	};
	let built = build(
		database,
		Query::insert()
			.into_table(Alias::new("generated_rows"))
			.columns([
				Alias::new("id"),
				Alias::new("name"),
				Alias::new("absent"),
				Alias::new("uid"),
				Alias::new("amount"),
			])
			.values_panic([
				Value::Int(Some(7)),
				text.into(),
				Value::String(None),
				Value::Uuid(Some(Box::new(uuid))),
				amount,
			])
			.take(),
	);
	assert_eq!(built.1.0.len(), 4);
	assert!(built.0.contains("NULL"));
	// Act
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let select = || {
		build(
			database,
			Query::select()
				.columns([Alias::new("id"), Alias::new("name"), Alias::new("absent")])
				.from(Alias::new("generated_rows"))
				.and_where(Expr::col(Alias::new("name")).eq(text))
				.and_where(Expr::col(Alias::new("uid")).eq(Value::Uuid(Some(Box::new(uuid)))))
				.take(),
		)
	};
	let rows = connection
		.fetch_all_generated(select(), None)
		.await
		.unwrap();
	// Assert: UUID equality still targets legacy text columns on MySQL/SQLite.
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 7);
	assert_eq!(rows[0].get::<String>("name").unwrap(), text);
	assert!(matches!(
		rows[0].data.get("absent"),
		Some(reinhardt_db::backends::QueryValue::Null)
	));
	assert_eq!(
		connection
			.fetch_one_generated(select(), None)
			.await
			.unwrap()
			.get::<String>("name")
			.unwrap(),
		text
	);
	assert!(
		connection
			.fetch_optional_generated(select(), None)
			.await
			.unwrap()
			.is_some()
	);
	// Independently read exact decimal precision through the native SQLx codec.
	match database {
		DatabaseType::Postgres => {
			let stored: rust_decimal::Decimal =
				sqlx::query_scalar("SELECT amount FROM generated_rows")
					.fetch_one(&connection.into_postgres().unwrap())
					.await
					.unwrap();
			assert_eq!(stored, precise);
		}
		DatabaseType::Mysql => {
			let stored: rust_decimal::Decimal =
				sqlx::query_scalar("SELECT amount FROM generated_rows")
					.fetch_one(&connection.into_mysql().unwrap())
					.await
					.unwrap();
			assert_eq!(stored, precise);
		}
		DatabaseType::Sqlite => {}
	}
	let update = build(
		database,
		Query::update()
			.table(Alias::new("generated_rows"))
			.value(Alias::new("name"), "updated' ? $2")
			.and_where(Expr::col(Alias::new("id")).eq(7_i32))
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(update, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	assert!(
		connection
			.fetch_optional_generated(select(), None)
			.await
			.unwrap()
			.is_none()
	);
	let delete = build(
		database,
		Query::delete()
			.from_table(Alias::new("generated_rows"))
			.and_where(Expr::col(Alias::new("id")).eq(7_i32))
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(delete, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let absent = build(
		database,
		Query::select()
			.column(Alias::new("id"))
			.from(Alias::new("generated_rows"))
			.take(),
	);
	assert!(
		connection
			.fetch_all_generated(absent, None)
			.await
			.unwrap()
			.is_empty()
	);
}

#[rstest]
#[tokio::test]
async fn postgres_generated_pool_execution_keeps_decimal_and_nullable_arrays() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	exercise_crud(&connection).await;
	// Arrange / Act: nullable array elements bypass the legacy Debug conversion.
	connection
		.execute("CREATE TABLE generated_arrays (items INTEGER[])", vec![])
		.await
		.unwrap();
	let built = build(
		DatabaseType::Postgres,
		Query::insert()
			.into_table(Alias::new("generated_arrays"))
			.columns([Alias::new("items")])
			.values_panic([Value::Array(
				reinhardt_query::ArrayType::Int,
				Some(Box::new(vec![Value::Int(Some(3)), Value::Int(None)])),
			)])
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let row = sqlx::query("SELECT items FROM generated_arrays")
		.fetch_one(&connection.into_postgres().unwrap())
		.await
		.unwrap();
	// Assert
	assert_eq!(
		row.try_get::<Vec<Option<i32>>, _>("items").unwrap(),
		[Some(3), None]
	);
}

#[rstest]
#[tokio::test]
async fn mysql_generated_pool_execution_keeps_decimal_and_full_unsigned_range() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	let connection = DatabaseConnection::connect_mysql(&url).await.unwrap();
	exercise_crud(&connection).await;
	// Arrange / Act: the native backend must bypass the signed compatibility bridge.
	connection
		.execute(
			"CREATE TABLE generated_unsigned (amount BIGINT UNSIGNED)",
			vec![],
		)
		.await
		.unwrap();
	let built = build(
		DatabaseType::Mysql,
		Query::insert()
			.into_table(Alias::new("generated_unsigned"))
			.columns([Alias::new("amount")])
			.values_panic([u64::MAX])
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let stored: u64 = sqlx::query_scalar("SELECT amount FROM generated_unsigned")
		.fetch_one(&connection.into_mysql().unwrap())
		.await
		.unwrap();
	// Assert
	assert_eq!(stored, u64::MAX);
}

#[rstest]
#[tokio::test]
async fn sqlite_generated_pool_execution_keeps_text_uuid_and_rejects_lossy_values() {
	let pool = sqlx::sqlite::SqlitePoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.unwrap();
	let connection = DatabaseConnection::from_sqlite_pool(pool);
	exercise_crud(&connection).await;
	// Act: failed encoding must occur before an invalid SQL statement is executed.
	let built = build(
		DatabaseType::Sqlite,
		Query::select()
			.expr(Expr::val("private payload"))
			.expr(Expr::val(u64::MAX))
			.take(),
	);
	let error = connection
		.fetch_all_generated(built, None)
		.await
		.expect_err("overflow must fail");
	// Assert
	assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
	let message = error.to_string();
	assert!(message.contains("BigUnsigned argument 2 for sqlite"));
	assert!(!message.contains("private payload"));
	assert!(!message.contains(&u64::MAX.to_string()));
}
