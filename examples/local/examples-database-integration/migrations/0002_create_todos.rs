use reinhardt::db::migrations::{ColumnDefinition, Migration, Operation};

pub fn migration() -> Migration {
	Migration::new("0002_create_todos", "database_integration")
		.add_operation(Operation::CreateTable {
			name: "todos".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "SERIAL PRIMARY KEY"),
				ColumnDefinition::new("title", "VARCHAR(255) NOT NULL"),
				ColumnDefinition::new("description", "TEXT"),
				ColumnDefinition::new("completed", "BOOLEAN NOT NULL DEFAULT FALSE"),
				ColumnDefinition::new("created_at", "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP"),
				ColumnDefinition::new("updated_at", "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP"),
			],
			constraints: vec![],
		})
		.add_operation(Operation::CreateIndex {
			table: "todos".to_string(),
			columns: vec!["completed".to_string()],
			unique: false,
		})
		.add_operation(Operation::CreateIndex {
			table: "todos".to_string(),
			columns: vec!["created_at".to_string()],
			unique: false,
		})
}
