//! Regression coverage for composite primary keys with physical column aliases.

use std::{collections::HashMap, sync::Arc};

use reinhardt_db::orm::{
	Model,
	composite_pk::{CompositePrimaryKey, PkValue},
};
use reinhardt_macros::model;
use reinhardt_query::prelude::{
	Alias, ColumnDef, ColumnType, Expr, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use testcontainers::{ContainerAsync, GenericImage};

#[model(app_label = "composite_column_aliases", table_name = "aliased_entries")]
#[derive(Serialize, Deserialize)]
struct AliasedEntry {
	#[field(primary_key = true, max_length = 64, db_column = "tenant_id")]
	tenant_key: String,
	#[field(primary_key = true, max_length = 64, db_column = "entry_id")]
	entry_key: String,
}

#[model(app_label = "composite_column_aliases", table_name = "mixed_entries")]
#[derive(Serialize, Deserialize)]
struct MixedEntry {
	#[field(primary_key = true, max_length = 64, db_column = "tenant_id")]
	tenant_key: String,
	#[field(primary_key = true, max_length = 64)]
	entry_id: String,
}

#[model(app_label = "composite_column_aliases", table_name = "keyword_entries")]
#[derive(Serialize, Deserialize)]
struct KeywordEntry {
	#[field(primary_key = true, max_length = 64)]
	r#type: String,
	#[field(primary_key = true, max_length = 64, db_column = "entry_id")]
	entry_key: String,
}

#[rstest]
fn raw_identifiers_and_aliases_preserve_key_names() {
	// Arrange
	let key = KeywordEntryCompositePk::new("kind".into(), "entry".into());

	// Act
	let metadata = KeywordEntry::composite_primary_key().unwrap();
	let values = key.to_pk_values().unwrap();

	// Assert
	assert_eq!(metadata.fields(), &["type", "entry_id"]);
	assert_eq!(
		values,
		HashMap::from([
			("type".into(), PkValue::String("kind".into())),
			("entry_id".into(), PkValue::String("entry".into())),
		])
	);
	assert_eq!(key.r#type, "kind");
	assert_eq!(key.to_string(), "(v2;r#type=4:kind, entry_key=5:entry)");
}

#[fixture]
fn aliased_entry() -> AliasedEntry {
	AliasedEntry {
		tenant_key: "t".into(),
		entry_key: "e".into(),
	}
}

#[fixture]
fn mixed_entry() -> MixedEntry {
	MixedEntry {
		tenant_key: "t".into(),
		entry_id: "e".into(),
	}
}

#[rstest]
#[case::all_aliased(AliasedEntry::composite_primary_key().unwrap())]
#[case::mixed(MixedEntry::composite_primary_key().unwrap())]
fn composite_metadata_uses_db_columns(#[case] key: CompositePrimaryKey) {
	assert_eq!(key.fields(), &["tenant_id", "entry_id"]);
	assert_eq!(key.to_sql(), "PRIMARY KEY (\"tenant_id\", \"entry_id\")");
}

#[rstest]
fn composite_values_use_db_columns(aliased_entry: AliasedEntry, mixed_entry: MixedEntry) {
	// Arrange
	let expected = HashMap::from([
		("tenant_id".into(), PkValue::String("t".into())),
		("entry_id".into(), PkValue::String("e".into())),
	]);

	// Act
	let aliased_values = AliasedEntryCompositePk::new("t".into(), "e".into())
		.to_pk_values()
		.unwrap();
	let mixed_values = MixedEntryCompositePk::new("t".into(), "e".into())
		.to_pk_values()
		.unwrap();

	// Assert
	assert_eq!(aliased_values, expected);
	assert_eq!(mixed_values, expected);
	assert_eq!(aliased_entry.get_composite_pk_values().unwrap(), expected);
	assert_eq!(mixed_entry.get_composite_pk_values().unwrap(), expected);
}

#[rstest]
#[case::all_aliased(
	AliasedEntry::composite_primary_key().unwrap(),
	AliasedEntryCompositePk::new("t".into(), "e".into()).to_pk_values().unwrap()
)]
#[case::mixed(
	MixedEntry::composite_primary_key().unwrap(),
	MixedEntryCompositePk::new("t".into(), "e".into()).to_pk_values().unwrap()
)]
fn composite_predicate_uses_db_columns(
	#[case] key: CompositePrimaryKey,
	#[case] values: HashMap<String, PkValue>,
) {
	assert_eq!(
		key.to_where_clause(&values).unwrap(),
		"\"tenant_id\" = 't' AND \"entry_id\" = 'e'"
	);
}

