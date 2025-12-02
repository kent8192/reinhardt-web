use reinhardt_migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "users",
		name: "auto_migration_20251202_060231",
		operations: vec![Operation::CreateTable {
			name: "users",
			columns: vec![
				ColumnDefinition {
					name: "id",
					type_definition: BigIntegerField,
					not_null: true,
					unique: false,
					primary_key: true,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "email",
					type_definition: CharField,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "created_at",
					type_definition: DateTimeField,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "name",
					type_definition: CharField,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
			],
			constraints: vec![],
		}],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
	}
}
