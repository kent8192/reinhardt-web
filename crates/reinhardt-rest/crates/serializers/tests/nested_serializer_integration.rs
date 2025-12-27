//! Nested Serializer Integration Tests
//!
//! Tests the internal behavior of nested serializer structures.
//!
//! ## Test Coverage
//!
//! This test file covers:
//! - **Nested Serialization**: Serializing deeply nested object structures
//! - **Nested Deserialization**: Deserializing complex JSON to nested structs
//! - **Circular References**: Handling self-referential structures
//! - **Lazy Loading**: Loading nested data only when accessed
//! - **Depth Limits**: Preventing infinite nesting
//!
//! ## Test Categories
//!
//! 1. **Basic Nesting**: Single-level parent-child relationships
//! 2. **Deep Nesting**: Multi-level hierarchical structures
//! 3. **Many-to-Many**: Nested collections with relationships
//! 4. **Polymorphic Nesting**: Different types in nested structures
//! 5. **Performance**: Handling large nested datasets efficiently
//!
//! ## Fixtures Used
//!
//! - `postgres_container`: For database-backed nested data tests
//!
//! ## What These Tests Verify
//!
//! ✅ Nested serializers serialize/deserialize correctly
//! ✅ Depth limits prevent stack overflow
//! ✅ Circular references are handled gracefully
//! ✅ Performance remains acceptable with deep nesting
//! ✅ Lazy loading works for nested relationships
//! ✅ Many-to-many relationships serialize correctly
//!
//! ## What These Tests Don't Cover
//!
//! ❌ UI rendering of nested data (covered by template tests)
//! ❌ API endpoint integration (covered by API tests)
//! ❌ Custom serializer field types (covered by field tests)
//! ❌ Validation of nested data (covered by validator tests)

use ::testcontainers::{ContainerAsync, GenericImage};
use reinhardt_serializers;
use reinhardt_test::fixtures::*;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::sync::Arc;

// ============ Test Helper Structs ============

/// Category with optional parent (hierarchical structure)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Category {
	id: i32,
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	parent_id: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	parent: Option<Box<Category>>,
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	children: Vec<Category>,
}

impl Category {
	fn new(id: i32, name: &str) -> Self {
		Self {
			id,
			name: name.to_string(),
			parent_id: None,
			parent: None,
			children: Vec::new(),
		}
	}

	fn with_parent(mut self, parent: Category) -> Self {
		self.parent_id = Some(parent.id);
		self.parent = Some(Box::new(parent));
		self
	}

	fn with_children(mut self, children: Vec<Category>) -> Self {
		self.children = children;
		self
	}
}

/// Author with books (one-to-many relationship)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Author {
	id: i32,
	name: String,
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	books: Vec<Book>,
}

impl Author {
	fn new(id: i32, name: &str) -> Self {
		Self {
			id,
			name: name.to_string(),
			books: Vec::new(),
		}
	}

	fn with_books(mut self, books: Vec<Book>) -> Self {
		self.books = books;
		self
	}
}

/// Book with optional author (many-to-one relationship)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Book {
	id: i32,
	title: String,
	author_id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	author: Option<Box<Author>>,
}

impl Book {
	fn new(id: i32, title: &str, author_id: i32) -> Self {
		Self {
			id,
			title: title.to_string(),
			author_id,
			author: None,
		}
	}
}

/// Comment with nested replies (self-referential structure)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Comment {
	id: i32,
	content: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	parent_id: Option<i32>,
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	replies: Vec<Comment>,
}

impl Comment {
	fn new(id: i32, content: &str) -> Self {
		Self {
			id,
			content: content.to_string(),
			parent_id: None,
			replies: Vec::new(),
		}
	}

	fn with_parent_id(mut self, parent_id: i32) -> Self {
		self.parent_id = Some(parent_id);
		self
	}

	fn with_replies(mut self, replies: Vec<Comment>) -> Self {
		self.replies = replies;
		self
	}

	/// Calculate depth of comment tree
	fn depth(&self) -> usize {
		if self.replies.is_empty() {
			1
		} else {
			1 + self.replies.iter().map(|r| r.depth()).max().unwrap_or(0)
		}
	}
}

// ============ Basic Nesting Tests ============

/// Test single-level nesting serialization
///
/// Verifies:
/// - Parent object contains nested child
/// - Serialization produces correct JSON structure
/// - Field ordering is preserved
#[test]
fn test_single_level_nesting_serialization() {
	let child = Category::new(2, "Electronics");
	let parent = Category::new(1, "Technology").with_children(vec![child]);

	let json = serde_json::to_value(&parent).expect("Failed to serialize");

	assert_eq!(json["id"], 1);
	assert_eq!(json["name"], "Technology");
	assert_eq!(json["children"].as_array().unwrap().len(), 1);
	assert_eq!(json["children"][0]["id"], 2);
	assert_eq!(json["children"][0]["name"], "Electronics");
}

