//! Settings and audit index initialization across SQL database backends.
#![cfg(all(native, feature = "dynamic-database"))]

use reinhardt_conf::settings::audit::backends::DatabaseAuditBackend;
use reinhardt_conf::settings::audit::{
	AuditBackend, AuditEvent, ChangeRecord, EventFilter, EventType,
};
use reinhardt_conf::settings::backends::DatabaseBackend;
use reinhardt_query::prelude::*;
#[cfg(feature = "mysql")]
use reinhardt_test::fixtures::mysql_container;
#[cfg(feature = "postgres")]
use reinhardt_test::fixtures::postgres_container;
use reinhardt_test::fixtures::temp_dir;
use rstest::{fixture, rstest};
use serde_json::json;
#[cfg(feature = "mysql")]
use sqlx::Row;
use sqlx::{AnyPool, any::AnyPoolOptions};
use std::{collections::HashMap, sync::Arc};
use tempfile::TempDir;
use testcontainers::{ContainerAsync, GenericImage};

#[derive(Debug, Iden)]
enum Settings {
	Table,
	Key,
	Value,
	ExpireDate,
}

#[derive(Debug, Iden)]
enum AuditEvents {
	Table,
	Id,
	Timestamp,
	EventType,
	User,
	Changes,
}

#[derive(Clone, Copy)]
enum Dialect {
	#[cfg(feature = "mysql")]
	Mysql,
	#[cfg(feature = "postgres")]
	Postgres,
	Sqlite,
}

struct StoreFixture {
	pool: Arc<AnyPool>,
	_container: Option<ContainerAsync<GenericImage>>,
	_directory: Option<TempDir>,
	url: String,
	dialect: Dialect,
}

impl StoreFixture {
	async fn connect(url: String, dialect: Dialect) -> Self {
		sqlx::any::install_default_drivers();
		let pool = AnyPoolOptions::new()
			.max_connections(2)
			.connect(&url)
			.await
			.unwrap();
		Self {
			_container: None,
			_directory: None,
			pool: Arc::new(pool),
			url,
			dialect,
		}
	}

	fn render(&self, statement: &impl QueryStatementBuilder) -> String {
		match self.dialect {
			#[cfg(feature = "mysql")]
			Dialect::Mysql => statement.to_string(MySqlQueryBuilder),
			#[cfg(feature = "postgres")]
			Dialect::Postgres => statement.to_string(PostgresQueryBuilder),
			Dialect::Sqlite => statement.to_string(SqliteQueryBuilder),
		}
	}

	async fn create_legacy_tables(&self) {
		let settings = Query::create_table()
			.table(Settings::Table.into_iden())
			.col(
				ColumnDef::new(Settings::Key)
					.string_len(255)
					.not_null(true)
					.primary_key(true),
			)
			.col(ColumnDef::new(Settings::Value).text().not_null(true))
			.col(ColumnDef::new(Settings::ExpireDate).text())
			.to_owned();
		sqlx::query(&self.render(&settings))
			.execute(self.pool.as_ref())
			.await
			.unwrap();

		let audit = Query::create_table()
			.table(AuditEvents::Table.into_iden())
			.col(
				ColumnDef::new(AuditEvents::Id)
					.integer()
					.not_null(true)
					.auto_increment(true)
					.primary_key(true),
			)
			.col(ColumnDef::new(AuditEvents::Timestamp).text().not_null(true))
			.col(ColumnDef::new(AuditEvents::EventType).text().not_null(true))
			.col(ColumnDef::new(AuditEvents::User).text())
			.col(ColumnDef::new(AuditEvents::Changes).text().not_null(true))
			.to_owned();
		sqlx::query(&self.render(&audit))
			.execute(self.pool.as_ref())
			.await
			.unwrap();

		let setting = Query::insert()
			.into_table(Settings::Table.into_iden())
			.columns([Settings::Key, Settings::Value, Settings::ExpireDate])
			.values(vec![
				"legacy".into_value(),
				json!({"preserved": true}).to_string().into_value(),
				"2999-01-01T00:00:00+00:00".into_value(),
			])
			.unwrap()
			.to_owned();
		sqlx::query(&self.render(&setting))
			.execute(self.pool.as_ref())
			.await
			.unwrap();
		let event = Query::insert()
			.into_table(AuditEvents::Table.into_iden())
			.columns([
				AuditEvents::Timestamp,
				AuditEvents::EventType,
				AuditEvents::User,
				AuditEvents::Changes,
			])
			.values(vec![
				"2020-01-01T00:00:00+00:00".into_value(),
				EventType::ConfigUpdate.as_str().into_value(),
				"legacy".into_value(),
				"{}".into_value(),
			])
			.unwrap()
			.to_owned();
		sqlx::query(&self.render(&event))
			.execute(self.pool.as_ref())
			.await
			.unwrap();
	}
}

