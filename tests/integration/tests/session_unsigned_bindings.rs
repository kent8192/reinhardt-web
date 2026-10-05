//! Public Session queries preserve supported unsigned filter values through
//! query construction, prepared arguments, SQLite execution, and model hydration.
//!
//! On main, `FilterValue::from(u64)` converts supported values to signed integers.
//! Direct `BigUnsigned` conversion and overflow rejection therefore remain binder
//! component tests; the public filter API cannot exercise that representation.

use reinhardt_db::orm::{
	Filter, FilterOperator, FilterValue, Manager, Model,
	fields::{BigIntegerField, Field},
	inspection::FieldInfo,
	model::FieldSelector,
	query::QuerySet,
	query_types::DbBackend,
	session::Session,
};
use reinhardt_query::{
	ColumnDef, Iden, IntoIden, Query, QueryStatementBuilder, SqliteQueryBuilder,
};
use reinhardt_test::fixtures::{loader::temp_dir, testcontainers::create_test_any_pool};
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::AnyPool;
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Debug, Iden)]
enum SessionUnsignedRecord {
	Table,
	Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct UnsignedRecord {
	id: i64,
}

#[derive(Clone)]
struct UnsignedRecordFields;

impl FieldSelector for UnsignedRecordFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for UnsignedRecord {
	type PrimaryKey = i64;
	type Fields = UnsignedRecordFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"session_unsigned_record"
	}

	fn new_fields() -> Self::Fields {
		UnsignedRecordFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		Some(self.id)
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = value;
	}

	fn field_metadata() -> Vec<FieldInfo> {
		let mut id = BigIntegerField::new();
		id.base.primary_key = true;
		id.set_attributes_from_name("id");
		vec![FieldInfo::from_field(&id)]
	}
}

struct SessionFixture {
	pool: Arc<AnyPool>,
	_directory: TempDir,
}

#[fixture]
async fn session_pool(temp_dir: TempDir) -> SessionFixture {
	sqlx::any::install_default_drivers();
	let database_url = format!(
		"sqlite://{}?mode=rwc",
		temp_dir.path().join("session.sqlite").display()
	);
	let pool = Arc::new(create_test_any_pool(&database_url).await.unwrap());
	let schema = Query::create_table()
		.table(SessionUnsignedRecord::Table.into_iden())
		.col(
			ColumnDef::new(SessionUnsignedRecord::Id)
				.big_integer()
				.primary_key(true),
		)
		.to_string(SqliteQueryBuilder);
	sqlx::query(&schema).execute(pool.as_ref()).await.unwrap();
	SessionFixture {
		pool,
		_directory: temp_dir,
	}
}

#[rstest]
#[case::zero(0, 0)]
#[case::ordinary(42, 42)]
#[case::above_i32_max((i32::MAX as u64) + 1, i64::from(i32::MAX) + 1)]
#[case::at_i64_max(i64::MAX as u64, i64::MAX)]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn session_list_preserves_supported_unsigned_filters(
	#[future] session_pool: SessionFixture,
	#[case] value: u64,
	#[case] expected: i64,
) {
	// Arrange: seed the independent signed oracle through reinhardt-query.
	let fixture = session_pool.await;
	let pool = &fixture.pool;
	let insert = Query::insert()
		.into_table(SessionUnsignedRecord::Table.into_iden())
		.columns([SessionUnsignedRecord::Id])
		.values_panic([expected])
		.to_string(SqliteQueryBuilder);
	sqlx::query(&insert).execute(pool.as_ref()).await.unwrap();
	let session = Session::new(pool.clone(), DbBackend::Sqlite).await.unwrap();
	let queryset = QuerySet::<UnsignedRecord>::new().filter(Filter::new(
		"id",
		FilterOperator::Eq,
		FilterValue::from(value),
	));

	// Act
	let records = session.list(&queryset).await.unwrap();

	// Assert: compare complete model rows, including values above i32::MAX.
	assert_eq!(records, vec![UnsignedRecord { id: expected }]);
}
