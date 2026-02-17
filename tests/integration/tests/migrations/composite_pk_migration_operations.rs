//! Migration operation tests for composite primary key support
//!
//! Tests SQL generation for CREATE TABLE and DROP TABLE operations with composite primary keys.

use reinhardt_db::backends::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
use reinhardt_db::migrations::FieldType;
use reinhardt_db::migrations::operations::FieldDefinition;
use reinhardt_db::migrations::operations::models::{CreateModel, DeleteModel};
use rstest::rstest;

/// Mock schema editor for testing SQL generation
struct MockSchemaEditor;

#[async_trait::async_trait]
impl BaseDatabaseSchemaEditor for MockSchemaEditor {
	async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
		// Mock implementation - does nothing
		Ok(())
	}
}

#[rstest]
fn test_composite_pk_rollback_migration() {
	// Create a composite PK table
	let create = CreateModel::new(
		"post_tags",
		vec![
			FieldDefinition::new("post_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("tag_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new(
				"created_at",
				FieldType::DateTime,
				false,
				false,
				None::<&str>,
			),
		],
	)
	.with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()])
	.expect("Valid composite primary key");

	let schema_editor = MockSchemaEditor;
	let create_sql = create.database_forwards(&schema_editor);

	assert_eq!(create_sql.len(), 1);
	assert!(create_sql[0].contains("CREATE TABLE"));

	// Test rollback (DROP TABLE)
	let delete = DeleteModel::new("post_tags");
	let drop_sql = delete.database_forwards(&schema_editor);

	assert_eq!(drop_sql.len(), 1);
	let sql = &drop_sql[0];

	println!("Rollback SQL:\n{}", sql);

	// Verify DROP TABLE statement
	assert!(
		sql.contains("DROP TABLE"),
		"SQL should contain DROP TABLE: {}",
		sql
	);
	assert!(
		sql.contains("post_tags") || sql.contains("\"post_tags\""),
		"SQL should contain table name 'post_tags': {}",
		sql
	);
}

#[rstest]
fn test_composite_pk_sql_syntax_validation() {
	let create = CreateModel::new(
		"order_items",
		vec![
			FieldDefinition::new("order_id", FieldType::BigInteger, true, false, None::<&str>),
			FieldDefinition::new("item_id", FieldType::BigInteger, true, false, None::<&str>),
			FieldDefinition::new("quantity", FieldType::Integer, false, false, Some("1")),
			FieldDefinition::new(
				"price",
				FieldType::Decimal {
					precision: 10,
					scale: 2,
				},
				false,
				false,
				None::<&str>,
			),
		],
	)
	.with_composite_primary_key(vec!["order_id".to_string(), "item_id".to_string()])
	.expect("Valid composite primary key");

	let schema_editor = MockSchemaEditor;
	let sql_statements = create.database_forwards(&schema_editor);

	assert_eq!(sql_statements.len(), 1);
	let sql = &sql_statements[0];

	println!("Generated SQL:\n{}", sql);

	// Verify SQL structure
	assert!(
		sql.starts_with("CREATE TABLE"),
		"SQL should start with CREATE TABLE"
	);

	// Verify table name
	assert!(
		sql.contains("order_items") || sql.contains("\"order_items\""),
		"SQL should contain table name: {}",
		sql
	);

	// Verify composite PK fields are NOT NULL
	assert!(
		sql.contains("order_id BIGINT NOT NULL") || sql.contains("\"order_id\" BIGINT NOT NULL"),
		"order_id should have NOT NULL constraint: {}",
		sql
	);
	assert!(
		sql.contains("item_id BIGINT NOT NULL") || sql.contains("\"item_id\" BIGINT NOT NULL"),
		"item_id should have NOT NULL constraint: {}",
		sql
	);

	// Verify composite PK fields do NOT have individual PRIMARY KEY
	assert!(
		!sql.contains("order_id BIGINT PRIMARY KEY"),
		"order_id should NOT have individual PRIMARY KEY: {}",
		sql
	);
	assert!(
		!sql.contains("item_id BIGINT PRIMARY KEY"),
		"item_id should NOT have individual PRIMARY KEY: {}",
		sql
	);

	// Verify table-level composite PRIMARY KEY constraint
	assert!(
		sql.contains("PRIMARY KEY (order_id, item_id)")
			|| sql.contains("PRIMARY KEY (\"order_id\", \"item_id\")"),
		"SQL should contain composite PRIMARY KEY constraint: {}",
		sql
	);

	// Verify non-PK fields
	assert!(
		sql.contains("quantity INTEGER") || sql.contains("\"quantity\" INTEGER"),
		"quantity field should exist: {}",
		sql
	);
	assert!(
		sql.contains("DEFAULT 1") || sql.contains("default 1"),
		"quantity should have DEFAULT 1: {}",
		sql
	);
	assert!(
		sql.contains("price DECIMAL(10, 2)") || sql.contains("\"price\" DECIMAL(10, 2)"),
		"price field should exist with correct type: {}",
		sql
	);

	// Verify SQL ends properly
	assert!(
		sql.ends_with(");") || sql.ends_with(")"),
		"SQL should end with closing parenthesis"
	);
}

