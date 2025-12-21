//! Create constraint test tables
//!
//! Creates test_posts and test_comments tables for testing
//! composite UNIQUE constraints and CASCADE DELETE behavior.

use reinhardt_migrations::{
	ColumnDefinition, Constraint, FieldType, ForeignKeyAction, Migration, Operation,
};

/// Create constraint test tables migration
///
/// Creates the following tables:
/// - test_posts: Posts with composite UNIQUE constraint on (user_id, title)
/// - test_comments: Comments with CASCADE DELETE on post_id
pub fn migration() -> Migration {
	Migration::new("0002_create_constraint_test_tables", "tests")
		// Must depend on 0001 because test_posts references test_users
		.add_dependency("tests", "0001_create_test_tables")
		// test_posts table
		.add_operation(Operation::CreateTable {
			name: "test_posts",
			columns: vec![
				ColumnDefinition::new(
					"id",
					FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
				),
				ColumnDefinition::new("user_id", FieldType::Integer),
				ColumnDefinition::new("title", FieldType::VarChar(200)),
				ColumnDefinition::new("content", FieldType::Text),
				ColumnDefinition::new(
					"created_at",
					FieldType::Custom("TIMESTAMP DEFAULT CURRENT_TIMESTAMP".to_string()),
				),
			],
			constraints: vec![
				Constraint::ForeignKey {
					name: "test_posts_user_id_fkey".to_string(),
					columns: vec!["user_id".to_string()],
					referenced_table: "test_users".to_string(),
					referenced_columns: vec!["id".to_string()],
					on_delete: ForeignKeyAction::NoAction,
					on_update: ForeignKeyAction::NoAction,
				},
				Constraint::Unique {
					name: "test_posts_user_title_unique".to_string(),
					columns: vec!["user_id".to_string(), "title".to_string()],
				},
			],
		})
		// test_comments table
		.add_operation(Operation::CreateTable {
			name: "test_comments",
			columns: vec![
				ColumnDefinition::new(
					"id",
					FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
				),
				ColumnDefinition::new("post_id", FieldType::Integer),
				ColumnDefinition::new("user_id", FieldType::Integer),
				ColumnDefinition::new("content", FieldType::Text),
				ColumnDefinition::new(
					"created_at",
					FieldType::Custom("TIMESTAMP DEFAULT CURRENT_TIMESTAMP".to_string()),
				),
			],
			constraints: vec![
				Constraint::ForeignKey {
					name: "test_comments_post_id_fkey".to_string(),
					columns: vec!["post_id".to_string()],
					referenced_table: "test_posts".to_string(),
					referenced_columns: vec!["id".to_string()],
					on_delete: ForeignKeyAction::Cascade,
					on_update: ForeignKeyAction::NoAction,
				},
				Constraint::ForeignKey {
					name: "test_comments_user_id_fkey".to_string(),
					columns: vec!["user_id".to_string()],
					referenced_table: "test_users".to_string(),
					referenced_columns: vec!["id".to_string()],
					on_delete: ForeignKeyAction::NoAction,
					on_update: ForeignKeyAction::NoAction,
				},
			],
		})
}
