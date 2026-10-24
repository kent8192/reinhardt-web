//! Regression coverage for typed MySQL index column prefixes.

use reinhardt_query::prelude::*;
use rstest::rstest;
use std::num::NonZeroU32;

#[rstest]
#[case(1)]
#[case(64)]
#[case(191)]
fn mysql_index_prefix_keeps_identifiers_and_other_columns(#[case] length: u32) {
	// Arrange
	let mut statement = Query::create_index();
	statement
		.name("idx`user")
		.table("audit_events")
		.col_prefix("user`name", NonZeroU32::new(length).unwrap())
		.col_order("timestamp", Order::Desc)
		.unique()
		.if_not_exists();
	let expected = format!(
		"CREATE UNIQUE INDEX `idx``user` ON `audit_events` (`user``name`({length}), `timestamp` DESC)"
	);

	// Act
	let (sql, values) = statement.build_any(&MySqlQueryBuilder);

	// Assert
	assert_eq!(sql, expected);
	assert_eq!(values.len(), 0);
	assert_eq!(statement.to_string(MySqlQueryBuilder), expected);
	assert_eq!(
		statement.clone().take().to_string(MySqlQueryBuilder),
		expected
	);
}

#[rstest]
#[case(false)]
#[case(true)]
#[should_panic(expected = "PostgreSQL does not support index column prefixes")]
fn postgres_rejects_index_prefix(#[case] inline: bool) {
	// Arrange
	let statement = Query::create_index()
		.name("idx_user")
		.table("audit_events")
		.col_prefix("user", NonZeroU32::new(191).unwrap())
		.to_owned();

	// Act / Assert: neither rendering path may silently discard the prefix.
	if inline {
		statement.to_string(PostgresQueryBuilder);
	} else {
		statement.build_any(&PostgresQueryBuilder);
	}
}

#[rstest]
#[case(false)]
#[case(true)]
#[should_panic(expected = "SQLite does not support index column prefixes")]
fn sqlite_rejects_index_prefix(#[case] inline: bool) {
	// Arrange
	let statement = Query::create_index()
		.name("idx_user")
		.table("audit_events")
		.col_prefix("user", NonZeroU32::new(191).unwrap())
		.to_owned();

	// Act / Assert
	if inline {
		statement.to_string(SqliteQueryBuilder);
	} else {
		statement.build_any(&SqliteQueryBuilder);
	}
}
