//! Integration tests for snippets application
//!
//! The `snippets` views (`apps::snippets::views::{list, create, retrieve,
//! update, delete}`) take `#[inject] db: DatabaseConnection`, which
//! requires a resolved DI container to invoke directly. Standing up the full
//! DI container in a unit test is out of scope here (see Task A3 notes), so
//! these tests exercise the `Manager::<Snippet>` ORM layer that the handlers
//! delegate to, against a real `DatabaseConnection` backed by a temporary
//! SQLite file. This covers the exact data-access path used by every view
//! (`all_with_db`, `get(..).all_with_db`, `create_with_conn`,
//! `update_with_conn`, `delete_with_conn`) while satisfying the project rule
//! that every test uses at least one Reinhardt component.

#[cfg(with_reinhardt)]
mod tests {
	use reinhardt::DatabaseConnection;
	use reinhardt::db::orm::{Filter, FilterOperator, FilterValue, Manager};
	use reinhardt::test::fixtures::create_table_for_model;
	use rstest::*;
	use tempfile::NamedTempFile;

	use examples_tutorial_rest::apps::snippets::models::Snippet;

	/// Fixture: temporary SQLite database with the `snippets` table created
	/// from the `Snippet` model metadata via `create_table_for_model`.
	#[fixture]
	async fn sqlite_with_migrations() -> (NamedTempFile, DatabaseConnection) {
		// Create temp file
		let temp_file = NamedTempFile::new().expect("Failed to create temp file");
		let db_path = temp_file.path().to_str().unwrap().to_string();

		// `connect_sqlite` automatically sets `create_if_missing(true)`.
		let database_url = format!("sqlite:///{}", db_path);

		let conn = DatabaseConnection::connect_sqlite(&database_url)
			.await
			.expect("Failed to create DatabaseConnection");

		// Create the `snippets` table from the `Snippet` model metadata.
		// `create_table_for_model` operates on the inner backend connection.
		create_table_for_model::<Snippet>(conn.inner())
			.await
			.expect("Failed to create snippets table");

		(temp_file, conn)
	}

	// ============================================================================
	// Unit Tests (2 tests)
	// ============================================================================

	#[rstest]
	fn test_snippet_model() {
		use examples_tutorial_rest::apps::snippets::models::Snippet;

		// Arrange / Act
		let snippet = Snippet::build()
			.title("Hello World")
			.code("println!(\"Hello, world!\");")
			.language("rust")
			.finish();

		// Assert
		assert_eq!(snippet.title, "Hello World");
		assert_eq!(snippet.code, "println!(\"Hello, world!\");");
		assert_eq!(snippet.language, "rust");
	}

