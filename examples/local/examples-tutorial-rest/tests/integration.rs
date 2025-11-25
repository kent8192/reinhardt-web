//! Integration tests for snippets application

#[cfg(with_reinhardt)]
mod tests {
	use example_test_macros::example_test;
	use rstest::*;
	use sqlx::PgPool;
	use std::sync::Arc;
	use testcontainers::ContainerAsync;
	use testcontainers_modules::postgres::Postgres;

	#[fixture]
	async fn postgres_with_migrations() -> (ContainerAsync<Postgres>, Arc<PgPool>, String) {
		reinhardt_test::fixtures::testcontainers::postgres_with_migrations("examples-tutorial-rest")
			.await
	}

	// ============================================================================
	// Unit Tests (2 tests)
	// ============================================================================

	#[test]
	fn test_snippet_model() {
		use examples_tutorial_rest::apps::snippets::models::Snippet;

		let snippet = Snippet::new(
			"Hello World".to_string(),
			"println!(\"Hello, world!\");".to_string(),
			"rust".to_string(),
		);

		assert_eq!(snippet.title, "Hello World");
		assert_eq!(snippet.code, "println!(\"Hello, world!\");");
		assert_eq!(snippet.language, "rust");
		assert!(snippet.id.is_none());
	}

	#[test]
	fn test_snippet_serializer_validation() {
		use examples_tutorial_rest::apps::snippets::serializers::SnippetSerializer;
		use validator::Validate;

		// Valid snippet
		let valid = SnippetSerializer {
			title: "Valid".to_string(),
			code: "fn main() {}".to_string(),
			language: "rust".to_string(),
		};
		assert!(valid.validate().is_ok());

		// Invalid: empty title
		let invalid_title = SnippetSerializer {
			title: "".to_string(),
			code: "fn main() {}".to_string(),
			language: "rust".to_string(),
		};
		assert!(invalid_title.validate().is_err());

		// Invalid: empty code
		let invalid_code = SnippetSerializer {
			title: "Valid".to_string(),
			code: "".to_string(),
			language: "rust".to_string(),
		};
		assert!(invalid_code.validate().is_err());
	}

