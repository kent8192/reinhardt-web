#![cfg(feature = "backends")]

#[cfg(all(feature = "orm", feature = "associations", feature = "sqlite"))]
mod sqlite {
	use reinhardt_db::{
		associations::many_to_many_manager::ManyToManyManager, orm::connection::DatabaseConnection,
	};
	use reinhardt_query::{
		ColumnDef, Query, QueryBuilder, QueryStatementBuilder, SqliteQueryBuilder,
	};
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn association_manager_native_execution_preserves_string_parameters() {
		// Arrange
		let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap();
		let create = Query::create_table()
			.table("links")
			.col(ColumnDef::new("source").string())
			.col(ColumnDef::new("target").string())
			.primary_key(["source", "target"])
			.to_owned();
		let (sql, _) = SqliteQueryBuilder.build_create_table(&create);
		db.execute(&sql, vec![]).await.unwrap();
		let source = "quoted \"source\",slash\\日本語";
		let target = "target'; DROP TABLE links; --";
		let manager = ManyToManyManager::<(), (), &str>::new(
			source,
			"links".to_owned(),
			"source".to_owned(),
			"target".to_owned(),
		);

		// Act
		manager.add_with_db(&db, target).await.unwrap();
		manager.add_with_db(&db, target).await.unwrap();
		let (sql, _) = Query::select()
			.from("links")
			.columns(["source", "target"])
			.build(SqliteQueryBuilder);
		let rows = db.query(&sql, vec![]).await.unwrap();

		// Assert
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].data["source"], serde_json::json!(source));
		assert_eq!(rows[0].data["target"], serde_json::json!(target));
	}
}

#[cfg(feature = "postgres")]
mod postgres {
	use reinhardt_db::backends::{DatabaseBackend, DatabaseType, PostgresBackend, QueryValue};
	use reinhardt_query::{
		ColumnDef, ColumnType, Expr, PostgresQueryBuilder, Query, QueryBuilder,
		QueryStatementBuilder, Value,
	};
	use rstest::{fixture, rstest};
	use sqlx::{Column, Row, TypeInfo};
	use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	#[fixture]
	async fn database() -> (ContainerAsync<Postgres>, PostgresBackend) {
		let container = Postgres::default()
			.with_tag("16-alpine")
			.start()
			.await
			.unwrap();
		let url = format!(
			"postgres://postgres:postgres@{}:{}/postgres",
			container.get_host().await.unwrap(),
			container.get_host_port_ipv4(5432).await.unwrap()
		);
		let pool = sqlx::PgPool::connect(&url).await.unwrap();
		(container, PostgresBackend::new(pool))
	}

