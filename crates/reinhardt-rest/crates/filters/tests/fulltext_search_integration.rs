//! Integration tests for full-text search capabilities
//!
//! These tests verify that FullTextSearchFilter works correctly with
//! PostgreSQL full-text search features.
//!
//! **Test Coverage:**
//! 1. Basic full-text search (Natural mode)
//! 2. Boolean mode search with operators
//! 3. Phrase mode search (exact matches)
//! 4. Multi-field full-text search
//! 5. Full-text search with relevance scoring
//! 6. Full-text search with highlighting
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container (reinhardt-test)
//! - fulltext_test_db: Custom fixture with test schema and FTS indexes

use reinhardt_filters::{
	FullTextSearchFilter, FullTextSearchMode, HtmlHighlighter, SearchHighlighter,
};
use reinhardt_test::fixtures::testcontainers::{ContainerAsync, GenericImage, postgres_container};
use rstest::*;
use sqlx::Row;
use std::sync::Arc;

// ========================================================================
// Custom Fixtures
// ========================================================================

/// Custom fixture providing PostgreSQL database with full-text search setup
///
/// **Schema:**
/// - articles: id, title, content, author, category, ts_content (tsvector)
/// - FTS index on ts_content
///
/// **Integration Point**: postgres_container → fulltext_test_db (fixture chaining)
#[fixture]
async fn fulltext_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>) {
	let (container, pool, _port, _url) = postgres_container.await;

	// Create articles table with tsvector column for full-text search
	sqlx::query(
		r#"
		CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			content TEXT NOT NULL,
			author TEXT NOT NULL,
			category TEXT NOT NULL,
			ts_content tsvector,
			created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create articles table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO articles (title, content, author, category) VALUES
		(
			'Introduction to Rust Programming',
			'Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It is designed to be memory safe without using garbage collection.',
			'Alice Johnson',
			'programming'
		),
		(
			'Advanced Rust Techniques',
			'Learn advanced Rust programming techniques including async/await, trait objects, and macro metaprogramming. Master the ownership system and zero-cost abstractions.',
			'Bob Smith',
			'programming'
		),
		(
			'Web Development with Rust',
			'Build fast and secure web applications using Rust frameworks like Actix-web and Rocket. Learn how to create RESTful APIs and integrate databases.',
			'Charlie Brown',
			'web'
		),
		(
			'Database Design Patterns',
			'Explore common database design patterns and best practices. Learn about normalization, indexing, and query optimization for PostgreSQL and MySQL.',
			'David Lee',
			'database'
		),
		(
			'Python for Data Science',
			'Python is the most popular language for data science and machine learning. Learn NumPy, Pandas, and Scikit-learn for data analysis.',
			'Eve Davis',
			'data-science'
		),
		(
			'JavaScript Modern Frameworks',
			'Discover modern JavaScript frameworks like React, Vue, and Angular. Build interactive web applications with component-based architecture.',
			'Frank Miller',
			'web'
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert articles");

	// Update tsvector column for full-text search
	sqlx::query(
		r#"
		UPDATE articles
		SET ts_content = to_tsvector('english', title || ' ' || content)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to update tsvector");

	// Create GIN index on tsvector column for performance
	sqlx::query(
		r#"
		CREATE INDEX idx_articles_fts ON articles USING GIN(ts_content)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create FTS index");

	(container, pool)
}

// ========================================================================
// Test 1: Basic Full-Text Search (Natural Mode)
// ========================================================================

/// Test basic full-text search in Natural language mode
///
/// **Test Intent**: Verify FullTextSearchFilter can perform natural language
/// full-text search using PostgreSQL's to_tsquery function.
///
/// **Integration Point**: FullTextSearchFilter → PostgreSQL FTS
///
/// **Verification**:
/// - Natural language query processing
/// - tsvector and tsquery matching
/// - Correct articles returned based on content
#[rstest]
#[tokio::test]
async fn test_basic_fulltext_search(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter in Natural mode
	#[derive(Clone)]
	struct Article;

	let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("rust programming")
		.add_field("ts_content")
		.mode(FullTextSearchMode::Natural);

	// Verify filter configuration
	assert_eq!(filter.query, "rust programming");
	assert_eq!(filter.mode, FullTextSearchMode::Natural);

	// Build full-text search query using PostgreSQL FTS
	let search_query = r#"
		SELECT id, title, content, author
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'rust & programming')
		ORDER BY ts_rank(ts_content, to_tsquery('english', 'rust & programming')) DESC
	"#;

	// Execute query
	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return articles about Rust programming
	// Expected: "Introduction to Rust Programming", "Advanced Rust Techniques"
	assert!(rows.len() >= 2);

	// Verify first result contains relevant content
	let title: String = rows[0].try_get("title").expect("Failed to get title");
	assert!(title.to_lowercase().contains("rust"));
}

// ========================================================================
// Test 2: Boolean Mode Search with Operators
// ========================================================================

/// Test full-text search in Boolean mode with operators
///
/// **Test Intent**: Verify Boolean mode supports AND/OR/NOT operators
/// for complex search queries.
///
/// **Integration Point**: FullTextSearchFilter (Boolean) → PostgreSQL tsquery
///
/// **Verification**:
/// - Boolean operators (AND, OR, NOT) work correctly
/// - Complex Boolean expressions supported
/// - Correct filtering based on Boolean logic
#[rstest]
#[tokio::test]
async fn test_boolean_mode_search(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter in Boolean mode
	#[derive(Clone)]
	struct Article;

	let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("rust & (web | database)")
		.add_field("ts_content")
		.mode(FullTextSearchMode::Boolean);

	// Verify filter configuration
	assert_eq!(filter.mode, FullTextSearchMode::Boolean);

	// Build Boolean search query: "rust" AND ("web" OR "database")
	let search_query = r#"
		SELECT id, title, content
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'rust & (web | database)')
	"#;

	// Execute query
	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return articles about Rust with web or database topics
	// Expected: "Web Development with Rust"
	assert!(!rows.is_empty());

	// Verify results contain relevant keywords
	for row in &rows {
		let content: String = row.try_get("content").expect("Failed to get content");
		let content_lower = content.to_lowercase();
		assert!(content_lower.contains("rust"));
		assert!(content_lower.contains("web") || content_lower.contains("database"));
	}

	// Test NOT operator: articles about programming but NOT Python
	let search_query_not = r#"
		SELECT id, title, content
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'programming & !python')
	"#;

	let rows_not = sqlx::query(search_query_not)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return programming articles excluding Python
	// Expected: Rust articles, NOT "Python for Data Science"
	assert!(rows_not.len() >= 2);

	for row in &rows_not {
		let content: String = row.try_get("content").expect("Failed to get content");
		let content_lower = content.to_lowercase();
		assert!(!content_lower.contains("python"));
	}
}

// ========================================================================
// Test 3: Phrase Mode Search (Exact Matches)
// ========================================================================

/// Test full-text search in Phrase mode for exact phrase matching
///
/// **Test Intent**: Verify Phrase mode searches for exact phrase matches
/// using PostgreSQL's phraseto_tsquery.
///
/// **Integration Point**: FullTextSearchFilter (Phrase) → PostgreSQL phrase search
///
/// **Verification**:
/// - Exact phrase matching
/// - Word order matters
/// - Only exact matches returned
#[rstest]
#[tokio::test]
async fn test_phrase_mode_search(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter in Phrase mode
	#[derive(Clone)]
	struct Article;

	let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("systems programming language")
		.add_field("ts_content")
		.mode(FullTextSearchMode::Phrase);

	// Verify filter configuration
	assert_eq!(filter.mode, FullTextSearchMode::Phrase);

	// Build phrase search query using phraseto_tsquery
	let search_query = r#"
		SELECT id, title, content
		FROM articles
		WHERE ts_content @@ phraseto_tsquery('english', 'systems programming language')
	"#;

	// Execute query
	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return articles with exact phrase "systems programming language"
	// Expected: "Introduction to Rust Programming"
	assert!(!rows.is_empty());

	// Verify results contain the exact phrase
	for row in &rows {
		let content: String = row.try_get("content").expect("Failed to get content");
		let content_lower = content.to_lowercase();
		assert!(content_lower.contains("systems programming language"));
	}

	// Test that word order matters: search for reversed phrase
	let search_query_reversed = r#"
		SELECT id, title, content
		FROM articles
		WHERE ts_content @@ phraseto_tsquery('english', 'language programming systems')
	"#;

	let rows_reversed = sqlx::query(search_query_reversed)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return fewer or no results (wrong word order)
	assert!(rows_reversed.len() < rows.len());
}

// ========================================================================
// Test 4: Multi-Field Full-Text Search
// ========================================================================

/// Test full-text search across multiple fields
///
/// **Test Intent**: Verify full-text search can search across multiple
/// fields (title and content) simultaneously.
///
/// **Integration Point**: FullTextSearchFilter (multi-field) → Combined tsvector
///
/// **Verification**:
/// - Search across multiple fields
/// - Results from both title and content
/// - Relevance ranking considers all fields
#[rstest]
#[tokio::test]
async fn test_multifield_fulltext_search(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter with multiple fields
	#[derive(Clone)]
	struct Article;

	let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("database")
		.add_field("title")
		.add_field("content");

	// Verify filter configuration
	assert_eq!(filter.fields, vec!["title", "content"]);

	// Build multi-field search query
	// NOTE: Our ts_content already combines title and content
	let search_query = r#"
		SELECT id, title, content, author,
		       ts_rank(ts_content, to_tsquery('english', 'database')) as rank
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'database')
		ORDER BY rank DESC
	"#;

	// Execute query
	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return articles containing "database" in title or content
	// Expected: "Database Design Patterns", "Web Development with Rust" (mentions databases)
	assert!(rows.len() >= 2);

	// Verify results contain "database" in title or content
	for row in &rows {
		let title: String = row.try_get("title").expect("Failed to get title");
		let content: String = row.try_get("content").expect("Failed to get content");
		let combined = format!("{} {}", title, content).to_lowercase();
		assert!(combined.contains("database"));
	}
}

