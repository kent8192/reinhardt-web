//! Error handling tests for composite primary key support in migrations

use reinhardt_db::backends::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
use reinhardt_db::migrations::FieldType;
use reinhardt_db::migrations::operations::FieldDefinition;
use reinhardt_db::migrations::operations::models::{CreateModel, ValidationError};
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
fn test_composite_pk_empty_list_error() {
	let result = CreateModel::new(
		"users",
		vec![
			FieldDefinition::new("id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("name", FieldType::VarChar(100), false, false, None::<&str>),
		],
	)
	.with_composite_primary_key(vec![]);

	assert!(
		result.is_err(),
		"Empty composite primary key should return error"
	);

	let err = result.unwrap_err();
	assert!(
		matches!(
			&err,
			ValidationError::EmptyCompositePrimaryKey { table_name } if table_name == "users"
		),
		"Expected EmptyCompositePrimaryKey error for table 'users', got: {:?}",
		err
	);

	assert_eq!(
		err.to_string(),
		"Composite primary key for table 'users' cannot be empty",
		"Error message should match expected format"
	);
}

#[rstest]
fn test_composite_pk_nonexistent_field_error() {
	let result = CreateModel::new(
		"posts",
		vec![
			FieldDefinition::new("id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("title", FieldType::VarChar(200), false, false, None::<&str>),
		],
	)
	.with_composite_primary_key(vec!["id".to_string(), "author_id".to_string()]);

	assert!(
		result.is_err(),
		"Non-existent field in composite primary key should return error"
	);

	let err = result.unwrap_err();
	match &err {
		ValidationError::NonExistentField {
			field_name,
			table_name,
			available_fields,
		} => {
			assert_eq!(field_name, "author_id", "Field name should be 'author_id'");
			assert_eq!(table_name, "posts", "Table name should be 'posts'");
			assert_eq!(
				available_fields,
				&vec!["id", "title"],
				"Available fields should be 'id' and 'title'"
			);
		}
		_ => panic!("Expected NonExistentField error, got: {:?}", err),
	}

	let err_msg = err.to_string();
	assert!(
		err_msg.contains("author_id"),
		"Error message should contain 'author_id', got: {}",
		err_msg
	);
	assert!(
		err_msg.contains("posts"),
		"Error message should contain 'posts', got: {}",
		err_msg
	);
	assert!(
		err_msg.contains("id, title"),
		"Error message should contain 'id, title', got: {}",
		err_msg
	);
}

#[rstest]
fn test_composite_pk_fields_not_null_constraint() {
	let create = CreateModel::new(
		"post_tags",
		vec![
			FieldDefinition::new("post_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new("tag_id", FieldType::Integer, true, false, None::<&str>),
			FieldDefinition::new(
				"description",
				FieldType::VarChar(200),
				false,
				true,
				None::<&str>,
			),
		],
	)
	.with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()])
	.expect("Valid composite primary key");

	let schema_editor = MockSchemaEditor;
	let sql_statements = create.database_forwards(&schema_editor);

	assert_eq!(sql_statements.len(), 1);
	let sql = &sql_statements[0];

	println!("Generated SQL:\n{}", sql);

	// Composite PK fields must have NOT NULL constraint
	assert!(
		sql.contains("post_id INTEGER NOT NULL") || sql.contains("\"post_id\" INTEGER NOT NULL"),
		"post_id should have NOT NULL constraint in SQL: {}",
		sql
	);
	assert!(
		sql.contains("tag_id INTEGER NOT NULL") || sql.contains("\"tag_id\" INTEGER NOT NULL"),
		"tag_id should have NOT NULL constraint in SQL: {}",
		sql
	);

	// Composite PK fields should not have individual PRIMARY KEY constraint
	assert!(
		!sql.contains("post_id INTEGER PRIMARY KEY"),
		"post_id should NOT have individual PRIMARY KEY in SQL: {}",
		sql
	);
	assert!(
		!sql.contains("tag_id INTEGER PRIMARY KEY"),
		"tag_id should NOT have individual PRIMARY KEY in SQL: {}",
		sql
	);

	// Should have table-level PRIMARY KEY constraint
	assert!(
		sql.contains("PRIMARY KEY (post_id, tag_id)")
			|| sql.contains("PRIMARY KEY (\"post_id\", \"tag_id\")"),
		"SQL should contain table-level PRIMARY KEY constraint: {}",
		sql
	);

	// Nullable fields should allow NULL
	assert!(
		sql.contains("description VARCHAR(200)") || sql.contains("\"description\" VARCHAR(200)"),
		"description should allow NULL in SQL: {}",
		sql
	);
	assert!(
		!sql.contains("description VARCHAR(200) NOT NULL"),
		"description should NOT have NOT NULL constraint in SQL: {}",
		sql
	);
}
