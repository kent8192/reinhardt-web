//! Integration tests for hybrid ORM + raw SQL query execution
//!
//! This test suite verifies:
//! - Hybrid ORM + raw SQL queries
//! - Query builder with raw clauses
//! - Raw SQL with ORM result mapping
//! - Complex JOIN with raw SQL
//! - Performance comparison (ORM vs raw)

use rstest::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_query::{Alias, Expr, ExprTrait, Order, PostgresQueryBuilder, Query};
use serde_json::Value;
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// Fixture for PostgreSQL container with test schema
#[fixture]
async fn postgres_container() -> (ContainerAsync<Postgres>, Arc<PgPool>) {
	let container = Postgres::default()
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let host_port = container
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get container port");

	let connection_string = format!(
		"postgres://postgres:postgres@127.0.0.1:{}/postgres",
		host_port
	);

	let pool = PgPool::connect(&connection_string)
		.await
		.expect("Failed to connect to database");

	// Create test tables
	sqlx::query(
		r#"
		CREATE TABLE users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(100) NOT NULL UNIQUE,
			email VARCHAR(255) NOT NULL,
			age INTEGER,
			status VARCHAR(50) DEFAULT 'active',
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(&pool)
	.await
	.expect("Failed to create users table");

	sqlx::query(
		r#"
		CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			title VARCHAR(255) NOT NULL,
			content TEXT,
			view_count INTEGER DEFAULT 0,
			published BOOLEAN DEFAULT FALSE,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(&pool)
	.await
	.expect("Failed to create posts table");

	sqlx::query(
		r#"
		CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
			user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			content TEXT NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(&pool)
	.await
	.expect("Failed to create comments table");

	// Create indexes individually (prepared statements can't handle multiple commands)
	sqlx::query("CREATE INDEX idx_posts_user_id ON posts(user_id)")
		.execute(&pool)
		.await
		.expect("Failed to create idx_posts_user_id");

	sqlx::query("CREATE INDEX idx_posts_published ON posts(published)")
		.execute(&pool)
		.await
		.expect("Failed to create idx_posts_published");

	sqlx::query("CREATE INDEX idx_comments_post_id ON comments(post_id)")
		.execute(&pool)
		.await
		.expect("Failed to create idx_comments_post_id");

	sqlx::query("CREATE INDEX idx_comments_user_id ON comments(user_id)")
		.execute(&pool)
		.await
		.expect("Failed to create idx_comments_user_id");

	// Insert test data
	insert_test_data(&pool).await;

	(container, Arc::new(pool))
}

/// Insert test data for hybrid query tests
async fn insert_test_data(pool: &PgPool) {
	// Insert users
	for i in 1..=10 {
		sqlx::query(
			r#"
			INSERT INTO users (username, email, age, status)
			VALUES ($1, $2, $3, $4)
			"#,
		)
		.bind(format!("user{}", i))
		.bind(format!("user{}@example.com", i))
		.bind(20 + i)
		.bind(if i % 3 == 0 { "inactive" } else { "active" })
		.execute(pool)
		.await
		.expect("Failed to insert user");
	}

	// Insert posts
	for i in 1..=20 {
		let user_id = ((i - 1) % 10) + 1;
		sqlx::query(
			r#"
			INSERT INTO posts (user_id, title, content, view_count, published)
			VALUES ($1, $2, $3, $4, $5)
			"#,
		)
		.bind(user_id)
		.bind(format!("Post Title {}", i))
		.bind(format!("Content for post {}", i))
		.bind(i * 10)
		.bind(i % 2 == 0)
		.execute(pool)
		.await
		.expect("Failed to insert post");
	}

	// Insert comments
	for i in 1..=50 {
		let post_id = ((i - 1) % 20) + 1;
		let user_id = ((i - 1) % 10) + 1;
		sqlx::query(
			r#"
			INSERT INTO comments (post_id, user_id, content)
			VALUES ($1, $2, $3)
			"#,
		)
		.bind(post_id)
		.bind(user_id)
		.bind(format!("Comment content {}", i))
		.execute(pool)
		.await
		.expect("Failed to insert comment");
	}
}

/// Test hybrid query: ORM query builder with raw WHERE clause
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_orm_query_with_raw_where_clause(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query using SeaQuery with custom raw WHERE clause
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
			(Alias::new("users"), Alias::new("email")),
			(Alias::new("users"), Alias::new("age")),
		])
		.from(Alias::new("users"))
		.and_where(Expr::cust("age > 25 AND status = 'active'"))
		.order_by((Alias::new("users"), Alias::new("age")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx (raw SQL without parameters since we used Expr::cust)
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute query");

	// Verify results
	assert!(!rows.is_empty());
	for row in rows {
		let age: i32 = row.get("age");
		assert!(age > 25, "Age should be greater than 25");
	}
}

/// Test hybrid query: Raw SQL with ORM-style result mapping
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_raw_sql_with_orm_result_mapping(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Execute raw SQL
	let rows = sqlx::query(
		r#"
		SELECT id, username, email, age, status
		FROM users
		WHERE age BETWEEN $1 AND $2
		ORDER BY age DESC
		"#,
	)
	.bind(22)
	.bind(28)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to execute raw SQL");

	// Map to structured data (ORM-style)
	let users: Vec<Value> = rows
		.into_iter()
		.map(|row| {
			serde_json::json!({
				"id": row.get::<i32, _>("id"),
				"username": row.get::<String, _>("username"),
				"email": row.get::<String, _>("email"),
				"age": row.get::<i32, _>("age"),
				"status": row.get::<String, _>("status"),
			})
		})
		.collect();

	// Verify results
	assert!(!users.is_empty());
	for user in users {
		let age = user["age"].as_i64().unwrap();
		assert!((22..=28).contains(&age), "Age should be between 22 and 28");
	}
}

/// Test complex JOIN query with hybrid ORM + raw SQL
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_complex_join_hybrid_query(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build complex JOIN using SeaQuery with raw conditions
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
		])
		.expr_as(
			Expr::cust("COUNT(DISTINCT posts.id)"),
			Alias::new("post_count"),
		)
		.expr_as(
			Expr::cust("COUNT(DISTINCT comments.id)"),
			Alias::new("comment_count"),
		)
		.from(Alias::new("users"))
		.left_join(
			Alias::new("posts"),
			Expr::col((Alias::new("posts"), Alias::new("user_id")))
				.equals((Alias::new("users"), Alias::new("id"))),
		)
		.left_join(
			Alias::new("comments"),
			Expr::col((Alias::new("comments"), Alias::new("user_id")))
				.equals((Alias::new("users"), Alias::new("id"))),
		)
		.and_where(Expr::cust("users.status = 'active'"))
		.group_by_col((Alias::new("users"), Alias::new("id")))
		.group_by_col((Alias::new("users"), Alias::new("username")))
		.and_having(Expr::cust("COUNT(DISTINCT posts.id) > 0"))
		.order_by((Alias::new("users"), Alias::new("username")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute complex JOIN query");

	// Verify results
	assert!(!rows.is_empty());
	for row in rows {
		let post_count: i64 = row.get("post_count");
		assert!(post_count > 0, "Post count should be greater than 0");
	}
}

/// Test query builder with raw SQL subquery
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_query_builder_with_raw_subquery(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query with raw SQL subquery
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
			(Alias::new("users"), Alias::new("email")),
		])
		.from(Alias::new("users"))
		.and_where(Expr::cust(
			"id IN (SELECT DISTINCT user_id FROM posts WHERE published = true)",
		))
		.order_by((Alias::new("users"), Alias::new("id")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute query with subquery");

	// Verify results - should only include users with published posts
	assert!(!rows.is_empty());

	// Verify each user has at least one published post
	for row in rows {
		let user_id: i32 = row.get("id");
		let published_count: i64 = sqlx::query_scalar(
			"SELECT COUNT(*) FROM posts WHERE user_id = $1 AND published = true",
		)
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to verify published posts");

		assert!(
			published_count > 0,
			"User should have at least one published post"
		);
	}
}

/// Test hybrid query with window functions
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_hybrid_query_with_window_functions(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query with window function using raw SQL
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("posts"), Alias::new("id")),
			(Alias::new("posts"), Alias::new("title")),
			(Alias::new("posts"), Alias::new("user_id")),
			(Alias::new("posts"), Alias::new("view_count")),
		])
		.expr_as(
			Expr::cust("ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY view_count DESC)"),
			Alias::new("rank"),
		)
		.from(Alias::new("posts"))
		.and_where(Expr::cust("posts.published = true"));

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Wrap in CTE to filter by rank
	let final_sql = format!(
		"WITH ranked_posts AS ({}) SELECT * FROM ranked_posts WHERE rank <= 2",
		sql
	);

	// Execute query with sqlx
	let rows = sqlx::query(&final_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute window function query");

	// Verify results - each user should have at most 2 posts
	assert!(!rows.is_empty());

	let mut user_post_counts = std::collections::HashMap::new();
	for row in rows {
		let user_id: i32 = row.get("user_id");
		let rank: i64 = row.get("rank");

		assert!(rank <= 2, "Rank should be at most 2");

		*user_post_counts.entry(user_id).or_insert(0) += 1;
	}

	for (_user_id, count) in user_post_counts {
		assert!(
			count <= 2,
			"Each user should have at most 2 posts in result"
		);
	}
}

/// Test hybrid aggregation query with raw GROUP BY
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_hybrid_aggregation_with_raw_group_by(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build aggregation query with raw GROUP BY expression
	let mut query = Query::select();
	query
		.expr_as(
			Expr::col((Alias::new("users"), Alias::new("status"))),
			Alias::new("status"),
		)
		.expr_as(Expr::cust("COUNT(*)"), Alias::new("user_count"))
		.expr_as(Expr::cust("AVG(age)"), Alias::new("avg_age"))
		.expr_as(Expr::cust("MIN(age)"), Alias::new("min_age"))
		.expr_as(Expr::cust("MAX(age)"), Alias::new("max_age"))
		.from(Alias::new("users"))
		.group_by_col((Alias::new("users"), Alias::new("status")))
		.and_having(Expr::cust("COUNT(*) > 0"))
		.order_by((Alias::new("users"), Alias::new("status")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute aggregation query");

	// Verify results
	assert!(!rows.is_empty());
	for row in rows {
		let user_count: i64 = row.get("user_count");
		assert!(user_count > 0, "User count should be greater than 0");

		// Verify avg_age is between min_age and max_age
		// PostgreSQL AVG() returns NUMERIC type, so we need Decimal
		let avg_age: Decimal = row
			.try_get("avg_age")
			.expect("Failed to get avg_age as Decimal");
		let min_age: i32 = row.get("min_age");
		let max_age: i32 = row.get("max_age");

		let avg_age_f64 = avg_age.to_f64().expect("Failed to convert Decimal to f64");
		assert!(
			avg_age_f64 >= min_age as f64 && avg_age_f64 <= max_age as f64,
			"Average age should be between min and max"
		);
	}
}

/// Test ORM query builder with raw CASE expression
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_query_builder_with_raw_case_expression(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query with raw CASE expression
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
			(Alias::new("users"), Alias::new("age")),
		])
		.expr_as(
			Expr::cust(
				"CASE WHEN age < 25 THEN 'young' WHEN age < 30 THEN 'adult' ELSE 'senior' END",
			),
			Alias::new("age_group"),
		)
		.from(Alias::new("users"))
		.order_by((Alias::new("users"), Alias::new("age")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute query with CASE expression");

	// Verify results
	assert!(!rows.is_empty());
	for row in rows {
		let age: i32 = row.get("age");
		let age_group: String = row.get("age_group");

		let expected_group = if age < 25 {
			"young"
		} else if age < 30 {
			"adult"
		} else {
			"senior"
		};

		assert_eq!(
			age_group, expected_group,
			"Age group should match expected value"
		);
	}
}

/// Test raw SQL with ORM-style pagination
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_raw_sql_with_orm_pagination(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	let page_size = 5;
	let page = 2; // Second page (offset 5)

	// Execute raw SQL with pagination
	let rows = sqlx::query(
		r#"
		SELECT id, title, user_id, view_count
		FROM posts
		WHERE published = true
		ORDER BY view_count DESC
		LIMIT $1 OFFSET $2
		"#,
	)
	.bind(page_size)
	.bind((page - 1) * page_size)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to execute paginated query");

	// Get total count
	let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE published = true")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get total count");

	// Verify results
	assert_eq!(
		rows.len(),
		page_size as usize,
		"Should return page_size rows"
	);
	assert!(
		total_count >= page_size as i64,
		"Should have enough total rows"
	);

	// Verify ordering
	let mut prev_view_count = i32::MAX;
	for row in rows {
		let view_count: i32 = row.get("view_count");
		assert!(
			view_count <= prev_view_count,
			"Results should be ordered by view_count DESC"
		);
		prev_view_count = view_count;
	}
}

/// Test hybrid query with JSON aggregation
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_hybrid_query_with_json_aggregation(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query with JSON aggregation using raw SQL
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
		])
		.expr_as(
			Expr::cust("JSON_AGG(JSON_BUILD_OBJECT('id', posts.id, 'title', posts.title))"),
			Alias::new("posts"),
		)
		.from(Alias::new("users"))
		.left_join(
			Alias::new("posts"),
			Expr::col((Alias::new("posts"), Alias::new("user_id")))
				.equals((Alias::new("users"), Alias::new("id"))),
		)
		.and_where(Expr::cust("posts.published = true"))
		.group_by_col((Alias::new("users"), Alias::new("id")))
		.group_by_col((Alias::new("users"), Alias::new("username")))
		.and_having(Expr::cust("COUNT(posts.id) > 0"));

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Add LIMIT to raw SQL to avoid parameterized queries
	let sql_with_limit = format!("{} LIMIT 5", sql);

	// Execute query with sqlx
	let rows = sqlx::query(&sql_with_limit)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute JSON aggregation query");

	// Verify results
	assert!(!rows.is_empty());
	for row in rows {
		let posts_json: Value = row.get("posts");
		assert!(posts_json.is_array(), "Posts should be a JSON array");

		let posts_array = posts_json.as_array().unwrap();
		assert!(!posts_array.is_empty(), "Posts array should not be empty");

		for post in posts_array {
			assert!(post.get("id").is_some(), "Post should have id field");
			assert!(post.get("title").is_some(), "Post should have title field");
		}
	}
}