#[rstest]
fn test_composite_pk_with_unique_constraint() {
	// Test composite PK with additional UNIQUE constraint on PK field
	let create = CreateModel::new(
		"user_sessions",
		vec![
			FieldDefinition::new("user_id", FieldType::Integer, true, true, None::<&str>), // Primary + Unique
			FieldDefinition::new(
				"session_id",
				FieldType::VarChar(255),
				true,
				false,
				None::<&str>,
			),
			FieldDefinition::new(
				"created_at",
				FieldType::DateTime,
				false,
				false,
				None::<&str>,
			),
		],
	)
	.with_composite_primary_key(vec!["user_id".to_string(), "session_id".to_string()])
	.expect("Valid composite primary key");

	let schema_editor = MockSchemaEditor;
	let sql_statements = create.database_forwards(&schema_editor);

	assert_eq!(sql_statements.len(), 1);
	let sql = &sql_statements[0];

	println!("Generated SQL with UNIQUE:\n{}", sql);

	// Verify composite PK constraint exists
	assert!(
		sql.contains("PRIMARY KEY (user_id, session_id)")
			|| sql.contains("PRIMARY KEY (\"user_id\", \"session_id\")"),
		"SQL should contain composite PRIMARY KEY: {}",
		sql
	);

	// Verify UNIQUE constraint on user_id (in addition to being part of composite PK)
	assert!(
		sql.contains("user_id INTEGER UNIQUE NOT NULL")
			|| sql.contains("\"user_id\" INTEGER UNIQUE NOT NULL"),
		"user_id should have UNIQUE and NOT NULL: {}",
		sql
	);

	// Verify session_id does NOT have individual PRIMARY KEY but has NOT NULL
	assert!(
		!sql.contains("session_id VARCHAR(255) PRIMARY KEY"),
		"session_id should NOT have individual PRIMARY KEY: {}",
		sql
	);
	assert!(
		sql.contains("session_id VARCHAR(255) NOT NULL")
			|| sql.contains("\"session_id\" VARCHAR(255) NOT NULL"),
		"session_id should have NOT NULL: {}",
		sql
	);
}

#[rstest]
fn test_composite_pk_forward_backward_consistency() {
	let table_name = "product_categories";
	let pk_fields = vec!["product_id".to_string(), "category_id".to_string()];

	// Forward migration: CREATE TABLE
	let create = CreateModel::new(
		table_name,
		vec![
			FieldDefinition::new("product_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("category_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("display_order", FieldType::Integer, false, false, Some("0")),
		],
	)
	.with_composite_primary_key(pk_fields.clone())
	.expect("Valid composite primary key");

	let schema_editor = MockSchemaEditor;
	let forward_sql = create.database_forwards(&schema_editor);

	assert_eq!(forward_sql.len(), 1);
	assert!(forward_sql[0].contains("CREATE TABLE"));
	assert!(
		forward_sql[0].contains("PRIMARY KEY (product_id, category_id)")
			|| forward_sql[0].contains("PRIMARY KEY (\"product_id\", \"category_id\")")
	);

	// Backward migration: DROP TABLE
	let delete = DeleteModel::new(table_name);
	let backward_sql = delete.database_forwards(&schema_editor);

	assert_eq!(backward_sql.len(), 1);
	assert!(backward_sql[0].contains("DROP TABLE"));
	assert!(
		backward_sql[0].contains(table_name)
			|| backward_sql[0].contains(&format!("\"{}\"", table_name))
	);

	// Verify forward and backward SQL reference the same table
	println!("Forward SQL:\n{}", forward_sql[0]);
	println!("\nBackward SQL:\n{}", backward_sql[0]);

	let forward_has_table = forward_sql[0].contains(table_name)
		|| forward_sql[0].contains(&format!("\"{}\"", table_name));
	let backward_has_table = backward_sql[0].contains(table_name)
		|| backward_sql[0].contains(&format!("\"{}\"", table_name));

	assert!(
		forward_has_table && backward_has_table,
		"Both forward and backward SQL should reference table '{}'",
		table_name
	);
}
