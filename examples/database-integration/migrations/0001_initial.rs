use reinhardt_migrations::{ColumnDefinition, Migration, Operation};

pub fn migration() -> Migration {
    Migration::new("0001_initial", "database_integration")
        .add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "SERIAL PRIMARY KEY"),
                ColumnDefinition::new("name", "VARCHAR(255) NOT NULL"),
                ColumnDefinition::new("email", "VARCHAR(255) NOT NULL UNIQUE"),
                ColumnDefinition::new("created_at", "TIMESTAMP DEFAULT CURRENT_TIMESTAMP"),
            ],
            constraints: vec![],
        })
        .add_operation(Operation::CreateIndex {
            table: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
        })
}
