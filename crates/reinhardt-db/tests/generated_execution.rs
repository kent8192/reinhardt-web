#![cfg(feature = "backends")]

#[cfg(all(
	feature = "orm",
	feature = "associations",
	any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
mod associations {
	use reinhardt_db::{
		associations::many_to_many_manager::ManyToManyManager,
		backends::{QueryValue, sql_build_helpers},
		orm::connection::DatabaseConnection,
	};
	use reinhardt_query::{ColumnDef, Query};

	pub(super) async fn verify_manager_lifecycle(db: &DatabaseConnection) {
		// Arrange
		let backend = db.inner().database_type();
		let links = Query::create_table()
			.table("links")
			.col(ColumnDef::new("source").string())
			.col(ColumnDef::new("target").string())
			.primary_key(["source", "target"])
			.to_owned();
		let targets = Query::create_table()
			.table("targets")
			.col(ColumnDef::new("id").string().primary_key(true))
			.col(ColumnDef::new("label").string())
			.to_owned();
		for statement in [links, targets] {
			let (sql, _) = sql_build_helpers::build_create_table(backend, &statement);
			db.execute(&sql, vec![]).await.unwrap();
		}
		let source = "quoted \"source\",slash\\日本語";
		let target = "target'; DROP TABLE links; --";
		let second_target = "second \"target\",slash\\日本語";
		for (id, label) in [(target, "first label"), (second_target, "second label")] {
			let statement = Query::insert()
				.into_table("targets")
				.columns(["id", "label"])
				.values_panic([id, label])
				.to_owned();
			let (sql, _) = sql_build_helpers::build_insert(backend, &statement);
			db.execute(
				&sql,
				vec![
					QueryValue::String(id.into()),
					QueryValue::String(label.into()),
				],
			)
			.await
			.unwrap();
		}
		let manager = |source| {
			ManyToManyManager::<(), (), &str>::new(
				source,
				"links".into(),
				"source".into(),
				"target".into(),
			)
		};
		let current = manager(source);
		let other = manager("unrelated source");
		let empty = manager("empty source");

		// Add and read relationships, preserving bound keys and source isolation.
		current.add_with_db(db, target).await.unwrap();
		current.add_with_db(db, target).await.unwrap();
		current.add_with_db(db, second_target).await.unwrap();
		other.add_with_db(db, target).await.unwrap();
		assert!(current.contains_with_db(db, target).await.unwrap());
		assert!(current.contains_with_db(db, second_target).await.unwrap());
		assert!(
			!current
				.contains_with_db(db, "missing target")
				.await
				.unwrap()
		);
		assert_eq!(current.count_with_db(db).await.unwrap(), 2);
		assert_eq!(other.count_with_db(db).await.unwrap(), 1);
		assert_eq!(empty.count_with_db(db).await.unwrap(), 0);
		let rows = current.all_with_db(db, "targets", "id").await.unwrap();
		let mut actual: Vec<_> = rows
			.iter()
			.map(|row| {
				(
					row.get::<String>("id").unwrap(),
					row.get::<String>("label").unwrap(),
				)
			})
			.collect();
		actual.sort();
		let mut expected = vec![
			(target.to_owned(), "first label".to_owned()),
			(second_target.to_owned(), "second label".to_owned()),
		];
		expected.sort();
		assert_eq!(actual, expected);

		// Remove one relationship without deleting the other source's link.
		current.remove_with_db(db, target).await.unwrap();
		current.remove_with_db(db, "missing target").await.unwrap();
		assert!(!current.contains_with_db(db, target).await.unwrap());
		assert!(current.contains_with_db(db, second_target).await.unwrap());
		assert_eq!(current.count_with_db(db).await.unwrap(), 1);
		assert!(other.contains_with_db(db, target).await.unwrap());
		let rows = current.all_with_db(db, "targets", "id").await.unwrap();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].get::<String>("id").unwrap(), second_target);

		// Clear only the selected source, including repeated empty clears.
		current.clear_with_db(db).await.unwrap();
		current.clear_with_db(db).await.unwrap();
		assert_eq!(current.count_with_db(db).await.unwrap(), 0);
		assert!(!current.contains_with_db(db, second_target).await.unwrap());
		assert!(
			current
				.all_with_db(db, "targets", "id")
				.await
				.unwrap()
				.is_empty()
		);
		assert_eq!(other.count_with_db(db).await.unwrap(), 1);
		assert!(other.contains_with_db(db, target).await.unwrap());
	}
}

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
	async fn association_manager_generated_operations_preserve_bound_keys() {
		let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap();
		super::associations::verify_manager_lifecycle(&db).await;
	}

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
		ColumnDef, ColumnType, Expr, ExprTrait, PostgresQueryBuilder, Query, QueryBuilder,
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
		let pool = sqlx::postgres::PgPoolOptions::new()
			.max_connections(1)
			.connect(&url)
			.await
			.unwrap();
		(container, PostgresBackend::new(pool))
	}

	#[rstest]
	#[tokio::test]
	async fn generated_execution_preserves_changing_parameter_signatures(
		#[future] database: (ContainerAsync<Postgres>, PostgresBackend),
		#[values("execute", "fetch_one", "fetch_all", "transaction")] operation: &str,
	) {
		// Arrange: one pooled connection makes statement-cache reuse deterministic.
		let (_container, backend) = database.await;
		let table = Query::create_table()
			.table("signature_probe")
			.col(ColumnDef::new("value").big_integer())
			.to_owned();
		let (sql, _) = PostgresQueryBuilder.build_create_table(&table);
		backend.execute(&sql, vec![]).await.unwrap();
		let wide = 1_i64 << 40;
		let (insert_sql, _) = Query::insert()
			.into_table("signature_probe")
			.columns(["value"])
			.values_panic([7])
			.build(PostgresQueryBuilder);
		let (select_sql, _) = Query::select()
			.column("value")
			.from("signature_probe")
			.and_where(Expr::col("value").eq(7))
			.build(PostgresQueryBuilder);
		let values = || reinhardt_query::Values(vec![Value::BigInt(Some(wide))]);

		// Act and Assert: every generated path must replace a cached INT4 signature.
		match operation {
			"execute" => {
				sqlx::query(&insert_sql)
					.bind(7_i32)
					.execute(backend.pool())
					.await
					.unwrap();
				let result = backend
					.__execute_generated(&insert_sql, values())
					.await
					.unwrap();
				assert_eq!(result.rows_affected, 1);
			}
			"fetch_one" | "fetch_all" => {
				sqlx::query(&insert_sql)
					.bind(wide)
					.execute(backend.pool())
					.await
					.unwrap();
				let initial = sqlx::query(&select_sql)
					.bind(7_i32)
					.fetch_all(backend.pool())
					.await
					.unwrap();
				assert_eq!(initial.len(), 0);
				let rows = if operation == "fetch_one" {
					vec![
						backend
							.__fetch_one_generated(&select_sql, values())
							.await
							.unwrap(),
					]
				} else {
					backend
						.__fetch_all_generated(&select_sql, values())
						.await
						.unwrap()
				};
				assert_eq!(rows.len(), 1);
				assert_eq!(rows[0].data["value"], QueryValue::Int(wide));
			}
			"transaction" => {
				let mut transaction = backend.begin().await.unwrap();
				let first = transaction
					.__execute_generated(
						&insert_sql,
						reinhardt_query::Values(vec![Value::Int(Some(7))]),
						DatabaseType::Postgres,
					)
					.await
					.unwrap();
				assert_eq!(first.rows_affected, 1);
				let result = transaction
					.__execute_generated(&insert_sql, values(), DatabaseType::Postgres)
					.await
					.unwrap();
				assert_eq!(result.rows_affected, 1);
				transaction.rollback().await.unwrap();
			}
			_ => panic!("unknown generated operation"),
		}
	}

	#[cfg(all(feature = "orm", feature = "associations"))]
	#[rstest]
	#[tokio::test]
	async fn association_manager_generated_operations_preserve_bound_keys(
		#[future] database: (ContainerAsync<Postgres>, PostgresBackend),
	) {
		let (_container, backend) = database.await;
		let db = reinhardt_db::orm::connection::DatabaseConnection::new(
			reinhardt_db::orm::connection::DatabaseBackend::Postgres,
			reinhardt_db::backends::DatabaseConnection::new(std::sync::Arc::new(backend)),
		);
		super::associations::verify_manager_lifecycle(&db).await;
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

#[cfg(all(feature = "orm", feature = "associations", feature = "mysql"))]
mod mysql {
	use reinhardt_db::orm::connection::DatabaseConnection;
	use rstest::{fixture, rstest};
	use testcontainers::{
		ContainerAsync, GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner,
	};

	#[fixture]
	async fn database() -> (ContainerAsync<GenericImage>, DatabaseConnection) {
		let container = GenericImage::new("mysql", "8.0")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(3306))
			// The initialization server uses port 0; wait for the TCP server.
			.with_wait_for(WaitFor::message_on_stderr("port: 3306"))
			.with_env_var("MYSQL_ROOT_PASSWORD", "test")
			.with_env_var("MYSQL_DATABASE", "test")
			.with_startup_timeout(std::time::Duration::from_secs(180))
			.start()
			.await
			.unwrap();
		let url = format!(
			"mysql://root:test@{}:{}/test",
			container.get_host().await.unwrap(),
			container.get_host_port_ipv4(3306).await.unwrap()
		);
		let db = DatabaseConnection::connect_mysql(&url).await.unwrap();
		(container, db)
	}

	#[rstest]
	#[tokio::test]
	async fn association_manager_generated_operations_preserve_bound_keys(
		#[future] database: (ContainerAsync<GenericImage>, DatabaseConnection),
	) {
		let (_container, db) = database.await;
		super::associations::verify_manager_lifecycle(&db).await;
	}
}