	// ============================================================================
	// Database Integration Tests - CRUD Operations (4 tests)
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_create(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create a snippet
		let result = sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			RETURNING id, title, code, language
			"#,
			"Test Snippet",
			"fn main() {}",
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create snippet");

		assert_eq!(result.title, "Test Snippet");
		assert_eq!(result.code, "fn main() {}");
		assert_eq!(result.language, "rust");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_read(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create a snippet
		let created = sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			RETURNING id
			"#,
			"Read Test",
			"println!(\"Hello\");",
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create snippet");

		// Read the snippet
		let result = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			WHERE id = $1
			"#,
			created.id
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read snippet");

		assert_eq!(result.id, created.id);
		assert_eq!(result.title, "Read Test");
		assert_eq!(result.code, "println!(\"Hello\");");
		assert_eq!(result.language, "rust");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_update(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create a snippet
		let created = sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			RETURNING id
			"#,
			"Original Title",
			"original code",
			"python"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create snippet");

		// Update the snippet
		let updated = sqlx::query!(
			r#"
			UPDATE snippets
			SET title = $1, code = $2, language = $3
			WHERE id = $4
			RETURNING id, title, code, language
			"#,
			"Updated Title",
			"updated code",
			"javascript",
			created.id
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to update snippet");

		assert_eq!(updated.id, created.id);
		assert_eq!(updated.title, "Updated Title");
		assert_eq!(updated.code, "updated code");
		assert_eq!(updated.language, "javascript");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_delete(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create a snippet
		let created = sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			RETURNING id
			"#,
			"To Delete",
			"delete me",
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create snippet");

		// Delete the snippet
		let deleted_rows = sqlx::query!(
			r#"
			DELETE FROM snippets
			WHERE id = $1
			"#,
			created.id
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete snippet")
		.rows_affected();

		assert_eq!(deleted_rows, 1);

		// Verify deletion
		let result = sqlx::query!(
			r#"
			SELECT id FROM snippets WHERE id = $1
			"#,
			created.id
		)
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to verify deletion");

		assert!(result.is_none());
	}

	// ============================================================================
	// Database Integration Tests - Query Operations (4 tests)
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_list_all(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create multiple snippets
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6),
				($7, $8, $9)
			"#,
			"Snippet 1",
			"code 1",
			"rust",
			"Snippet 2",
			"code 2",
			"python",
			"Snippet 3",
			"code 3",
			"javascript"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// List all snippets
		let snippets = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			ORDER BY id
			"#
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to list snippets");

		assert_eq!(snippets.len(), 3);
		assert_eq!(snippets[0].title, "Snippet 1");
		assert_eq!(snippets[1].title, "Snippet 2");
		assert_eq!(snippets[2].title, "Snippet 3");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_filter_by_language(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create snippets with different languages
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6),
				($7, $8, $9)
			"#,
			"Rust Snippet",
			"fn main() {}",
			"rust",
			"Python Snippet",
			"print('hello')",
			"python",
			"Another Rust",
			"let x = 5;",
			"rust"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// Filter by language
		let rust_snippets = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			WHERE language = $1
			ORDER BY id
			"#,
			"rust"
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to filter snippets");

		assert_eq!(rust_snippets.len(), 2);
		assert_eq!(rust_snippets[0].title, "Rust Snippet");
		assert_eq!(rust_snippets[1].title, "Another Rust");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_search_by_title(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create snippets with searchable titles
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6),
				($7, $8, $9)
			"#,
			"Hello World",
			"println!(\"Hello\");",
			"rust",
			"Goodbye World",
			"println!(\"Goodbye\");",
			"rust",
			"Unrelated",
			"some code",
			"python"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// Search by title pattern
		let results = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			WHERE title LIKE $1
			ORDER BY id
			"#,
			"%World%"
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to search snippets");

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].title, "Hello World");
		assert_eq!(results[1].title, "Goodbye World");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_pagination(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create 5 snippets
		for i in 1..=5 {
			sqlx::query!(
				r#"
				INSERT INTO snippets (title, code, language)
				VALUES ($1, $2, $3)
				"#,
				format!("Snippet {}", i),
				format!("code {}", i),
				"rust"
			)
			.execute(pool.as_ref())
			.await
			.expect("Failed to create snippet");
		}

		// First page (limit 2, offset 0)
		let page1 = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			ORDER BY id
			LIMIT $1 OFFSET $2
			"#,
			2i64,
			0i64
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to fetch page 1");

		assert_eq!(page1.len(), 2);
		assert_eq!(page1[0].title, "Snippet 1");
		assert_eq!(page1[1].title, "Snippet 2");

		// Second page (limit 2, offset 2)
		let page2 = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			ORDER BY id
			LIMIT $1 OFFSET $2
			"#,
			2i64,
			2i64
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to fetch page 2");

		assert_eq!(page2.len(), 2);
		assert_eq!(page2[0].title, "Snippet 3");
		assert_eq!(page2[1].title, "Snippet 4");
	}

	// ============================================================================
	// Database Integration Tests - Edge Cases (7 tests)
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_empty_database(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Query empty database
		let snippets = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			"#
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query empty database");

		assert_eq!(snippets.len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_nonexistent_id(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Query with nonexistent ID
		let result = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			WHERE id = $1
			"#,
			99999i64
		)
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to query nonexistent ID");

		assert!(result.is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_duplicate_title_allowed(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create first snippet
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			"#,
			"Duplicate Title",
			"code 1",
			"rust"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create first snippet");

		// Create second snippet with same title (should succeed - no unique constraint)
		let result = sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES ($1, $2, $3)
			RETURNING id
			"#,
			"Duplicate Title",
			"code 2",
			"python"
		)
		.fetch_one(pool.as_ref())
		.await;

		assert!(result.is_ok());

		// Verify both exist
		let count = sqlx::query!(
			r#"
			SELECT COUNT(*) as count
			FROM snippets
			WHERE title = $1
			"#,
			"Duplicate Title"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count snippets");

		assert_eq!(count.count, Some(2));
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_count(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create 3 snippets
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6),
				($7, $8, $9)
			"#,
			"Snippet 1",
			"code 1",
			"rust",
			"Snippet 2",
			"code 2",
			"python",
			"Snippet 3",
			"code 3",
			"rust"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// Count all snippets
		let total = sqlx::query!(
			r#"
			SELECT COUNT(*) as count
			FROM snippets
			"#
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count all snippets");

		assert_eq!(total.count, Some(3));

		// Count rust snippets
		let rust_count = sqlx::query!(
			r#"
			SELECT COUNT(*) as count
			FROM snippets
			WHERE language = $1
			"#,
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count rust snippets");

		assert_eq!(rust_count.count, Some(2));
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_order_by_title(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create snippets with different titles
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6),
				($7, $8, $9)
			"#,
			"Charlie",
			"code c",
			"rust",
			"Alice",
			"code a",
			"python",
			"Bob",
			"code b",
			"javascript"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// Order by title ascending
		let results = sqlx::query!(
			r#"
			SELECT id, title, code, language
			FROM snippets
			ORDER BY title ASC
			"#
		)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to order snippets");

		assert_eq!(results.len(), 3);
		assert_eq!(results[0].title, "Alice");
		assert_eq!(results[1].title, "Bob");
		assert_eq!(results[2].title, "Charlie");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_language_case_sensitivity(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create snippets with different case languages
		sqlx::query!(
			r#"
			INSERT INTO snippets (title, code, language)
			VALUES
				($1, $2, $3),
				($4, $5, $6)
			"#,
			"Lowercase Rust",
			"code 1",
			"rust",
			"Uppercase Rust",
			"code 2",
			"RUST"
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create snippets");

		// Exact match (case-sensitive)
		let exact = sqlx::query!(
			r#"
			SELECT COUNT(*) as count
			FROM snippets
			WHERE language = $1
			"#,
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count exact match");

		assert_eq!(exact.count, Some(1));

		// Case-insensitive match
		let case_insensitive = sqlx::query!(
			r#"
			SELECT COUNT(*) as count
			FROM snippets
			WHERE LOWER(language) = LOWER($1)
			"#,
			"rust"
		)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count case-insensitive match");

		assert_eq!(case_insensitive.count, Some(2));
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_update_nonexistent(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Try to update nonexistent snippet
		let updated_rows = sqlx::query!(
			r#"
			UPDATE snippets
			SET title = $1
			WHERE id = $2
			"#,
			"New Title",
			99999i64
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update nonexistent snippet")
		.rows_affected();

		assert_eq!(updated_rows, 0);
	}
}
