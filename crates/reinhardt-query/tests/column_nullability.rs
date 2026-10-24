//! Explicit nullability changes and the legacy MODIFY COLUMN contract.

use reinhardt_query::prelude::*;
use reinhardt_query::types::{ColumnDef, ColumnType};
use rstest::rstest;

#[rstest]
#[case::nullable(false, "DROP")]
#[case::required(true, "SET")]
fn explicit_column_nullability(
	#[case] not_null: bool,
	#[case] action: &str,
	#[values(false, true)] with_type: bool,
	#[values(false, true)] cockroachdb: bool,
) {
	// Arrange
	let mut column = ColumnDef::new("user\"name").not_null(not_null);
	if with_type {
		column = column.column_type(ColumnType::Text);
	}
	let mut stmt = Query::alter_table();
	stmt.table("user\"data").modify_column(column);
	let prefix = "ALTER TABLE \"user\"\"data\" ALTER COLUMN \"user\"\"name\"";
	let expected = if with_type {
		if cockroachdb {
			format!("{prefix} TYPE TEXT; {prefix} {action} NOT NULL")
		} else {
			format!("{prefix} TYPE TEXT, ALTER COLUMN \"user\"\"name\" {action} NOT NULL")
		}
	} else {
		format!("{prefix} {action} NOT NULL")
	};

	// Act
	let (sql, values) = if cockroachdb {
		CockroachDBQueryBuilder::new().build_alter_table(&stmt)
	} else {
		PostgresQueryBuilder::new().build_alter_table(&stmt)
	};

	// Assert
	assert_eq!(sql, expected);
	assert!(values.is_empty());
}

#[rstest]
#[case::nullable(false, "")]
#[case::required(true, " NOT NULL")]
fn mysql_explicit_column_nullability(#[case] not_null: bool, #[case] suffix: &str) {
	// Arrange
	let mut stmt = Query::alter_table();
	stmt.table("users").modify_column(
		ColumnDef::new("name")
			.column_type(ColumnType::Text)
			.not_null(not_null),
	);

	// Act
	let (sql, values) = MySqlQueryBuilder::new().build_alter_table(&stmt);

	// Assert
	assert_eq!(
		sql,
		format!("ALTER TABLE `users` MODIFY COLUMN `name` TEXT{suffix}")
	);
	assert!(values.is_empty());
}

#[rstest]
fn unspecified_modify_column_preserves_nullability() {
	// Arrange
	let mut stmt = Query::alter_table();
	stmt.table("users")
		.modify_column(ColumnDef::new("name").column_type(ColumnType::Text));

	// Act
	let (sql, _) = PostgresQueryBuilder::new().build_alter_table(&stmt);

	// Assert
	assert_eq!(sql, "ALTER TABLE \"users\" ALTER COLUMN \"name\" TYPE TEXT");
}

#[rstest]
#[case::postgres(
	false,
	"ALTER TABLE \"users\" ALTER COLUMN \"age\" TYPE INTEGER, ALTER COLUMN \"age\" DROP NOT NULL, ALTER COLUMN \"age\" SET DEFAULT $1, ADD COLUMN \"name\" TEXT",
	"ALTER TABLE \"users\" ALTER COLUMN \"age\" TYPE INTEGER, ALTER COLUMN \"age\" DROP NOT NULL, ALTER COLUMN \"age\" SET DEFAULT 7, ADD COLUMN \"name\" TEXT"
)]
#[case::cockroachdb(
	true,
	"ALTER TABLE \"users\" ALTER COLUMN \"age\" TYPE INTEGER; ALTER TABLE \"users\" ALTER COLUMN \"age\" DROP NOT NULL; ALTER TABLE \"users\" ALTER COLUMN \"age\" SET DEFAULT $1; ALTER TABLE \"users\" ADD COLUMN \"name\" TEXT",
	"ALTER TABLE \"users\" ALTER COLUMN \"age\" TYPE INTEGER; ALTER TABLE \"users\" ALTER COLUMN \"age\" DROP NOT NULL; ALTER TABLE \"users\" ALTER COLUMN \"age\" SET DEFAULT 7; ALTER TABLE \"users\" ADD COLUMN \"name\" TEXT"
)]
fn nullability_with_default_and_following_operation(
	#[case] cockroachdb: bool,
	#[case] expected_bound: &str,
	#[case] expected_inline: &str,
) {
	// Arrange
	let mut stmt = Query::alter_table();
	stmt.table("users")
		.modify_column(
			ColumnDef::new("age")
				.column_type(ColumnType::Integer)
				.not_null(false)
				.default(7.into()),
		)
		.add_column(ColumnDef::new("name").column_type(ColumnType::Text));

	// Act
	let (sql, values, inline) = if cockroachdb {
		let builder = CockroachDBQueryBuilder::new();
		let (sql, values) = builder.build_alter_table(&stmt);
		let inline = reinhardt_query::query::traits::inline_params(&sql, &values);
		(sql, values, inline)
	} else {
		let builder = PostgresQueryBuilder::new();
		let (sql, values) = builder.build_alter_table(&stmt);
		(sql, values, stmt.to_string(builder))
	};

	// Assert
	assert_eq!(sql, expected_bound);
	assert_eq!(values.0, vec![7.into()]);
	assert_eq!(inline, expected_inline);
}

#[rstest]
fn cockroachdb_keeps_type_free_operations_in_one_statement() {
	// Arrange
	let mut stmt = Query::alter_table();
	stmt.table("users")
		.add_column(ColumnDef::new("age").column_type(ColumnType::Integer))
		.add_column(ColumnDef::new("name").column_type(ColumnType::Text));

	// Act
	let (sql, _) = CockroachDBQueryBuilder::new().build_alter_table(&stmt);

	// Assert
	assert_eq!(
		sql,
		"ALTER TABLE \"users\" ADD COLUMN \"age\" INTEGER, ADD COLUMN \"name\" TEXT"
	);
}

#[rstest]
#[should_panic(expected = "SQLite does not support MODIFY COLUMN - table recreation required")]
fn sqlite_explicit_column_nullability_requires_recreation() {
	// Arrange
	let mut stmt = Query::alter_table();
	stmt.table("users")
		.modify_column(ColumnDef::new("name").column_type(ColumnType::Text));

	// Act
	SqliteQueryBuilder::new().build_alter_table(&stmt);
}