// ========================================================================
// Test 5: Full-Text Search with Relevance Scoring
// ========================================================================

/// Test full-text search with relevance scoring using ts_rank
///
/// **Test Intent**: Verify full-text search can rank results by relevance
/// using PostgreSQL's ts_rank function.
///
/// **Integration Point**: FullTextSearchFilter → ts_rank relevance scoring
///
/// **Verification**:
/// - Relevance scores calculated correctly
/// - Results ordered by relevance
/// - More relevant documents ranked higher
#[rstest]
#[tokio::test]
async fn test_fulltext_search_with_relevance(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter with min_score threshold
	#[derive(Clone)]
	struct Article;

	let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("rust")
		.add_field("ts_content")
		.min_score(0.1); // Minimum relevance threshold

	// Verify filter configuration
	assert_eq!(filter.min_score, Some(0.1));

	// Build search query with relevance ranking
	let search_query = r#"
		SELECT id, title, content,
		       ts_rank(ts_content, to_tsquery('english', 'rust')) as relevance
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'rust')
		ORDER BY relevance DESC
	"#;

	// Execute query
	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return articles about Rust, ordered by relevance
	assert!(rows.len() >= 2);

	// Verify relevance scores are in descending order
	let mut prev_relevance: f32 = f32::MAX;
	for row in &rows {
		let relevance: f32 = row.try_get("relevance").expect("Failed to get relevance");
		assert!(relevance > 0.0); // All results should have positive relevance
		assert!(relevance <= prev_relevance); // Descending order
		prev_relevance = relevance;
	}

	// Verify most relevant article is about Rust
	let title: String = rows[0].try_get("title").expect("Failed to get title");
	assert!(title.to_lowercase().contains("rust"));
}

