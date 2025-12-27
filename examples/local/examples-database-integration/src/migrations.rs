//! Migrations for database integration example
//!
//! Provides migrations for users and todos tables using the MigrationProvider trait.

use reinhardt::db::migrations::{
	ColumnDefinition, FieldType, Migration, MigrationProvider, Operation,
};

/// Migration provider for the database integration example
///
/// Provides migrations for:
/// - users table (users app)
/// - todos table (todos app)
pub struct ExampleMigrations;

impl MigrationProvider for ExampleMigrations {
	fn migrations() -> Vec<Migration> {
		vec![
			Migration {
				app_label: "users",
				name: "0001_initial",
				operations: vec![Operation::CreateTable {
					name: "users",
					columns: vec![
						ColumnDefinition {
							name: "id",
							type_definition: FieldType::BigInteger,
							not_null: true,
							unique: false,
							primary_key: true,
							auto_increment: true,
							default: None,
						},
						ColumnDefinition {
							name: "name",
							type_definition: FieldType::VarChar(255),
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
						ColumnDefinition {
							name: "email",
							type_definition: FieldType::VarChar(255),
							not_null: true,
							unique: true,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
						ColumnDefinition {
							name: "created_at",
							type_definition: FieldType::TimestampTz,
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
					],
					constraints: vec![],
				}],
				dependencies: vec![],
				replaces: vec![],
				atomic: true,
				initial: Some(true),
				state_only: false,
				database_only: false,
			},
			Migration {
				app_label: "todos",
				name: "0001_initial",
				operations: vec![Operation::CreateTable {
					name: "todos",
					columns: vec![
						ColumnDefinition {
							name: "id",
							type_definition: FieldType::BigInteger,
							not_null: true,
							unique: false,
							primary_key: true,
							auto_increment: true,
							default: None,
						},
						ColumnDefinition {
							name: "title",
							type_definition: FieldType::VarChar(255),
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
						ColumnDefinition {
							name: "description",
							type_definition: FieldType::Text,
							not_null: false,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
						ColumnDefinition {
							name: "completed",
							type_definition: FieldType::Boolean,
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: Some("false"),
						},
						ColumnDefinition {
							name: "created_at",
							type_definition: FieldType::TimestampTz,
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
						ColumnDefinition {
							name: "updated_at",
							type_definition: FieldType::TimestampTz,
							not_null: true,
							unique: false,
							primary_key: false,
							auto_increment: false,
							default: None,
						},
					],
					constraints: vec![],
				}],
				dependencies: vec![],
				replaces: vec![],
				atomic: true,
				initial: Some(true),
				state_only: false,
				database_only: false,
			},
		]
	}
}