#[cfg(feature = "mysql")]
#[fixture]
async fn mysql_store(
	#[future] mysql_container: (
		ContainerAsync<GenericImage>,
		Arc<sqlx::MySqlPool>,
		u16,
		String,
	),
) -> StoreFixture {
	let (container, _pool, _port, url) = mysql_container.await;
	let mut store = StoreFixture::connect(url, Dialect::Mysql).await;
	store._container = Some(container);
	store
}

#[cfg(feature = "postgres")]
#[fixture]
async fn postgres_store(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> StoreFixture {
	let (container, _pool, _port, url) = postgres_container.await;
	let mut store = StoreFixture::connect(url, Dialect::Postgres).await;
	store._container = Some(container);
	store
}

#[fixture]
async fn sqlite_store(temp_dir: TempDir) -> StoreFixture {
	let url = format!(
		"sqlite://{}?mode=rwc",
		temp_dir.path().join("settings.db").display()
	);
	let mut store = StoreFixture::connect(url, Dialect::Sqlite).await;
	store._directory = Some(temp_dir);
	store
}

async fn exercise_initialization(store: &StoreFixture, legacy: bool) {
	// Arrange
	if legacy {
		store.create_legacy_tables().await;
	}
	let settings = DatabaseBackend::from_pool(store.pool.clone(), &store.url);

	// Act: initialize missing indexes, then initialize the same tables again.
	assert_eq!(settings.create_table().await, Ok(()));
	assert_eq!(settings.create_table().await, Ok(()));
	let audit = DatabaseAuditBackend::new(&store.url).await.unwrap();
	let reopened_audit = DatabaseAuditBackend::new(&store.url).await.unwrap();

	// Assert: legacy rows remain readable after both initialization attempts.
	assert_eq!(
		settings.get("legacy").await.unwrap(),
		legacy.then(|| json!({"preserved": true}))
	);
	let legacy_events = reopened_audit.get_events(None).await.unwrap();
	assert_eq!(legacy_events.len(), usize::from(legacy));
	if legacy {
		assert_eq!(legacy_events[0].user.as_deref(), Some("legacy"));
		assert_eq!(legacy_events[0].event_type, EventType::ConfigUpdate);
		assert_eq!(legacy_events[0].changes.len(), 0);
	}

	// CREATE / READ: TTL values and permanent values retain their lookup semantics.
	let value = json!({"value": "preserved"});
	settings.set("active", &value, Some(3600)).await.unwrap();
	settings.set("permanent", &value, None).await.unwrap();
	settings.set("expired", &value, Some(0)).await.unwrap();
	assert_eq!(settings.get("active").await.unwrap(), Some(value.clone()));
	assert_eq!(
		settings.get("permanent").await.unwrap(),
		Some(value.clone())
	);
	assert_eq!(settings.get("expired").await.unwrap(), None);
	assert!(!settings.exists("expired").await.unwrap());
	let mut keys = settings.keys().await.unwrap();
	keys.sort();
	let expected = if legacy {
		vec!["active", "legacy", "permanent"]
	} else {
		vec!["active", "permanent"]
	};
	assert_eq!(keys, expected);

	// DELETE: expired reads remove their row; bulk cleanup handles unread expiry.
	assert_eq!(settings.cleanup_expired().await.unwrap(), 0);
	settings
		.set("cleanup_expired", &value, Some(0))
		.await
		.unwrap();
	assert_eq!(settings.cleanup_expired().await.unwrap(), 1);
	assert!(settings.exists("active").await.unwrap());
	assert!(settings.exists("permanent").await.unwrap());
	assert_eq!(
		settings.get("legacy").await.unwrap(),
		legacy.then(|| json!({"preserved": true}))
	);

	// CREATE / READ: long users sharing the entire index prefix remain distinct.
	for suffix in ["first", "second"] {
		let user = format!("{}{suffix}", "名".repeat(250));
		let event = AuditEvent::new(
			EventType::ConfigUpdate,
			Some(user.clone()),
			HashMap::from([(
				"setting".to_owned(),
				ChangeRecord {
					old_value: None,
					new_value: Some(json!(suffix)),
				},
			)]),
		);
		let timestamp = event.timestamp;
		audit.log_event(event).await.unwrap();
		let events = reopened_audit
			.get_events(Some(EventFilter {
				event_type: Some(EventType::ConfigUpdate),
				user: Some(user.clone()),
				start_time: Some(timestamp - chrono::Duration::seconds(1)),
				end_time: Some(timestamp + chrono::Duration::seconds(1)),
			}))
			.await
			.unwrap();
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].timestamp, timestamp);
		assert_eq!(events[0].user.as_deref(), Some(user.as_str()));
		assert_eq!(events[0].changes["setting"].new_value, Some(json!(suffix)));
	}
	assert_eq!(
		reopened_audit.get_events(None).await.unwrap().len(),
		2 + usize::from(legacy)
	);
}