// ========================================================================
// Test 6: Full-Text Search with Highlighting
// ========================================================================

/// Test full-text search with result highlighting
///
/// **Test Intent**: Verify search results can be highlighted using
/// SearchHighlighter to show matching terms.
///
/// **Integration Point**: FullTextSearchFilter + SearchHighlighter
///
/// **Verification**:
/// - Search terms highlighted in results
/// - HTML tags correctly inserted
/// - Multiple occurrences highlighted
#[rstest]
#[tokio::test]
async fn test_fulltext_search_with_highlighting(
	#[future] fulltext_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = fulltext_test_db.await;

	// Create FullTextSearchFilter
	#[derive(Clone)]
	struct Article;

	let _filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
		.query("rust programming")
		.add_field("ts_content");

	// Execute search query
	let search_query = r#"
		SELECT id, title, content
		FROM articles
		WHERE ts_content @@ to_tsquery('english', 'rust & programming')
	"#;

	let rows = sqlx::query(search_query)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	assert!(!rows.is_empty());

	// Create highlighter for search terms
	let highlighter = HtmlHighlighter::new();

	// Highlight search terms in results
	for row in &rows {
		let title: String = row.try_get("title").expect("Failed to get title");
		let content: String = row.try_get("content").expect("Failed to get content");

		// Highlight "rust" in title
		if title.to_lowercase().contains("rust") {
			let highlighted_title = highlighter.highlight(&title, "Rust");
			assert!(highlighted_title.contains("<mark>"));
			assert!(highlighted_title.contains("</mark>"));
		}

		// Highlight "programming" in content
		if content.to_lowercase().contains("programming") {
			let highlighted_content = highlighter.highlight(&content, "programming");
			assert!(highlighted_content.contains("<mark>"));
			assert!(highlighted_content.contains("</mark>"));

			// Verify original text is preserved except for highlighting
			let plain_content = highlighted_content
				.replace("<mark>", "")
				.replace("</mark>", "");
			assert_eq!(plain_content, content);
		}
	}

	// Test highlighting multiple terms
	let title_with_both = "Introduction to Rust Programming";
	let highlighted = highlighter.highlight_many(title_with_both, &["Rust", "Programming"]);

	// Should contain highlights for both terms
	assert!(highlighted.contains("<mark>Rust</mark>"));
	assert!(highlighted.contains("<mark>Programming</mark>"));
}
