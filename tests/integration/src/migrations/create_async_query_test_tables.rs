//! Create async query test tables
//!
//! Creates test_models table for async query integration tests.

use reinhardt_migrations::{ColumnDefinition, FieldType, Migration, Operation};

/// Create async query test tables migration
///
/// Creates the following table:
/// - test_models: Simple model table for async query tests
pub fn migration() -> Migration {
	Migration::new("0003_create_async_query_test_tables", "tests").add_operation(
		Operation::CreateTable {
			name: "test_models",
			columns: vec![
				ColumnDefinition::new("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
				ColumnDefinition::new("name", FieldType::Text),
			],
			constraints: vec![],
		},
	)
}