	#[rstest]
	fn test_snippet_serializer_validation() {
		use examples_tutorial_rest::apps::snippets::serializers::SnippetSerializer;
		use reinhardt::Validate;

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
	// Database Integration Tests - CRUD Operations
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_create(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let snippet = Snippet::build()
			.title("Test Snippet")
			.code("fn main() {}")
			.language("rust")
			.finish();

		// Act
		let created = Manager::<Snippet>::new()
			.create_with_conn(&conn, &snippet)
			.await
			.expect("Failed to create snippet");

		// Assert
		assert_eq!(created.title, "Test Snippet");
		assert_eq!(created.code, "fn main() {}");
		assert_eq!(created.language, "rust");
		assert!(created.id > 0, "created snippet should have an assigned id");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_read(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let snippet = Snippet::build()
			.title("Read Test")
			.code("println!(\"Hello\");")
			.language("rust")
			.finish();
		let created = Manager::<Snippet>::new()
			.create_with_conn(&conn, &snippet)
			.await
			.expect("Failed to create snippet");

		// Act
		let found = Manager::<Snippet>::new()
			.get(created.id)
			.all_with_db(&conn)
			.await
			.expect("Failed to read snippet");

		// Assert
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].id, created.id);
		assert_eq!(found[0].title, "Read Test");
		assert_eq!(found[0].code, "println!(\"Hello\");");
		assert_eq!(found[0].language, "rust");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_update(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let snippet = Snippet::build()
			.title("Original Title")
			.code("original code")
			.language("python")
			.finish();
		let manager = Manager::<Snippet>::new();
		let created = manager
			.create_with_conn(&conn, &snippet)
			.await
			.expect("Failed to create snippet");

		let mut to_update = created.clone();
		to_update.title = "Updated Title".to_string();
		to_update.code = "updated code".to_string();
		to_update.language = "javascript".to_string();

		// Act
		let updated = manager
			.update_with_conn(&conn, &to_update)
			.await
			.expect("Failed to update snippet");

		// Assert
		assert_eq!(updated.id, created.id);
		assert_eq!(updated.title, "Updated Title");
		assert_eq!(updated.code, "updated code");
		assert_eq!(updated.language, "javascript");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_delete(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let snippet = Snippet::build()
			.title("To Delete")
			.code("delete me")
			.language("rust")
			.finish();
		let manager = Manager::<Snippet>::new();
		let created = manager
			.create_with_conn(&conn, &snippet)
			.await
			.expect("Failed to create snippet");

		// Act
		manager
			.delete_with_conn(&conn, created.id)
			.await
			.expect("Failed to delete snippet");

		// Assert
		let remaining = manager
			.get(created.id)
			.all_with_db(&conn)
			.await
			.expect("Failed to query after deletion");
		assert_eq!(remaining.len(), 0);
	}

	// ============================================================================
	// Database Integration Tests - Query Operations
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_list_all(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for (title, language) in [
			("Snippet 1", "rust"),
			("Snippet 2", "python"),
			("Snippet 3", "javascript"),
		] {
			let snippet = Snippet::build()
				.title(title)
				.code(format!("code for {}", title))
				.language(language)
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act
		let snippets = manager
			.all()
			.order_by(&["id"])
			.all_with_db(&conn)
			.await
			.expect("Failed to list snippets");

		// Assert
		assert_eq!(snippets.len(), 3);
		assert_eq!(snippets[0].title, "Snippet 1");
		assert_eq!(snippets[1].title, "Snippet 2");
		assert_eq!(snippets[2].title, "Snippet 3");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_filter_by_language(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for (title, code, language) in [
			("Rust Snippet", "fn main() {}", "rust"),
			("Python Snippet", "print('hello')", "python"),
			("Another Rust", "let x = 5;", "rust"),
		] {
			let snippet = Snippet::build()
				.title(title)
				.code(code)
				.language(language)
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act
		let filter = Filter::new(
			"language",
			FilterOperator::Eq,
			FilterValue::String("rust".to_string()),
		);
		let rust_snippets = manager
			.filter(filter)
			.order_by(&["id"])
			.all_with_db(&conn)
			.await
			.expect("Failed to filter snippets");

		// Assert
		assert_eq!(rust_snippets.len(), 2);
		assert_eq!(rust_snippets[0].title, "Rust Snippet");
		assert_eq!(rust_snippets[1].title, "Another Rust");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_search_by_title(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for (title, code, language) in [
			("Hello World", "println!(\"Hello\");", "rust"),
			("Goodbye World", "println!(\"Goodbye\");", "rust"),
			("Unrelated", "some code", "python"),
		] {
			let snippet = Snippet::build()
				.title(title)
				.code(code)
				.language(language)
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act
		let filter = Filter::new(
			"title",
			FilterOperator::Contains,
			FilterValue::String("World".to_string()),
		);
		let results = manager
			.filter(filter)
			.order_by(&["id"])
			.all_with_db(&conn)
			.await
			.expect("Failed to search snippets");

		// Assert
		assert_eq!(results.len(), 2);
		assert_eq!(results[0].title, "Hello World");
		assert_eq!(results[1].title, "Goodbye World");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_count(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for i in 1..=4 {
			let snippet = Snippet::build()
				.title(format!("Snippet {}", i))
				.code(format!("code {}", i))
				.language("rust")
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act
		let snippets = manager
			.all()
			.all_with_db(&conn)
			.await
			.expect("Failed to count snippets");

		// Assert
		assert_eq!(snippets.len(), 4);
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_order_by_title(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for title in ["Charlie", "Alpha", "Bravo"] {
			let snippet = Snippet::build()
				.title(title)
				.code("fn main() {}")
				.language("rust")
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act
		let snippets = manager
			.all()
			.order_by(&["title"])
			.all_with_db(&conn)
			.await
			.expect("Failed to order snippets by title");

		// Assert
		assert_eq!(snippets.len(), 3);
		assert_eq!(snippets[0].title, "Alpha");
		assert_eq!(snippets[1].title, "Bravo");
		assert_eq!(snippets[2].title, "Charlie");
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_pagination(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange
		let manager = Manager::<Snippet>::new();
		for i in 1..=5 {
			let snippet = Snippet::build()
				.title(format!("Snippet {}", i))
				.code(format!("code {}", i))
				.language("rust")
				.finish();
			manager
				.create_with_conn(&conn, &snippet)
				.await
				.expect("Failed to create snippet");
		}

		// Act - first page (limit 2, offset 0)
		let page1 = manager
			.all()
			.order_by(&["id"])
			.limit(2)
			.offset(0)
			.all_with_db(&conn)
			.await
			.expect("Failed to fetch page 1");

		// Act - second page (limit 2, offset 2)
		let page2 = manager
			.all()
			.order_by(&["id"])
			.limit(2)
			.offset(2)
			.all_with_db(&conn)
			.await
			.expect("Failed to fetch page 2");

		// Assert
		assert_eq!(page1.len(), 2);
		assert_eq!(page1[0].title, "Snippet 1");
		assert_eq!(page1[1].title, "Snippet 2");

		assert_eq!(page2.len(), 2);
		assert_eq!(page2[0].title, "Snippet 3");
		assert_eq!(page2[1].title, "Snippet 4");
	}

	// ============================================================================
	// Database Integration Tests - Edge Cases
	// ============================================================================

	#[rstest]
	#[tokio::test]
	async fn test_snippet_empty_database(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Act
		let snippets = Manager::<Snippet>::new()
			.all()
			.all_with_db(&conn)
			.await
			.expect("Failed to query empty database");

		// Assert
		assert_eq!(snippets.len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_nonexistent_id(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Act
		let result = Manager::<Snippet>::new()
			.get(99999)
			.all_with_db(&conn)
			.await
			.expect("Failed to query nonexistent id");

		// Assert
		assert_eq!(result.len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_snippet_update_nonexistent(
		#[future] sqlite_with_migrations: (NamedTempFile, DatabaseConnection),
	) {
		let (_file, conn) = sqlite_with_migrations.await;

		// Arrange - a model instance whose primary key has no matching row
		let mut nonexistent = Snippet::build()
			.title("New Title")
			.code("code")
			.language("rust")
			.finish();
		nonexistent.id = 99999;

		// Act
		let result = Manager::<Snippet>::new()
			.update_with_conn(&conn, &nonexistent)
			.await;

		// Assert
		assert!(
			result.is_err(),
			"updating a nonexistent snippet should fail"
		);
	}
}
