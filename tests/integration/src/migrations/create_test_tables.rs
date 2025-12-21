//! Create test tables for validator tests
//!
//! Creates test_users, test_products, and test_orders tables.

use reinhardt_migrations::{
	ColumnDefinition, Constraint, FieldType, ForeignKeyAction, Migration, Operation,
};

/// Create test tables migration
///
/// Creates the following tables:
/// - test_users: User data with username and email
/// - test_products: Product data with code, price, and stock
/// - test_orders: Order data with references to users and products
pub fn migration() -> Migration {
	Migration::new("0001_create_test_tables", "tests")
		// test_users table
		.add_operation(Operation::CreateTable {
			name: "test_users",
			columns: vec![
				ColumnDefinition::new(
					"id",
					FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
				),
				ColumnDefinition::new("username", FieldType::VarChar(100)),
				ColumnDefinition::new("email", FieldType::VarChar(255)),
				ColumnDefinition::new(
					"created_at",
					FieldType::Custom("TIMESTAMP DEFAULT CURRENT_TIMESTAMP".to_string()),
				),
			],
			constraints: vec![
				Constraint::Unique {
					name: "test_users_username_unique".to_string(),
					columns: vec!["username".to_string()],
				},
				Constraint::Unique {
					name: "test_users_email_unique".to_string(),
					columns: vec!["email".to_string()],
				},
			],
		})
		// test_products table
		.add_operation(Operation::CreateTable {
			name: "test_products",
			columns: vec![
				ColumnDefinition::new(
					"id",
					FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
				),
				ColumnDefinition::new("name", FieldType::VarChar(200)),
				ColumnDefinition::new("code", FieldType::VarChar(50)),
				ColumnDefinition::new("price", FieldType::Decimal { precision: 10, scale: 2 }),
				ColumnDefinition::new("stock", FieldType::Integer),
				ColumnDefinition::new(
					"created_at",
					FieldType::Custom("TIMESTAMP DEFAULT CURRENT_TIMESTAMP".to_string()),
				),
			],
			constraints: vec![
				Constraint::Unique {
					name: "test_products_code_unique".to_string(),
					columns: vec!["code".to_string()],
				},
				Constraint::Check {
					name: "test_products_price_check".to_string(),
					expression: "price >= 0".to_string(),
				},
				Constraint::Check {
					name: "test_products_stock_check".to_string(),
					expression: "stock >= 0".to_string(),
				},
			],
		})
		// test_orders table
		.add_operation(Operation::CreateTable {
			name: "test_orders",
			columns: vec![
				ColumnDefinition::new(
					"id",
					FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
				),
				ColumnDefinition::new("user_id", FieldType::Integer),
				ColumnDefinition::new("product_id", FieldType::Integer),
				ColumnDefinition::new("quantity", FieldType::Integer),
				ColumnDefinition::new(
					"order_date",
					FieldType::Custom("TIMESTAMP DEFAULT CURRENT_TIMESTAMP".to_string()),
				),
			],
			constraints: vec![
				Constraint::ForeignKey {
					name: "test_orders_user_id_fkey".to_string(),
					columns: vec!["user_id".to_string()],
					referenced_table: "test_users".to_string(),
					referenced_columns: vec!["id".to_string()],
					on_delete: ForeignKeyAction::NoAction,
					on_update: ForeignKeyAction::NoAction,
				},
				Constraint::ForeignKey {
					name: "test_orders_product_id_fkey".to_string(),
					columns: vec!["product_id".to_string()],
					referenced_table: "test_products".to_string(),
					referenced_columns: vec!["id".to_string()],
					on_delete: ForeignKeyAction::NoAction,
					on_update: ForeignKeyAction::NoAction,
				},
				Constraint::Unique {
					name: "test_orders_user_product_unique".to_string(),
					columns: vec!["user_id".to_string(), "product_id".to_string()],
				},
				Constraint::Check {
					name: "test_orders_quantity_check".to_string(),
					expression: "quantity > 0".to_string(),
				},
			],
		})
}