#[rstest]
fn composite_rust_names_are_preserved(mut aliased_entry: AliasedEntry) {
	// Arrange
	let key = AliasedEntryCompositePk {
		tenant_key: "other".into(),
		entry_key: "entry".into(),
	};

	// Act
	aliased_entry.set_primary_key(key.clone());
	let tuple: (String, String) = key.clone().into();
	let serialized = serde_json::to_value(&aliased_entry).unwrap();

	// Assert
	assert_eq!(aliased_entry.primary_key(), Some(key.clone()));
	assert_eq!(key.tenant_key, "other");
	assert_eq!(key.entry_key, "entry");
	assert_eq!(tuple, ("other".into(), "entry".into()));
	assert_eq!(AliasedEntryCompositePk::from(tuple), key);
	assert_eq!(
		key.to_string(),
		"(v2;tenant_key=5:other, entry_key=5:entry)"
	);
	assert_eq!(
		serialized,
		serde_json::json!({"tenant_key": "other", "entry_key": "entry"})
	);
}

#[derive(Debug, reinhardt_query::Iden)]
enum EntryColumn {
	TenantId,
	EntryId,
}

#[fixture]
async fn physical_column_tables(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>) {
	let (container, pool, _port, _url) = postgres_container.await;
	for table in [AliasedEntry::table_name(), MixedEntry::table_name()] {
		let create = Query::create_table()
			.table(Alias::new(table))
			.col(
				ColumnDef::new(EntryColumn::TenantId)
					.column_type(ColumnType::String(Some(64)))
					.not_null(true),
			)
			.col(
				ColumnDef::new(EntryColumn::EntryId)
					.column_type(ColumnType::String(Some(64)))
					.not_null(true),
			)
			.primary_key([EntryColumn::TenantId, EntryColumn::EntryId])
			.to_string(PostgresQueryBuilder);
		sqlx::query(&create).execute(pool.as_ref()).await.unwrap();

		let insert = Query::insert()
			.into_table(Alias::new(table))
			.columns([EntryColumn::TenantId, EntryColumn::EntryId])
			.values_panic(["t", "e"])
			.values_panic(["t", "other"])
			.values_panic(["other", "e"])
			.to_string(PostgresQueryBuilder);
		sqlx::query(&insert).execute(pool.as_ref()).await.unwrap();
	}
	(container, pool)
}

#[rstest]
#[case::all_aliased(
	AliasedEntry::table_name(),
	AliasedEntry::composite_primary_key().unwrap(),
	AliasedEntryCompositePk::new("t".into(), "e".into()).to_pk_values().unwrap()
)]
#[case::mixed(
	MixedEntry::table_name(),
	MixedEntry::composite_primary_key().unwrap(),
	MixedEntryCompositePk::new("t".into(), "e".into()).to_pk_values().unwrap()
)]
#[tokio::test]
async fn composite_predicate_queries_physical_postgres_columns(
	#[future] physical_column_tables: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
	#[case] table: &str,
	#[case] key: CompositePrimaryKey,
	#[case] values: HashMap<String, PkValue>,
) {
	// Arrange
	let (_container, pool) = physical_column_tables.await;
	let predicate = key.to_where_clause(&values).unwrap();
	let select = Query::select()
		.columns([EntryColumn::TenantId, EntryColumn::EntryId])
		.from(Alias::new(table))
		.and_where(Expr::cust(predicate))
		.to_string(PostgresQueryBuilder);

	// Act
	let rows: Vec<(String, String)> = sqlx::query_as(&select)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows, vec![("t".into(), "e".into())]);
}
