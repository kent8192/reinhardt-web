//! Regression coverage for pooled PostgreSQL recorder locks (issue #6405).

#![cfg(feature = "postgres")]

use std::sync::Arc;
use std::time::Duration;

use reinhardt_db::backends::{DatabaseConnection, DatabaseError};
use reinhardt_db::migrations::{MigrationError, recorder::DatabaseMigrationRecorder};
use reinhardt_query::prelude::{
	Alias, ColumnDef, Expr, ExprTrait, Func, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::rstest;
use sqlx::{Connection, PgPool};
use testcontainers::{ContainerAsync, GenericImage};
use tokio::time::{sleep, timeout};

type PostgresFixture = (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String);
const DEADLINE: Duration = Duration::from_secs(5);

async fn warmed_connection(url: &str, pool_size: u32) -> DatabaseConnection {
	let connection = DatabaseConnection::connect_postgres_with_pool_size(url, Some(pool_size))
		.await
		.expect("connect recorder pool");
	let pool = connection.into_postgres().expect("PostgreSQL pool");
	let mut sessions = Vec::new();
	for _ in 0..pool_size {
		sessions.push(pool.acquire().await.expect("warm distinct pooled session"));
	}
	assert_eq!(pool.size(), pool_size);
	drop(sessions);
	timeout(DEADLINE, async {
		while pool.num_idle() != pool_size as usize {
			sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("warmed sessions returned to pool");
	connection
}

async fn lock_count(pool: &PgPool, lock_type: &str, granted: bool) -> i64 {
	let sql = Query::select()
		.expr(Func::count(Expr::asterisk().into_simple_expr()))
		.from(Alias::new("pg_locks"))
		.and_where(Expr::col(Alias::new("locktype")).eq(lock_type))
		.and_where(Expr::col(Alias::new("granted")).eq(granted))
		.to_string(PostgresQueryBuilder);
	sqlx::query_scalar(&sql)
		.fetch_one(pool)
		.await
		.expect("inspect PostgreSQL locks")
}

async fn ensure_schema(recorder: &DatabaseMigrationRecorder) {
	timeout(DEADLINE, recorder.ensure_schema_table())
		.await
		.expect("recorder must not block on a leaked lock or exhaust its pool")
		.expect("initialize recorder schema");
}

#[rstest]
#[case::single_connection(1)]
#[case::multiple_connections(2)]
#[tokio::test]
async fn successful_initialization_releases_lock(
	#[future] postgres_container: PostgresFixture,
	#[case] pool_size: u32,
) {
	// Arrange
	let (_container, observer, _port, url) = postgres_container.await;
	let recorder = DatabaseMigrationRecorder::new(warmed_connection(&url, pool_size).await);
	let independent = DatabaseMigrationRecorder::new(warmed_connection(&url, 1).await);

	// Act: repeat initialization to cover both creation and the existing schema.
	for _ in 0..3 {
		ensure_schema(&recorder).await;

		// Assert
		assert_eq!(lock_count(&observer, "advisory", true).await, 0);
		ensure_schema(&independent).await;
		assert_eq!(lock_count(&observer, "advisory", true).await, 0);
	}

	// The committed table and unique index must support normal recorder writes.
	for _ in 0..2 {
		independent
			.record_applied("lock_test", "0001_initial")
			.await
			.expect("record migration using the committed unique index");
	}
	let records = recorder
		.get_applied_for_app("lock_test")
		.await
		.expect("read committed migration record");
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].app, "lock_test");
	assert_eq!(records[0].name, "0001_initial");
}

#[rstest]
#[case::single_connection(1)]
#[case::multiple_connections(2)]
#[tokio::test]
async fn failed_schema_creation_releases_lock(
	#[future] postgres_container: PostgresFixture,
	#[case] pool_size: u32,
) {
	// Arrange: the missing app column makes unique-index creation fail.
	let (_container, observer, _port, url) = postgres_container.await;
	let recorder = DatabaseMigrationRecorder::new(warmed_connection(&url, pool_size).await);
	let malformed_schema = Query::create_table()
		.table(Alias::new("reinhardt_migrations"))
		.col(ColumnDef::new("id").integer())
		.to_string(PostgresQueryBuilder);
	sqlx::query(&malformed_schema)
		.execute(observer.as_ref())
		.await
		.expect("create malformed recorder table");

	// Act
	let error = timeout(DEADLINE, recorder.ensure_schema_table())
		.await
		.expect("failing initialization must complete")
		.expect_err("missing column must reject index creation");

	// Assert
	let MigrationError::DatabaseError(DatabaseError::QueryError(message)) = error else {
		panic!("expected a database query error, got {error:?}");
	};
	assert_eq!(message, "column \"app\" does not exist");
	assert_eq!(lock_count(&observer, "advisory", true).await, 0);

	let drop_table = Query::drop_table()
		.table(Alias::new("reinhardt_migrations"))
		.to_string(PostgresQueryBuilder);
	sqlx::query(&drop_table)
		.execute(observer.as_ref())
		.await
		.expect("remove malformed schema");
	let independent = DatabaseMigrationRecorder::new(warmed_connection(&url, 1).await);
	ensure_schema(&independent).await;
	ensure_schema(&recorder).await;
	assert_eq!(lock_count(&observer, "advisory", true).await, 0);
}

#[rstest]
#[tokio::test]
async fn concurrent_recorders_share_the_schema_lock(#[future] postgres_container: PostgresFixture) {
	// Arrange
	let (_container, observer, _port, url) = postgres_container.await;
	let shared = warmed_connection(&url, 2).await;
	let first = DatabaseMigrationRecorder::new(shared.clone());
	let second = DatabaseMigrationRecorder::new(shared);
	let independent = DatabaseMigrationRecorder::new(warmed_connection(&url, 1).await);

	// Act
	tokio::join!(
		ensure_schema(&first),
		ensure_schema(&second),
		ensure_schema(&independent),
	);

	// Assert
	assert_eq!(lock_count(&observer, "advisory", true).await, 0);
	ensure_schema(&independent).await;
}

#[rstest]
#[tokio::test]
async fn recorder_waits_for_existing_session_lock(#[future] postgres_container: PostgresFixture) {
	// Arrange: a detached session closes on drop, including assertion failures,
	// so the legacy session-scoped lock cannot leak back into a pool.
	let (_container, observer, _port, url) = postgres_container.await;
	let recorder = DatabaseMigrationRecorder::new(warmed_connection(&url, 1).await);
	let mut legacy_session = observer.acquire().await.expect("legacy session").detach();
	let lock_sql = Query::select()
		.expr(Expr::cust(
			"pg_advisory_lock(hashtext('reinhardt_migrations'))",
		))
		.to_string(PostgresQueryBuilder);
	sqlx::query(&lock_sql)
		.execute(&mut legacy_session)
		.await
		.expect("acquire the existing migration lock key");

	// Act
	let initialization = recorder.ensure_schema_table();
	tokio::pin!(initialization);
	tokio::select! {
		result = &mut initialization => {
			panic!("recorder bypassed the existing session lock: {result:?}");
		}
		result = timeout(DEADLINE, async {
			while lock_count(&observer, "advisory", false).await != 1 {
				sleep(Duration::from_millis(10)).await;
			}
		}) => result.expect("recorder must wait for the existing migration lock"),
	}
	legacy_session.close().await.expect("release legacy lock");

	// Assert
	timeout(DEADLINE, &mut initialization)
		.await
		.expect("recorder proceeds after the legacy lock is released")
		.expect("initialize schema");
	assert_eq!(lock_count(&observer, "advisory", true).await, 0);
}

#[rstest]
#[tokio::test]
async fn cancelled_schema_creation_releases_lock(#[future] postgres_container: PostgresFixture) {
	// Arrange: hold a write transaction so CREATE INDEX waits after taking
	// the advisory lock. The transaction guard also cleans up on test failure.
	let (_container, observer, _port, url) = postgres_container.await;
	let recorder = DatabaseMigrationRecorder::new(warmed_connection(&url, 2).await);
	let create_table = Query::create_table()
		.table(Alias::new("reinhardt_migrations"))
		.col(ColumnDef::new("app").string_len(255))
		.col(ColumnDef::new("name").string_len(255))
		.to_string(PostgresQueryBuilder);
	sqlx::query(&create_table)
		.execute(observer.as_ref())
		.await
		.expect("create table without unique index");
	let mut blocker = observer.begin().await.expect("begin blocking transaction");
	let update = Query::update()
		.table(Alias::new("reinhardt_migrations"))
		.value(Alias::new("app"), "lock_test")
		.to_string(PostgresQueryBuilder);
	sqlx::query(&update)
		.execute(&mut *blocker)
		.await
		.expect("hold a row-exclusive table lock");

	// Act: cancel only after observing the pending DDL, not after a fixed sleep.
	tokio::select! {
		result = recorder.ensure_schema_table() => {
			panic!("schema creation completed while DDL was blocked: {result:?}");
		}
		result = timeout(DEADLINE, async {
			loop {
				if lock_count(&observer, "advisory", true).await == 1
					&& lock_count(&observer, "relation", false).await == 1
				{
					break;
				}
				sleep(Duration::from_millis(10)).await;
			}
		}) => result.expect("recorder must reach the blocked schema operation"),
	}
	blocker.rollback().await.expect("release the DDL blocker");

	// Assert: dropping the future must not leave its advisory lock in the pool.
	let independent = DatabaseMigrationRecorder::new(warmed_connection(&url, 1).await);
	ensure_schema(&independent).await;
	assert_eq!(lock_count(&observer, "advisory", true).await, 0);
	ensure_schema(&recorder).await;
}
