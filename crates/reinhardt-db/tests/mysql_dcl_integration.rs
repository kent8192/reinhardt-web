//! Integration tests for MySQL DCL (Data Control Language) statements
//!
//! These tests demonstrate real-world usage of MySQL user management with
//! proper `'user'@'host'` syntax.

#[cfg(feature = "mysql")]
mod mysql_dcl_tests {
	use reinhardt_db::backends::{
		AlterUserStatement, CreateUserStatement, DefaultRoleSpec, DropUserStatement, MySqlUser,
		RenameUserStatement, SetDefaultRoleStatement,
	};

	#[test]
	fn test_create_user_scenarios() {
		// Scenario 1: Create a local-only user with password
		let stmt = CreateUserStatement::new("webapp@localhost").password("webapp_pass123");
		assert_eq!(
			stmt.build(),
			"CREATE USER 'webapp'@'localhost' IDENTIFIED BY 'webapp_pass123'"
		);

		// Scenario 2: Create a user accessible from any host
		let stmt = CreateUserStatement::new("api_service").password("api_secret");
		assert_eq!(
			stmt.build(),
			"CREATE USER 'api_service'@'%' IDENTIFIED BY 'api_secret'"
		);

		// Scenario 3: Create a user for specific IP range with IF NOT EXISTS
		let stmt = CreateUserStatement::new("db_admin@192.168.1.%")
			.if_not_exists()
			.password("admin123");
		assert_eq!(
			stmt.build(),
			"CREATE USER IF NOT EXISTS 'db_admin'@'192.168.1.%' IDENTIFIED BY 'admin123'"
		);

		// Scenario 4: Create a user from a specific domain
		let stmt = CreateUserStatement::new("remote@db.example.com").password("remote_pass");
		assert_eq!(
			stmt.build(),
			"CREATE USER 'remote'@'db.example.com' IDENTIFIED BY 'remote_pass'"
		);
	}

	#[test]
	fn test_alter_user_password_rotation() {
		// Scenario: Password rotation for security compliance
		let users = vec!["admin@localhost", "webapp@%", "api@192.168.1.100"];

		for user in users {
			let stmt = AlterUserStatement::new(user).password("new_rotated_password");
			let sql = stmt.build();
			assert!(sql.starts_with("ALTER USER"));
			assert!(sql.contains("IDENTIFIED BY 'new_rotated_password'"));
			// Verify user@host syntax is preserved
			assert!(sql.contains('@'));
			assert!(sql.contains('\''));
		}
	}

	#[test]
	fn test_drop_user_cleanup() {
		// Scenario 1: Drop a single user safely with IF EXISTS
		let stmt = DropUserStatement::new()
			.user("deprecated_user@localhost")
			.if_exists();
		assert_eq!(
			stmt.build(),
			"DROP USER IF EXISTS 'deprecated_user'@'localhost'"
		);

		// Scenario 2: Bulk user cleanup
		let stmt = DropUserStatement::new()
			.user("temp_user1@localhost")
			.user("temp_user2@%")
			.user("temp_user3@192.168.1.100")
			.if_exists();
		let sql = stmt.build();
		assert!(sql.contains("IF EXISTS"));
		assert!(sql.contains("'temp_user1'@'localhost'"));
		assert!(sql.contains("'temp_user2'@'%'"));
		assert!(sql.contains("'temp_user3'@'192.168.1.100'"));
	}

	#[test]
	fn test_rename_user_workflow() {
		// Scenario: Rename user after organizational change
		let stmt = RenameUserStatement::new("old_department@localhost", "new_department@localhost");
		assert_eq!(
			stmt.build(),
			"RENAME USER 'old_department'@'localhost' TO 'new_department'@'localhost'"
		);

		// Scenario: Change user host restriction
		let stmt = RenameUserStatement::new("webapp@localhost", "webapp@%");
		assert_eq!(
			stmt.build(),
			"RENAME USER 'webapp'@'localhost' TO 'webapp'@'%'"
		);
	}

