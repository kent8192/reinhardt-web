//! Example usage of DCL statement builders
//!
//! This example demonstrates how to use the DCL (Data Control Language) statement builders
//! to create users, roles, and manage permissions in a database-agnostic way.

use std::sync::Arc;

use reinhardt_db::backends::{
	AlterRoleStatement, AlterUserStatement, CreateRoleStatement, CreateUserStatement,
	DatabaseBackend, RenameUserStatement, SetDefaultRoleStatement, SetRoleStatement,
};

/// Example: Create a new user with validation
fn create_user_example(backend: Arc<dyn DatabaseBackend>) {
	// Valid user creation
	let stmt = CreateUserStatement::new(backend.clone())
		.user("admin")
		.expect("Valid user name");
	let sql = stmt.build();
	println!("Create user SQL: {}", sql);
	// PostgreSQL: CREATE USER "admin"
	// MySQL: CREATE USER 'admin'@'%'

	// Whitespace is automatically trimmed
	let stmt = CreateUserStatement::new(backend.clone())
		.user("  john_doe  ")
		.expect("Valid user name");
	let sql = stmt.build();
	println!("Create user (trimmed) SQL: {}", sql);
	// User name becomes "john_doe" (trimmed)

	// Empty strings are rejected
	match CreateUserStatement::new(backend.clone()).user("") {
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Empty user rejected: {}", e),
		// Output: "User name cannot be empty or whitespace"
	}

	// Whitespace-only strings are rejected
	match CreateUserStatement::new(backend).user("   ") {
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Whitespace-only user rejected: {}", e),
		// Output: "User name cannot be empty or whitespace"
	}
}

/// Example: Create and manage roles
fn role_management_example(backend: Arc<dyn DatabaseBackend>) {
	// Create a role
	let stmt = CreateRoleStatement::new(backend.clone())
		.role("developer")
		.expect("Valid role name");
	let sql = stmt.build();
	println!("Create role SQL: {}", sql);
	// PostgreSQL: CREATE ROLE "developer"
	// MySQL: CREATE ROLE 'developer'

	// Alter a role
	let stmt = AlterRoleStatement::new(backend.clone())
		.role("developer")
		.expect("Valid role name");
	let sql = stmt.build();
	println!("Alter role SQL: {}", sql);

	// Set role for session
	let stmt = SetRoleStatement::new(backend)
		.role("developer")
		.expect("Valid role name");
	let sql = stmt.build();
	println!("Set role SQL: {}", sql);
	// PostgreSQL: SET ROLE "developer"
	// MySQL: SET ROLE 'developer'
}

/// Example: Rename a user
fn rename_user_example(backend: Arc<dyn DatabaseBackend>) {
	// Rename user
	let stmt = RenameUserStatement::new(backend.clone())
		.rename("oldname", "newname")
		.expect("Valid names");
	let sql = stmt.build();
	println!("Rename user SQL: {}", sql);
	// PostgreSQL: ALTER USER "oldname" RENAME TO "newname"
	// MySQL: RENAME USER 'oldname'@'%' TO 'newname'@'%'

	// Both names are validated
	match RenameUserStatement::new(backend.clone()).rename("", "newname") {
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Empty old name rejected: {}", e),
	}

	match RenameUserStatement::new(backend).rename("oldname", "   ") {
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Whitespace-only new name rejected: {}", e),
	}
}

/// Example: Set default roles for a user
fn set_default_roles_example(backend: Arc<dyn DatabaseBackend>) {
	// Set multiple default roles
	let stmt = SetDefaultRoleStatement::new(backend.clone())
		.user("john_doe")
		.expect("Valid user name")
		.users(&["developer", "analyst"])
		.expect("Valid role names");
	let sql = stmt.build();
	println!("Set default roles SQL: {}", sql);
	// MySQL: SET DEFAULT ROLE 'developer', 'analyst' TO 'john_doe'@'%'

	// All role names are validated
	match SetDefaultRoleStatement::new(backend.clone())
		.user("john_doe")
		.expect("Valid user name")
		.users(&["role1", "", "role2"])
	{
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Empty role name rejected: {}", e),
	}

	// User name is validated
	match SetDefaultRoleStatement::new(backend).user("   ") {
		Ok(_) => println!("This should not happen!"),
		Err(e) => println!("Whitespace-only user rejected: {}", e),
	}
}

/// Example: Comprehensive user and role workflow
fn comprehensive_example(backend: Arc<dyn DatabaseBackend>) {
	println!("\n=== Comprehensive DCL Example ===\n");

	// Step 1: Create roles
	println!("1. Creating roles...");
	let roles = vec!["admin", "developer", "analyst", "viewer"];
	for role in roles {
		let stmt = CreateRoleStatement::new(backend.clone())
			.role(role)
			.expect("Valid role");
		println!("   {}", stmt.build());
	}

	// Step 2: Create users
	println!("\n2. Creating users...");
	let users = vec!["alice", "bob", "charlie"];
	for user in users {
		let stmt = CreateUserStatement::new(backend.clone())
			.user(user)
			.expect("Valid user");
		println!("   {}", stmt.build());
	}

	// Step 3: Set default roles for users
	println!("\n3. Assigning default roles...");
	let stmt = SetDefaultRoleStatement::new(backend.clone())
		.user("alice")
		.expect("Valid user")
		.users(&["admin", "developer"])
		.expect("Valid roles");
	println!("   Alice: {}", stmt.build());

	let stmt = SetDefaultRoleStatement::new(backend.clone())
		.user("bob")
		.expect("Valid user")
		.users(&["developer"])
		.expect("Valid roles");
	println!("   Bob: {}", stmt.build());

	// Step 4: Rename a user
	println!("\n4. Renaming user...");
	let stmt = RenameUserStatement::new(backend.clone())
		.rename("charlie", "charles")
		.expect("Valid names");
	println!("   {}", stmt.build());

	// Step 5: Alter a user
	println!("\n5. Altering user...");
	let stmt = AlterUserStatement::new(backend)
		.user("alice")
		.expect("Valid user");
	println!("   {}", stmt.build());
}

fn main() {
	println!("=== DCL Statement Builders Examples ===\n");
	println!("Note: This example demonstrates the API. In a real application,");
	println!("you would execute these statements against an actual database.\n");

	// For demonstration, we'll use a mock backend
	// In a real application, you would use PostgresBackend, MySqlBackend, etc.
	println!("See individual example functions for usage patterns:");
	println!("- create_user_example()");
	println!("- role_management_example()");
	println!("- rename_user_example()");
	println!("- set_default_roles_example()");
	println!("- comprehensive_example()");
	println!("\nAll builders validate input and reject empty/whitespace strings.");
}