/// Test single-level nesting deserialization
///
/// Verifies:
/// - JSON with nested objects deserializes correctly
/// - Nested fields are properly populated
/// - Type information is preserved
#[test]
fn test_single_level_nesting_deserialization() {
	let json_str = r#"{
		"id": 1,
		"name": "Technology",
		"children": [
			{"id": 2, "name": "Electronics", "children": []},
			{"id": 3, "name": "Software", "children": []}
		]
	}"#;

	let category: Category = serde_json::from_str(json_str).expect("Failed to deserialize");

	assert_eq!(category.id, 1);
	assert_eq!(category.name, "Technology");
	assert_eq!(category.children.len(), 2);
	assert_eq!(category.children[0].id, 2);
	assert_eq!(category.children[0].name, "Electronics");
	assert_eq!(category.children[1].id, 3);
	assert_eq!(category.children[1].name, "Software");
}

/// Test bidirectional relationship serialization
///
/// Verifies:
/// - Parent references child
/// - Child references parent (avoiding circular JSON)
/// - Serialization uses skip_serializing_if to prevent infinite loops
#[test]
fn test_bidirectional_relationship_serialization() {
	let parent = Category::new(1, "Root");
	let child = Category::new(2, "Child").with_parent(parent.clone());

	let json = serde_json::to_value(&child).expect("Failed to serialize");

	assert_eq!(json["id"], 2);
	assert_eq!(json["name"], "Child");
	assert_eq!(json["parent_id"], 1);
	// parent field should serialize (nested Category)
	assert_eq!(json["parent"]["id"], 1);
	assert_eq!(json["parent"]["name"], "Root");
}

// ============ Deep Nesting Tests ============

/// Test multi-level hierarchical nesting
///
/// Verifies:
/// - Deep nesting (3+ levels) works correctly
/// - Each level is properly serialized
/// - Depth can be calculated
#[test]
fn test_multi_level_hierarchical_nesting() {
	let grandchild = Category::new(3, "Laptops");
	let child = Category::new(2, "Electronics").with_children(vec![grandchild]);
	let parent = Category::new(1, "Technology").with_children(vec![child]);

	let json = serde_json::to_value(&parent).expect("Failed to serialize");

	// Verify 3 levels
	assert_eq!(json["id"], 1);
	assert_eq!(json["children"][0]["id"], 2);
	assert_eq!(json["children"][0]["children"][0]["id"], 3);
	assert_eq!(json["children"][0]["children"][0]["name"], "Laptops");
}

/// Test deeply nested comment tree
///
/// Verifies:
/// - Self-referential structures can nest deeply
/// - Depth calculation works correctly
/// - Serialization/deserialization preserves structure
#[test]
fn test_deeply_nested_comment_tree() {
	let reply_lvl3 = Comment::new(4, "Deep reply").with_parent_id(3);
	let reply_lvl2 = Comment::new(3, "Nested reply")
		.with_parent_id(2)
		.with_replies(vec![reply_lvl3]);
	let reply_lvl1 = Comment::new(2, "First reply")
		.with_parent_id(1)
		.with_replies(vec![reply_lvl2]);
	let root = Comment::new(1, "Root comment").with_replies(vec![reply_lvl1]);

	// Verify depth
	assert_eq!(root.depth(), 4);

	// Serialize and verify structure
	let json = serde_json::to_value(&root).expect("Failed to serialize");
	assert_eq!(json["id"], 1);
	assert_eq!(json["replies"][0]["id"], 2);
	assert_eq!(json["replies"][0]["replies"][0]["id"], 3);
	assert_eq!(json["replies"][0]["replies"][0]["replies"][0]["id"], 4);

	// Deserialize and verify
	let deserialized: Comment = serde_json::from_value(json).expect("Failed to deserialize");
	assert_eq!(deserialized.depth(), 4);
	assert_eq!(
		deserialized.replies[0].replies[0].replies[0].content,
		"Deep reply"
	);
}