	#[rstest]
	#[tokio::test]
	async fn generated_insert_retains_decimal_array_and_temporal_types(
		#[future] database: (ContainerAsync<Postgres>, PostgresBackend),
	) {
		// Arrange
		let (_container, backend) = database.await;
		let create = Query::create_table()
			.table("bind_probe")
			.col(ColumnDef::new("decimal").decimal(28, 15))
			.col(ColumnDef::new("big_decimal").decimal(60, 30))
			.col(ColumnDef::new("strings").array(ColumnType::Text))
			.col(ColumnDef::new("empty_array").array(ColumnType::Integer))
			.col(ColumnDef::new("null_array").array(ColumnType::Integer))
			.col(ColumnDef::new("json_array").array(ColumnType::Json))
			.col(ColumnDef::new("jsonb_array").array(ColumnType::JsonBinary))
			.col(ColumnDef::new("date").date())
			.col(ColumnDef::new("time").time())
			.to_owned();
		let (sql, _) = PostgresQueryBuilder.build_create_table(&create);
		backend.execute(&sql, vec![]).await.unwrap();
		let decimal: rust_decimal::Decimal = "1234567890123.123456789012345".parse().unwrap();
		let big_decimal: sqlx::types::BigDecimal =
			"12345678901234567890123456789.12345678901234567890123456789"
				.parse()
				.unwrap();
		let strings = vec![
			Some("quoted \"element\",slash\\".to_owned()),
			None,
			Some("日本語".to_owned()),
		];
		let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
		let time = chrono::NaiveTime::from_hms_micro_opt(1, 2, 3, 456_789).unwrap();
		let json = serde_json::json!({"quoted": "\"comma,slash\\", "number": 123});
		let statement = Query::insert()
			.into_table("bind_probe")
			.columns([
				"decimal",
				"big_decimal",
				"strings",
				"empty_array",
				"null_array",
				"json_array",
				"jsonb_array",
				"date",
				"time",
			])
			.values_panic([
				decimal.into(),
				big_decimal.clone().into(),
				Value::Array(
					reinhardt_query::ArrayType::String,
					Some(Box::new(
						strings
							.iter()
							.map(|v| Value::String(v.clone().map(Box::new)))
							.collect(),
					)),
				),
				Value::Array(reinhardt_query::ArrayType::Int, Some(Box::default())),
				Value::Array(reinhardt_query::ArrayType::Int, None),
				Value::Array(
					reinhardt_query::ArrayType::Json,
					Some(Box::new(vec![json.clone().into()])),
				),
				Value::Array(
					reinhardt_query::ArrayType::Jsonb,
					Some(Box::new(vec![json.clone().into()])),
				),
				date.into(),
				time.into(),
			])
			.to_owned();
		let (sql, values) = statement.build(PostgresQueryBuilder);

		// Act
		let result = backend.__execute_generated(&sql, values).await.unwrap();
		let (sql, _) = Query::select()
			.from("bind_probe")
			.expr(Expr::asterisk())
			.build(PostgresQueryBuilder);
		let row = sqlx::query(&sql).fetch_one(backend.pool()).await.unwrap();

		// Assert
		assert_eq!(result.rows_affected, 1);
		assert_eq!(row.get::<rust_decimal::Decimal, _>("decimal"), decimal);
		assert_eq!(
			row.get::<sqlx::types::BigDecimal, _>("big_decimal"),
			big_decimal
		);
		assert_eq!(row.get::<Vec<Option<String>>, _>("strings"), strings);
		assert_eq!(row.get::<Vec<i32>, _>("empty_array"), Vec::<i32>::new());
		assert_eq!(row.get::<Option<Vec<i32>>, _>("null_array"), None);
		assert_eq!(
			row.get::<Vec<sqlx::types::Json<serde_json::Value>>, _>("json_array"),
			vec![sqlx::types::Json(json.clone())]
		);
		assert_eq!(
			row.get::<Vec<sqlx::types::Json<serde_json::Value>>, _>("jsonb_array"),
			vec![sqlx::types::Json(json)]
		);
		assert_eq!(row.column("json_array").type_info().name(), "JSON[]");
		assert_eq!(row.column("jsonb_array").type_info().name(), "JSONB[]");
		assert_eq!(row.get::<chrono::NaiveDate, _>("date"), date);
		assert_eq!(row.get::<chrono::NaiveTime, _>("time"), time);
	}

	#[rstest]
	#[tokio::test]
	async fn generated_transaction_keeps_rollback_and_rejects_overflow(
		#[future] database: (ContainerAsync<Postgres>, PostgresBackend),
	) {
		// Arrange
		let (_container, backend) = database.await;
		let create = Query::create_table()
			.table("transaction_probe")
			.col(ColumnDef::new("value").big_integer())
			.to_owned();
		let (sql, _) = PostgresQueryBuilder.build_create_table(&create);
		backend.execute(&sql, vec![]).await.unwrap();
		let mut transaction = backend.begin().await.unwrap();
		let statement = Query::insert()
			.into_table("transaction_probe")
			.columns(["value"])
			.values_panic([Value::BigUnsigned(Some(u64::MAX))])
			.to_owned();
		let (sql, values) = statement.build(PostgresQueryBuilder);

		// Act
		let error = transaction
			.__execute_generated(&sql, values, DatabaseType::Postgres)
			.await
			.unwrap_err();
		let statement = Query::insert()
			.into_table("transaction_probe")
			.columns(["value"])
			.values_panic([Value::BigInt(Some(42))])
			.to_owned();
		let (sql, values) = statement.build(PostgresQueryBuilder);
		let result = transaction
			.__execute_generated(&sql, values, DatabaseType::Postgres)
			.await
			.unwrap();
		transaction.rollback().await.unwrap();
		let (sql, _) = Query::select()
			.from("transaction_probe")
			.column("value")
			.build(PostgresQueryBuilder);

		// Assert
		assert_eq!(
			error.to_string(),
			"Type conversion error: cannot encode BigUnsigned argument 1 for postgres: unsigned integer exceeds signed 64-bit range"
		);
		assert_eq!(result.rows_affected, 1);
		assert_eq!(backend.fetch_all(&sql, vec![]).await.unwrap(), vec![]);
		let (sql, _) = Query::select()
			.expr(Expr::val(42i64))
			.build(PostgresQueryBuilder);
		assert_eq!(
			backend
				.fetch_one(&sql, vec![QueryValue::Int(42)])
				.await
				.unwrap()
				.data
				.into_values()
				.collect::<Vec<_>>(),
			vec![QueryValue::Int(42)]
		);
	}
}
