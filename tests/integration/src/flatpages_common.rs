//! Common test utilities for flatpages tests

use reinhardt_contrib::flatpages::FlatPage;
use sqlx::{Pool, Postgres, Row};
use std::env;
#[cfg(feature = "testcontainers")]
use testcontainers::{GenericImage, runners::AsyncRunner};

pub const TEST_SITE_ID: i64 = 1;

/// Retry database connection with exponential backoff
#[allow(dead_code)]
async fn retry_connect(url: &str, max_retries: u32) -> Pool<Postgres> {
	for i in 0..max_retries {
		tokio::time::sleep(tokio::time::Duration::from_millis(500 * (i + 1) as u64)).await;
		if let Ok(pool) = Pool::<Postgres>::connect(url).await {
			return pool;
		}
	}
	panic!(
		"Failed to connect to testcontainer database after {} retries",
		max_retries
	);
}

/// Setup test database pool using testcontainers
#[cfg(feature = "testcontainers")]
pub async fn setup_test_db() -> Pool<Postgres> {
	// Check if a specific DATABASE_URL is provided (for manual testing)
	if let Ok(database_url) = env::var("TEST_DATABASE_URL") {
		return Pool::<Postgres>::connect(&database_url)
			.await
			.expect("Failed to connect to test database");
	}

	// Otherwise, use testcontainers to automatically start PostgreSQL
	let container = GenericImage::new("postgres", "17-alpine")
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = container
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get container port");

	let database_url = format!("postgres://postgres@localhost:{}/postgres", port);

	// Wait for the database to be fully ready with retry logic
	let pool = retry_connect(&database_url, 10).await;

	// Keep the container alive by leaking it (it will be cleaned up when the test process exits)
	std::mem::forget(container);

	pool
}

/// Setup test database pool without testcontainers (requires manual database setup)
#[cfg(not(feature = "testcontainers"))]
pub async fn setup_test_db() -> Pool<Postgres> {
	// Check if a specific DATABASE_URL is provided (for manual testing)
	if let Ok(database_url) = env::var("TEST_DATABASE_URL") {
		return Pool::<Postgres>::connect(&database_url)
			.await
			.expect("Failed to connect to test database");
	}

	panic!(
		"TEST_DATABASE_URL environment variable must be set when testcontainers feature is not enabled"
	);
}

/// Create test schema and tables
pub async fn create_test_tables(pool: &Pool<Postgres>) {
	// Create flatpages table
	sqlx::query(
		r#"
        CREATE TABLE IF NOT EXISTS flatpages (
            id BIGSERIAL PRIMARY KEY,
            url VARCHAR(255) NOT NULL,
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            enable_comments BOOLEAN NOT NULL DEFAULT FALSE,
            template_name VARCHAR(255),
            registration_required BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(url)
        )
        "#,
	)
	.execute(pool)
	.await
	.expect("Failed to create flatpages table");

	// Create sites table
	sqlx::query(
		r#"
        CREATE TABLE IF NOT EXISTS sites (
            id BIGSERIAL PRIMARY KEY,
            domain VARCHAR(255) NOT NULL,
            name VARCHAR(255) NOT NULL,
            UNIQUE(domain)
        )
        "#,
	)
	.execute(pool)
	.await
	.expect("Failed to create sites table");

	// Create flatpage_sites junction table
	sqlx::query(
		r#"
        CREATE TABLE IF NOT EXISTS flatpage_sites (
            id BIGSERIAL PRIMARY KEY,
            flatpage_id BIGINT NOT NULL REFERENCES flatpages(id) ON DELETE CASCADE,
            site_id BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
            UNIQUE(flatpage_id, site_id)
        )
        "#,
	)
	.execute(pool)
	.await
	.expect("Failed to create flatpage_sites table");
}

/// Clean all test tables
pub async fn cleanup_test_tables(pool: &Pool<Postgres>) {
	let _ = sqlx::query("DROP TABLE IF EXISTS flatpage_sites CASCADE")
		.execute(pool)
		.await;
	let _ = sqlx::query("DROP TABLE IF EXISTS flatpages CASCADE")
		.execute(pool)
		.await;
	let _ = sqlx::query("DROP TABLE IF EXISTS sites CASCADE")
		.execute(pool)
		.await;
}

/// Create a test site
pub async fn create_test_site(pool: &Pool<Postgres>, domain: &str, name: &str) -> i64 {
	let row = sqlx::query(
        "INSERT INTO sites (domain, name) VALUES ($1, $2) ON CONFLICT (domain) DO UPDATE SET name = $2 RETURNING id"
    )
    .bind(domain)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("Failed to create test site");

	row.get("id")
}

/// Create a test flatpage and associate it with a site
pub async fn create_test_flatpage(
	pool: &Pool<Postgres>,
	url: &str,
	title: &str,
	content: &str,
	registration_required: bool,
	site_id: i64,
) -> FlatPage {
	use reinhardt_database::DatabaseConnection;

	let mut flatpage = FlatPage::new(url.to_string(), title.to_string(), content.to_string());
	flatpage.registration_required = registration_required;
	flatpage.save(pool).await.expect("Failed to save flatpage");

	let db = DatabaseConnection::from_postgres_pool(pool.clone());

	// Associate with site using reinhardt-database
	db.insert("flatpage_sites")
		.value("flatpage_id", flatpage.id)
		.value("site_id", site_id)
		.execute()
		.await
		.expect("Failed to associate flatpage with site");

	flatpage
}

/// Clear all flatpages from database
pub async fn clear_flatpages(pool: &Pool<Postgres>) {
	use reinhardt_database::DatabaseConnection;

	let db = DatabaseConnection::from_postgres_pool(pool.clone());

	db.delete("flatpage_sites")
		.execute()
		.await
		.expect("Failed to clear flatpage_sites");

	db.delete("flatpages")
		.execute()
		.await
		.expect("Failed to clear flatpages");
}