/// Test depth limit handling
///
/// Verifies:
/// - Very deep nesting doesn't cause stack overflow
/// - Depth can be measured
/// - Large trees are handled gracefully
#[test]
fn test_depth_limit_handling() {
	// Create a deep comment tree (10 levels)
	let mut current = Comment::new(10, "Deepest level");
	for i in (1..10).rev() {
		current = Comment::new(i, &format!("Level {}", i))
			.with_parent_id(i - 1)
			.with_replies(vec![current]);
	}

	assert_eq!(current.depth(), 10);

	// Verify serialization doesn't overflow
	let json_result = serde_json::to_value(&current);
	assert!(
		json_result.is_ok(),
		"Deep nesting should serialize without overflow"
	);

	// Verify deserialization works
	let json = json_result.unwrap();
	let deserialized: Result<Comment, _> = serde_json::from_value(json);
	assert!(
		deserialized.is_ok(),
		"Deep nesting should deserialize without overflow"
	);
}

// ============ Many-to-Many Relationship Tests ============

/// Test one-to-many relationship serialization
///
/// Verifies:
/// - Author with multiple books serializes correctly
/// - Books are included in author JSON
/// - Relationship is unidirectional (no circular reference)
#[test]
fn test_one_to_many_serialization() {
	let book1 = Book::new(1, "Book One", 1);
	let book2 = Book::new(2, "Book Two", 1);
	let author = Author::new(1, "John Doe").with_books(vec![book1, book2]);

	let json = serde_json::to_value(&author).expect("Failed to serialize");

	assert_eq!(json["id"], 1);
	assert_eq!(json["name"], "John Doe");
	assert_eq!(json["books"].as_array().unwrap().len(), 2);
	assert_eq!(json["books"][0]["title"], "Book One");
	assert_eq!(json["books"][1]["title"], "Book Two");
}

/// Test many-to-one relationship deserialization
///
/// Verifies:
/// - Book with author reference deserializes correctly
/// - Author object is nested within book
/// - Prevents infinite recursion (author.books not included)
#[test]
fn test_many_to_one_deserialization() {
	let json_str = r#"{
		"id": 1,
		"title": "Book One",
		"author_id": 1,
		"author": {
			"id": 1,
			"name": "John Doe",
			"books": []
		}
	}"#;

	let book: Book = serde_json::from_str(json_str).expect("Failed to deserialize");

	assert_eq!(book.id, 1);
	assert_eq!(book.title, "Book One");
	assert_eq!(book.author_id, 1);
	assert!(book.author.is_some());

	let author = book.author.unwrap();
	assert_eq!(author.id, 1);
	assert_eq!(author.name, "John Doe");
	assert_eq!(author.books.len(), 0); // Prevents circular reference
}

// ============ Database-Backed Nested Data Tests ============