/// Test performance comparison: ORM vs raw SQL
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_performance_comparison_orm_vs_raw(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Test 1: ORM query builder approach
	let start_orm = std::time::Instant::now();

	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
		])
		.expr_as(Expr::cust("COUNT(posts.id)"), Alias::new("post_count"))
		.from(Alias::new("users"))
		.left_join(
			Alias::new("posts"),
			Expr::col((Alias::new("posts"), Alias::new("user_id")))
				.equals((Alias::new("users"), Alias::new("id"))),
		)
		.group_by_col((Alias::new("users"), Alias::new("id")))
		.group_by_col((Alias::new("users"), Alias::new("username")))
		.order_by((Alias::new("users"), Alias::new("id")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	let orm_rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute ORM query");

	let orm_duration = start_orm.elapsed();

	// Test 2: Raw SQL approach
	let start_raw = std::time::Instant::now();

	let raw_rows = sqlx::query(
		r#"
		SELECT users.id, users.username, COUNT(posts.id) AS post_count
		FROM users
		LEFT JOIN posts ON posts.user_id = users.id
		GROUP BY users.id, users.username
		ORDER BY users.id ASC
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to execute raw SQL query");

	let raw_duration = start_raw.elapsed();

	// Verify both approaches return same results
	assert_eq!(
		orm_rows.len(),
		raw_rows.len(),
		"ORM and raw SQL should return same number of rows"
	);

	for (orm_row, raw_row) in orm_rows.iter().zip(raw_rows.iter()) {
		let orm_id: i32 = orm_row.get("id");
		let raw_id: i32 = raw_row.get("id");
		assert_eq!(orm_id, raw_id, "IDs should match");

		let orm_count: i64 = orm_row.get("post_count");
		let raw_count: i64 = raw_row.get("post_count");
		assert_eq!(orm_count, raw_count, "Post counts should match");
	}

	// Log performance comparison (no strict assertion, just informational)
	println!("ORM duration: {:?}", orm_duration);
	println!("Raw SQL duration: {:?}", raw_duration);
	println!(
		"Performance ratio (ORM/Raw): {:.2}",
		orm_duration.as_secs_f64() / raw_duration.as_secs_f64()
	);
}

/// Test hybrid query with CTE (Common Table Expression)
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_hybrid_query_with_cte(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build CTE query combining ORM and raw SQL
	let mut inner_query = Query::select();
	inner_query
		.columns([(Alias::new("posts"), Alias::new("user_id"))])
		.expr_as(Expr::cust("COUNT(*)"), Alias::new("post_count"))
		.from(Alias::new("posts"))
		.and_where(Expr::cust("posts.published = true"))
		.group_by_col((Alias::new("posts"), Alias::new("user_id")))
		.and_having(Expr::cust("COUNT(*) > 1"));

	let (inner_sql, _inner_values) = inner_query.build(PostgresQueryBuilder);

	// Construct CTE with raw SQL
	let cte_sql = format!(
		r#"
		WITH active_users AS ({})
		SELECT u.id, u.username, au.post_count
		FROM users u
		INNER JOIN active_users au ON u.id = au.user_id
		ORDER BY au.post_count DESC
		"#,
		inner_sql
	);

	// Execute CTE query with sqlx
	let rows = sqlx::query(&cte_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute CTE query");

	// Verify results
	assert!(!rows.is_empty());

	let mut prev_post_count = i64::MAX;
	for row in rows {
		let post_count: i64 = row.get("post_count");
		assert!(post_count > 1, "Post count should be greater than 1");
		assert!(
			post_count <= prev_post_count,
			"Results should be ordered by post_count DESC"
		);
		prev_post_count = post_count;
	}
}

/// Test hybrid query with raw EXISTS subquery
#[rstest]
#[serial(hybrid_query)]
#[tokio::test]
async fn test_hybrid_query_with_exists_subquery(
	#[future] postgres_container: (ContainerAsync<Postgres>, Arc<PgPool>),
) {
	let (_container, pool) = postgres_container.await;

	// Build query with EXISTS subquery
	let mut query = Query::select();
	query
		.columns([
			(Alias::new("users"), Alias::new("id")),
			(Alias::new("users"), Alias::new("username")),
			(Alias::new("users"), Alias::new("email")),
		])
		.from(Alias::new("users"))
		.and_where(Expr::cust(
			r#"EXISTS (
				SELECT 1 FROM posts
				WHERE posts.user_id = users.id
				AND posts.published = true
				AND posts.view_count > 50
			)"#,
		))
		.order_by((Alias::new("users"), Alias::new("id")), Order::Asc);

	let (sql, _values) = query.build(PostgresQueryBuilder);

	// Execute query with sqlx
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute EXISTS subquery");

	// Verify results - each user should have at least one published post with view_count > 50
	assert!(!rows.is_empty());

	for row in rows {
		let user_id: i32 = row.get("id");
		let matching_posts: i64 = sqlx::query_scalar(
			r#"
			SELECT COUNT(*)
			FROM posts
			WHERE user_id = $1
			AND published = true
			AND view_count > 50
			"#,
		)
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to verify matching posts");

		assert!(
			matching_posts > 0,
			"User should have at least one matching post"
		);
	}
}