	#[test]
	fn test_set_default_role_scenarios() {
		// Scenario 1: Grant all roles to an admin user
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("admin@localhost");
		assert_eq!(stmt.build(), "SET DEFAULT ROLE ALL TO 'admin'@'localhost'");

		// Scenario 2: Set specific roles for a service account
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::Roles(vec![
				"app_read".to_string(),
				"app_write".to_string(),
			]))
			.user("service_account@%");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE app_read, app_write TO 'service_account'@'%'"
		);

		// Scenario 3: Clear default roles for a restricted user
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::None)
			.user("restricted@localhost");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE NONE TO 'restricted'@'localhost'"
		);

		// Scenario 4: Set roles for multiple users at once
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("webapp@localhost")
			.user("api@localhost");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE ALL TO 'webapp'@'localhost', 'api'@'localhost'"
		);
	}

	#[test]
	fn test_mysql_user_parsing_edge_cases() {
		// Test various MySQL user identifier formats
		let test_cases = vec![
			("user", "'user'@'%'"), // No host -> default to %
			("user@localhost", "'user'@'localhost'"),
			("user@%", "'user'@'%'"),
			("user@192.168.1.1", "'user'@'192.168.1.1'"),
			("user@%.example.com", "'user'@'%.example.com'"),
			("user@2001:db8::1", "'user'@'2001:db8::1'"), // IPv6
			(
				"complex_user_123@host-name.example.com",
				"'complex_user_123'@'host-name.example.com'",
			),
		];

		for (input, expected) in test_cases {
			let user = MySqlUser::parse(input);
			assert_eq!(user.to_string(), expected, "Failed for input: {}", input);
		}
	}

	#[test]
	fn test_complete_user_lifecycle() {
		// Simulate a complete user lifecycle: create -> use -> modify -> cleanup

		// Step 1: Create the user
		let create_stmt = CreateUserStatement::new("lifecycle_test@localhost")
			.password("initial_pass")
			.if_not_exists();
		let create_sql = create_stmt.build();
		assert!(create_sql.contains("CREATE USER IF NOT EXISTS"));

		// Step 2: Set default roles
		let role_stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::Roles(vec!["app_user".to_string()]))
			.user("lifecycle_test@localhost");
		let role_sql = role_stmt.build();
		assert!(role_sql.contains("SET DEFAULT ROLE app_user"));

		// Step 3: Rotate password (security requirement)
		let alter_stmt =
			AlterUserStatement::new("lifecycle_test@localhost").password("rotated_pass");
		let alter_sql = alter_stmt.build();
		assert!(alter_sql.contains("ALTER USER"));

		// Step 4: Rename if needed
		let rename_stmt =
			RenameUserStatement::new("lifecycle_test@localhost", "renamed_test@localhost");
		let rename_sql = rename_stmt.build();
		assert!(rename_sql.contains("RENAME USER"));

		// Step 5: Clean up
		let drop_stmt = DropUserStatement::new()
			.user("renamed_test@localhost")
			.if_exists();
		let drop_sql = drop_stmt.build();
		assert!(drop_sql.contains("DROP USER IF EXISTS"));
	}

	#[test]
	fn test_security_focused_workflows() {
		// Scenario 1: Least privilege - create user with minimal access from specific host
		let stmt = CreateUserStatement::new("readonly@192.168.1.100").password("readonly_pass");
		assert!(stmt.build().contains("'readonly'@'192.168.1.100'"));

		// Scenario 2: Deny all default roles for security
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::None)
			.user("untrusted@%");
		assert!(stmt.build().contains("NONE"));

		// Scenario 3: Bulk cleanup of temporary users
		let mut drop_stmt = DropUserStatement::new().if_exists();
		for i in 1..=5 {
			drop_stmt = drop_stmt.user(format!("temp_user_{}@localhost", i));
		}
		let sql = drop_stmt.build();
		assert!(sql.contains("IF EXISTS"));
		// Verify all 5 users are in the statement
		for i in 1..=5 {
			assert!(sql.contains(&format!("'temp_user_{}'", i)));
		}
	}
}