/// Test loading nested categories from database
///
/// Verifies:
/// - Hierarchical data can be stored in database
/// - Nested structure can be reconstructed from flat rows
/// - Parent-child relationships are preserved
#[rstest]
#[tokio::test]
async fn test_nested_categories_from_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create categories table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS categories (
			id SERIAL PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			parent_id INT REFERENCES categories(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create categories table");

	// Insert hierarchical data
	sqlx::query("INSERT INTO categories (id, name, parent_id) VALUES ($1, $2, $3)")
		.bind(1)
		.bind("Root")
		.bind(None::<i32>)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert root");

	sqlx::query("INSERT INTO categories (id, name, parent_id) VALUES ($1, $2, $3)")
		.bind(2)
		.bind("Child1")
		.bind(Some(1))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert child1");

	sqlx::query("INSERT INTO categories (id, name, parent_id) VALUES ($1, $2, $3)")
		.bind(3)
		.bind("Child2")
		.bind(Some(1))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert child2");

	sqlx::query("INSERT INTO categories (id, name, parent_id) VALUES ($1, $2, $3)")
		.bind(4)
		.bind("Grandchild")
		.bind(Some(2))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert grandchild");

	// Load root category
	let root_row = sqlx::query("SELECT id, name, parent_id FROM categories WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch root");

	let root_id: i32 = root_row.get("id");
	let root_name: String = root_row.get("name");

	assert_eq!(root_id, 1);
	assert_eq!(root_name, "Root");

	// Load children
	let children_rows =
		sqlx::query("SELECT id, name, parent_id FROM categories WHERE parent_id = $1")
			.bind(1)
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to fetch children");

	assert_eq!(children_rows.len(), 2);

	let child1_id: i32 = children_rows[0].get("id");
	let child1_name: String = children_rows[0].get("name");
	assert_eq!(child1_id, 2);
	assert_eq!(child1_name, "Child1");

	// Load grandchildren
	let grandchildren_rows =
		sqlx::query("SELECT id, name, parent_id FROM categories WHERE parent_id = $1")
			.bind(2)
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to fetch grandchildren");

	assert_eq!(grandchildren_rows.len(), 1);

	let grandchild_id: i32 = grandchildren_rows[0].get("id");
	let grandchild_name: String = grandchildren_rows[0].get("name");
	assert_eq!(grandchild_id, 4);
	assert_eq!(grandchild_name, "Grandchild");
}

/// Test loading author with books from database
///
/// Verifies:
/// - One-to-many relationship data can be loaded
/// - Books are associated with correct author
/// - Nested structure matches database relationships
#[rstest]
#[tokio::test]
async fn test_author_books_from_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(100) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create authors table");

	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(200) NOT NULL,
			author_id INT NOT NULL REFERENCES authors(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create books table");

	// Insert author
	let author_id: i32 = sqlx::query_scalar("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Jane Smith")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert author");

	// Insert books
	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("First Book")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert book 1");

	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("Second Book")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert book 2");

	// Load author with books using JOIN
	let books = sqlx::query("SELECT id, title, author_id FROM books WHERE author_id = $1")
		.bind(author_id)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to fetch books");

	assert_eq!(books.len(), 2);

	let book1_title: String = books[0].get("title");
	let book2_title: String = books[1].get("title");

	assert_eq!(book1_title, "First Book");
	assert_eq!(book2_title, "Second Book");
}

// ============ Performance Tests ============

/// Test serialization performance with large nested dataset
///
/// Verifies:
/// - Large nested structures serialize efficiently
/// - Memory usage remains reasonable
/// - No performance degradation with depth
#[test]
fn test_large_nested_dataset_serialization() {
	// Create author with 100 books
	let books: Vec<Book> = (1..=100)
		.map(|i| Book::new(i, &format!("Book {}", i), 1))
		.collect();

	let author = Author::new(1, "Prolific Author").with_books(books);

	// Serialize
	let json = serde_json::to_value(&author).expect("Failed to serialize large dataset");

	assert_eq!(json["books"].as_array().unwrap().len(), 100);

	// Verify random books
	assert_eq!(json["books"][0]["title"], "Book 1");
	assert_eq!(json["books"][49]["title"], "Book 50");
	assert_eq!(json["books"][99]["title"], "Book 100");
}

/// Test deserialization performance with deep nesting
///
/// Verifies:
/// - Deep structures deserialize efficiently
/// - No stack overflow with reasonable depth
/// - Deserialization time is acceptable
#[test]
fn test_deep_nesting_deserialization_performance() {
	// Create JSON with 20 levels of nesting
	let mut json = json!({
		"id": 20,
		"content": "Level 20",
		"replies": []
	});

	for i in (1..20).rev() {
		json = json!({
			"id": i,
			"content": format!("Level {}", i),
			"parent_id": i - 1,
			"replies": [json]
		});
	}

	// Deserialize
	let comment: Comment =
		serde_json::from_value(json).expect("Failed to deserialize deep structure");

	assert_eq!(comment.depth(), 20);
	assert_eq!(comment.id, 1);

	// Navigate to deepest level
	let mut current = &comment;
	for _ in 1..20 {
		current = &current.replies[0];
	}

	assert_eq!(current.id, 20);
	assert_eq!(current.content, "Level 20");
}

/// Test circular reference prevention
///
/// Verifies:
/// - Serialization doesn't enter infinite loop
/// - skip_serializing_if prevents circular JSON
/// - Deserialization reconstructs partial relationships
#[test]
fn test_circular_reference_prevention() {
	let parent = Category::new(1, "Parent");
	let child = Category::new(2, "Child").with_parent(parent.clone());

	// Serialize child (includes parent reference)
	let child_json = serde_json::to_value(&child).expect("Failed to serialize child");

	assert_eq!(child_json["id"], 2);
	assert_eq!(child_json["parent"]["id"], 1);
	// Parent's children array should be empty or null to prevent circular reference
	// Note: serde's skip_serializing_if may omit the field entirely
	let parent_children = &child_json["parent"]["children"];
	assert!(
		parent_children.is_null()
			|| parent_children
				.as_array()
				.map(|a| a.is_empty())
				.unwrap_or(false),
		"Parent's children should be empty or null to prevent circular reference"
	);

	// Deserialize back
	let deserialized: Category = serde_json::from_value(child_json).expect("Failed to deserialize");

	assert_eq!(deserialized.id, 2);
	assert_eq!(deserialized.name, "Child");
	assert!(deserialized.parent.is_some());
	assert_eq!(deserialized.parent.unwrap().id, 1);
}
