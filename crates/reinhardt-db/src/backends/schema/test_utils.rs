//! Test utilities for schema editor testing
//!
//! This module provides mock implementations for testing migration operations
//! without requiring actual database connections.

use crate::backends::SchemaEditorResult;
use crate::backends::schema::BaseDatabaseSchemaEditor;
use crate::backends::types::DatabaseType;
use async_trait::async_trait;

/// Mock schema editor for testing
///
/// A minimal mock implementation of `BaseDatabaseSchemaEditor` that doesn't
/// execute actual SQL but allows testing of schema modification logic.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_db::backends::schema::test_utils::MockSchemaEditor;
///
/// let editor = MockSchemaEditor::new();
/// let stmt = editor.create_table_statement("users", &[
///     ("id", "INTEGER PRIMARY KEY"),
///     ("name", "VARCHAR(100)"),
/// ]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MockSchemaEditor;

impl MockSchemaEditor {
	/// Create a new mock schema editor
	pub fn new() -> Self {
		Self
	}
}

impl Default for MockSchemaEditor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl BaseDatabaseSchemaEditor for MockSchemaEditor {
	fn database_type(&self) -> DatabaseType {
		DatabaseType::Sqlite
	}

	async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
		// Mock implementation - doesn't execute anything
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mock_schema_editor_creation() {
		let editor = MockSchemaEditor::new();
		assert_eq!(editor.database_type(), DatabaseType::Sqlite);
	}

	#[test]
	fn test_mock_schema_editor_default() {
		let editor = MockSchemaEditor::default();
		assert_eq!(editor.database_type(), DatabaseType::Sqlite);
	}
}
