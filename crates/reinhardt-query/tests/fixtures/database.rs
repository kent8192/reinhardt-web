//! Database pool fixture for DML integration tests

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// PostgreSQL database pool fixture
#[fixture]
pub async fn pg_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> Arc<PgPool> {
	let (_container, pool, _port, _url) = postgres_container.await;
	pool
}

/// Empty database pool fixture (alias for pg_pool)
#[fixture]
pub async fn empty_db(#[future] pg_pool: Arc<PgPool>) -> Arc<PgPool> {
	pg_pool.await
}
