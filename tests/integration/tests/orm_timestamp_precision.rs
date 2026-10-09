//! Native manager CRUD accepts framework clocks and truncates explicit timestamps.

#![cfg(feature = "postgres")]

use chrono::{DateTime, Timelike, Utc};
use reinhardt::model;
use reinhardt_db::backends::DatabaseConnection as BackendConnection;
use reinhardt_db::orm::{DatabaseConnectionLease, Manager, Model};
use reinhardt_query::{ColumnDef, PostgresQueryBuilder, Query, QueryStatementBuilder};
use reinhardt_test::fixtures::postgres_container;
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[model(app_label = "timestamp_precision", table_name = "timestamp_precision")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimestampProbe {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 64)]
	label: String,
	#[field(auto_now_add = true)]
	created_at: DateTime<Utc>,
	#[field(auto_now = true)]
	updated_at: DateTime<Utc>,
}

#[rstest]
#[tokio::test]
async fn manager_crud_normalizes_auto_and_explicit_nanosecond_timestamps(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	// Arrange: retain the container and registered connection for the whole test.
	let (_container, _pool, _port, url) = postgres_container.await;
	let owner = BackendConnection::connect(&url).await.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let mut connection = lease.handle();
	let mut schema = Query::create_table();
	schema
		.table(TimestampProbe::table_name())
		.col(ColumnDef::new("id").big_integer().primary_key(true))
		.col(ColumnDef::new("label").string_len(64).not_null(true))
		.col(
			ColumnDef::new("created_at")
				.timestamp_with_time_zone()
				.not_null(true),
		)
		.col(
			ColumnDef::new("updated_at")
				.timestamp_with_time_zone()
				.not_null(true),
		);
	connection
		.execute(&schema.to_string(PostgresQueryBuilder), vec![])
		.await
		.unwrap();
	let manager = Manager::<TimestampProbe>::new();
	let clock = TimestampProbe::build()
		.id(1)
		.label("framework clock")
		.finish();

	// Act: auto_now_add and auto_now use the framework producer unmodified.
	let created = manager
		.create_with_conn(&mut connection, &clock)
		.await
		.unwrap();

	// Assert: both persisted clocks agree with truncation, not server-side rounding.
	assert_eq!(
		created.created_at,
		clock
			.created_at
			.with_nanosecond(clock.created_at.nanosecond() / 1_000 * 1_000)
			.unwrap()
	);
	assert_eq!(
		created.updated_at,
		clock
			.updated_at
			.with_nanosecond(clock.updated_at.nanosecond() / 1_000 * 1_000)
			.unwrap()
	);

	// Arrange: 789 nanoseconds would round upward if sent as PostgreSQL text.
	let nanos = DateTime::<Utc>::from_timestamp(1_700_000_000, 123_456_789).unwrap();
	let micros = DateTime::<Utc>::from_timestamp(1_700_000_000, 123_456_000).unwrap();
	let explicit = TimestampProbe {
		id: 2,
		label: "explicit clock".to_owned(),
		created_at: nanos,
		updated_at: nanos,
	};

	// Act: deterministic caller timestamps exercise create and update bindings.
	let inserted = manager
		.create_with_conn(&mut connection, &explicit)
		.await
		.unwrap();
	let changed = TimestampProbe {
		updated_at: nanos + chrono::Duration::seconds(1),
		label: "updated clock".to_owned(),
		..explicit
	};
	let updated = manager
		.update_with_conn(&mut connection, &changed)
		.await
		.unwrap();

	// Assert: the returned database rows prove the microsecond storage contract.
	assert_eq!(inserted.created_at, micros);
	assert_eq!(inserted.updated_at, micros);
	assert_eq!(updated.created_at, micros);
	assert_eq!(updated.updated_at, micros + chrono::Duration::seconds(1));
	assert_eq!(updated.label, "updated clock");
}
