//! Native Session range regressions for backend quoting and inclusive endpoints.
#![cfg(all(feature = "orm", feature = "mysql", feature = "sqlite"))]
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use std::{collections::HashMap, sync::Arc, time::Duration};

use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::query::Filter;
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::Session;
use reinhardt_db::orm::{FieldSelector, FilterOperator, FilterValue, Manager, Model, QuerySet};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Row, any::AnyPoolOptions};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[derive(Clone)]
struct Fields;

impl FieldSelector for Fields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RangeRow {
	id: i64,
}

impl Model for RangeRow {
	type PrimaryKey = i64;
	type Fields = Fields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"range_rows"
	}

	fn new_fields() -> Self::Fields {
		Fields
	}

	fn primary_key(&self) -> Option<i64> {
		Some(self.id)
	}

	fn set_primary_key(&mut self, id: i64) {
		self.id = id;
	}

	fn field_metadata() -> Vec<FieldInfo> {
		vec![FieldInfo {
			name: "id".into(),
			field_type: "reinhardt.orm.models.BigIntegerField".into(),
			nullable: false,
			primary_key: true,
			unique: true,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: Some("quoted\"id`".into()),
			choices: None,
			attributes: HashMap::new(),
			domain: None,
			storage_kind: None,
		}]
	}
}

struct RangeDatabase {
	pool: Arc<AnyPool>,
	backend: DbBackend,
	_container: Option<ContainerAsync<Mysql>>,
}

#[fixture]
async fn range_database(
	#[default(DbBackend::Sqlite)] backend: DbBackend,
	#[default(false)] no_backslash_escapes: bool,
) -> RangeDatabase {
	sqlx::any::install_default_drivers();
	let (url, container, quoted_column) = match backend {
		DbBackend::Mysql => {
			let image = Mysql::default()
				.with_tag("8.0")
				.with_startup_timeout(Duration::from_secs(180));
			let container = if no_backslash_escapes {
				image.with_cmd(["--sql-mode=NO_BACKSLASH_ESCAPES"])
			} else {
				image
			}
			.start()
			.await
			.unwrap();
			let url = format!(
				"mysql://root@{}:{}/test",
				container.get_host().await.unwrap(),
				container.get_host_port_ipv4(3306).await.unwrap(),
			);
			(url, Some(container), "`quoted\"id```")
		}
		DbBackend::Sqlite => ("sqlite::memory:".into(), None, "\"quoted\"\"id`\""),
		DbBackend::Postgres => panic!("this fixture covers MySQL and SQLite"),
	};
	let database = RangeDatabase {
		pool: Arc::new(
			AnyPoolOptions::new()
				.max_connections(1)
				.connect(&url)
				.await
				.unwrap(),
		),
		backend,
		_container: container,
	};
	sqlx::query(&format!(
		"CREATE TABLE range_rows ({quoted_column} BIGINT PRIMARY KEY)"
	))
	.execute(database.pool.as_ref())
	.await
	.unwrap();
	for id in [1_i64, 9_007_199_254_740_993] {
		sqlx::query(&format!(
			"INSERT INTO range_rows ({quoted_column}) VALUES (?)"
		))
		.bind(id)
		.execute(database.pool.as_ref())
		.await
		.unwrap();
	}
	database
}

#[rstest]
#[case::mysql(DbBackend::Mysql, false)]
#[case::mysql_no_backslash_escapes(DbBackend::Mysql, true)]
#[case::sqlite(DbBackend::Sqlite, false)]
#[tokio::test]
async fn session_range_includes_each_boundary_and_preserves_wide_integers(
	#[case] backend: DbBackend,
	#[case] no_backslash_escapes: bool,
	#[with(backend, no_backslash_escapes)]
	#[future]
	range_database: RangeDatabase,
) {
	// Arrange
	let database = range_database.await;
	let session = Session::new(database.pool.clone(), database.backend)
		.await
		.unwrap();
	if backend == DbBackend::Mysql {
		let mode: String = sqlx::query("SELECT @@SESSION.sql_mode")
			.fetch_one(database.pool.as_ref())
			.await
			.unwrap()
			.get(0);
		assert!(!mode.split(',').any(|mode| mode == "ANSI_QUOTES"));
		assert_eq!(
			mode.split(',').any(|mode| mode == "NO_BACKSLASH_ESCAPES"),
			no_backslash_escapes,
		);
	}

	for id in [1_i64, 9_007_199_254_740_993] {
		for (lower, upper, included) in [
			(id, id, true),
			(id - 1, id, true),
			(id, id + 1, true),
			(id - 2, id - 1, false),
			(id + 1, id + 2, false),
		] {
			let queryset = QuerySet::<RangeRow>::new().filter(Filter::new(
				"quoted\"id`",
				FilterOperator::Range,
				FilterValue::Range(
					Box::new(FilterValue::Integer(lower)),
					Box::new(FilterValue::Int(upper)),
				),
			));

			// Act
			let rows = session.list(&queryset).await.unwrap();

			// Assert
			let expected = if included {
				vec![RangeRow { id }]
			} else {
				vec![]
			};
			assert_eq!(rows, expected, "range: {lower}..={upper}");
		}
	}
}