#[cfg(feature = "mysql")]
#[rstest]
#[case::fresh(false)]
#[case::legacy(true)]
#[tokio::test]
async fn mysql_initialization_creates_all_text_indexes(
	#[future] mysql_store: StoreFixture,
	#[case] legacy: bool,
) {
	let store = mysql_store.await;
	exercise_initialization(&store, legacy).await;

	// Assert: MySQL actually installed every index with the intended prefix length.
	let query = Query::select()
		.columns([
			Alias::new("INDEX_NAME"),
			Alias::new("COLUMN_NAME"),
			Alias::new("SUB_PART"),
		])
		.from((Alias::new("information_schema"), Alias::new("statistics")))
		.and_where(Expr::col(Alias::new("TABLE_SCHEMA")).eq("test_db"))
		.and_where(Expr::col(Alias::new("TABLE_NAME")).is_in(["settings", "audit_events"]))
		.and_where(Expr::col(Alias::new("INDEX_NAME")).ne("PRIMARY"))
		.order_by(Alias::new("INDEX_NAME"), Order::Asc)
		.to_owned();
	let rows = sqlx::query(&store.render(&query))
		.fetch_all(store.pool.as_ref())
		.await
		.unwrap();
	let indexes: Vec<(String, String, i64)> = rows
		.iter()
		.map(|row| (row.get(0), row.get(1), row.get(2)))
		.collect();
	assert_eq!(
		indexes,
		vec![
			(
				"idx_events_timestamp".to_owned(),
				"timestamp".to_owned(),
				64
			),
			("idx_events_type".to_owned(), "event_type".to_owned(), 32),
			("idx_events_user".to_owned(), "user".to_owned(), 191),
			(
				"idx_settings_expire_date".to_owned(),
				"expire_date".to_owned(),
				64
			),
		]
	);
}

#[cfg(feature = "postgres")]
#[rstest]
#[case::fresh(false)]
#[case::legacy(true)]
#[tokio::test]
async fn postgres_initialization_preserves_settings_and_audit(
	#[future] postgres_store: StoreFixture,
	#[case] legacy: bool,
) {
	let store = postgres_store.await;
	exercise_initialization(&store, legacy).await;
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_settings_index_initialization_accepts_concurrent_creators(
	#[future] mysql_store: StoreFixture,
) {
	// Arrange: both initializers see the existing table without an expiry index.
	let store = mysql_store.await;
	store.create_legacy_tables().await;
	let first = DatabaseBackend::from_pool(store.pool.clone(), &store.url);
	let second = first.clone();

	// Act
	let (first_result, second_result) = tokio::join!(first.create_table(), second.create_table());

	// Assert
	assert_eq!(first_result, Ok(()));
	assert_eq!(second_result, Ok(()));
	assert_eq!(
		first.get("legacy").await.unwrap(),
		Some(json!({"preserved": true}))
	);
}

#[rstest]
#[case::fresh(false)]
#[case::legacy(true)]
#[tokio::test]
async fn sqlite_initialization_preserves_settings_and_audit(
	#[future] sqlite_store: StoreFixture,
	#[case] legacy: bool,
) {
	let store = sqlite_store.await;
	exercise_initialization(&store, legacy).await;
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_audit_initialization_reports_index_errors(#[future] mysql_store: StoreFixture) {
	// Arrange: an incompatible existing table lacks the indexed timestamp column.
	let store = mysql_store.await;
	let table = Query::create_table()
		.table(AuditEvents::Table.into_iden())
		.col(ColumnDef::new(AuditEvents::Id).integer().primary_key(true))
		.to_owned();
	sqlx::query(&store.render(&table))
		.execute(store.pool.as_ref())
		.await
		.unwrap();

	// Act
	let error = DatabaseAuditBackend::new(&store.url).await.err().unwrap();

	// Assert: non-duplicate errors cannot be discarded as successful initialization.
	assert_eq!(
		error.split(": ").next(),
		Some("Failed to create audit index idx_events_timestamp")
	);
}
