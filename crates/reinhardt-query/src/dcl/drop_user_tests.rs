//! # DropUserStatement Unit Tests
//!
//! Comprehensive unit tests for DropUserStatement covering:
//! - Happy Path: Normal operations
//! - Error Path: Validation and error handling
//! - Edge Cases: Boundary values and special cases
//!
//! ## Test Coverage
//!
//! - Statements tested: DropUserStatement
//! - Code coverage target: 96%
//! - Total tests: ~20

use crate::backend::{MySqlQueryBuilder, PostgresQueryBuilder, QueryBuilder};
use crate::dcl::DropUserStatement;
use rstest::rstest;

// ============================================================================
// Happy Path Tests
// ============================================================================

#[rstest]
fn test_drop_user_new() {
	let stmt = DropUserStatement::new();

	assert!(stmt.user_names.is_empty());
	assert!(!stmt.if_exists);
}

#[rstest]
#[case("test_user")]
#[case("user_with_underscores")]
#[case("UserWithCamelCase")]
fn test_drop_single_user(#[case] user_name: &str) {
	let stmt = DropUserStatement::new().user(user_name);

	assert_eq!(stmt.user_names.len(), 1);
	assert_eq!(stmt.user_names[0], user_name);
}

#[rstest]
fn test_drop_multiple_users() {
	let stmt = DropUserStatement::new()
		.user("user1")
		.user("user2")
		.user("user3");

	assert_eq!(stmt.user_names.len(), 3);
}

#[rstest]
fn test_drop_user_with_if_exists() {
	let stmt = DropUserStatement::new().user("test_user").if_exists(true);

	assert!(stmt.if_exists);
	assert_eq!(stmt.user_names.len(), 1);
}

#[rstest]
fn test_drop_user_with_users_method() {
	let users = vec!["user1".to_string(), "user2".to_string()];
	let stmt = DropUserStatement::new().users(users.clone());

	assert_eq!(stmt.user_names, users);
}

#[rstest]
fn test_drop_user_comprehensive() {
	let stmt = DropUserStatement::new()
		.users(vec!["user1".to_string(), "user2".to_string()])
		.if_exists(true);

	assert_eq!(stmt.user_names.len(), 2);
	assert!(stmt.if_exists);
}

#[rstest]
fn test_drop_user_at_host() {
	let stmt = DropUserStatement::new().user("app_user@localhost");

	assert_eq!(stmt.user_names[0], "app_user@localhost");
}

#[rstest]
fn test_drop_multiple_users_at_different_hosts() {
	let stmt = DropUserStatement::new()
		.user("user1@localhost")
		.user("user2@'192.168.1.1'")
		.user("user3@'example.com'");

	assert_eq!(stmt.user_names.len(), 3);
}

#[rstest]
fn test_clone_statement() {
	let stmt1 = DropUserStatement::new().user("test_user");
	let stmt2 = stmt1.clone();

	assert_eq!(stmt1.user_names, stmt2.user_names);
	assert_eq!(stmt1.if_exists, stmt2.if_exists);
}

// ============================================================================
// Error Path Tests
// ============================================================================

#[rstest]
fn test_empty_user_list_validation() {
	let stmt = DropUserStatement::new();

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of empty user name validation in DropUserStatement::validate()"]
fn test_empty_user_in_list() {
	let stmt = DropUserStatement::new().user("");

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of whitespace validation in DropUserStatement::validate()"]
fn test_whitespace_only_user() {
	let stmt = DropUserStatement::new().user("   ");

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of empty user name validation in DropUserStatement::validate()"]
fn test_mixed_empty_and_valid_users() {
	let stmt = DropUserStatement::new().users(vec!["user1".to_string(), "".to_string()]);

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
fn test_valid_user_validation() {
	let stmt = DropUserStatement::new().user("valid_user");

	let result = stmt.validate();
	assert!(result.is_ok());
}

#[rstest]
fn test_multiple_valid_users_validation() {
	let stmt = DropUserStatement::new().users(vec![
		"user1".to_string(),
		"user2".to_string(),
		"user3".to_string(),
	]);

	let result = stmt.validate();
	assert!(result.is_ok());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[rstest]
#[case(1, "single char")]
#[case(63, "boundary before limit")]
#[case(64, "boundary at limit")]
#[case(255, "long name")]
fn test_user_name_length(#[case] length: usize, #[case] _desc: &str) {
	let user_name = "a".repeat(length);
	let stmt = DropUserStatement::new().user(user_name.clone());

	assert_eq!(stmt.user_names[0], user_name);
	assert_eq!(stmt.user_names[0].len(), length);
}

#[rstest]
fn test_very_long_user_name() {
	let user_name = "a".repeat(256);
	let stmt = DropUserStatement::new().user(user_name);

	assert_eq!(stmt.user_names[0].len(), 256);
}

#[rstest]
fn test_unicode_user_name() {
	let user_name = "ユーザー名";
	let stmt = DropUserStatement::new().user(user_name);

	assert_eq!(stmt.user_names[0], user_name);
}

#[rstest]
fn test_user_with_special_characters() {
	let user_name = "test-user_123";
	let stmt = DropUserStatement::new().user(user_name);

	assert_eq!(stmt.user_names[0], user_name);
}

#[rstest]
fn test_drop_many_users() {
	let users: Vec<String> = (0..100).map(|i| format!("user_{}", i)).collect();
	let stmt = DropUserStatement::new().users(users.clone());

	assert_eq!(stmt.user_names.len(), 100);
}

// ============================================================================
// SQL Generation Tests (PostgreSQL)
// ============================================================================

#[rstest]
fn test_postgres_drop_single_user() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropUserStatement::new().user("test_user");

	let (sql, values) = builder.build_drop_user(&stmt);

	// PostgreSQL DROP USER is DROP ROLE
	assert_eq!(sql, r#"DROP ROLE "test_user""#);
	assert!(values.is_empty());
}

#[rstest]
fn test_postgres_drop_multiple_users() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropUserStatement::new()
		.user("user1")
		.user("user2")
		.user("user3");

	let (sql, values) = builder.build_drop_user(&stmt);

	// PostgreSQL DROP USER is DROP ROLE
	assert!(sql.contains(r#"DROP ROLE"#));
	assert!(sql.contains(r#""user1""#));
	assert!(sql.contains(r#""user2""#));
	assert!(sql.contains(r#""user3""#));
	assert!(values.is_empty());
}

#[rstest]
fn test_postgres_drop_user_if_exists() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropUserStatement::new().user("test_user").if_exists(true);

	let (sql, values) = builder.build_drop_user(&stmt);

	// PostgreSQL DROP USER is DROP ROLE
	assert!(sql.contains(r#"DROP ROLE IF EXISTS "test_user""#));
	assert!(values.is_empty());
}

// ============================================================================
// SQL Generation Tests (MySQL)
// ============================================================================

#[rstest]
#[ignore = "Requires implementation of user@host syntax in MySQL DROP USER backend"]
fn test_mysql_drop_single_user() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropUserStatement::new().user("test_user");

	let (sql, values) = builder.build_drop_user(&stmt);

	assert_eq!(sql, r#"DROP USER 'test_user'@"#);
	assert!(values.is_empty());
}

#[rstest]
#[ignore = "Requires implementation of proper user@host parsing in MySQL DROP USER backend"]
fn test_mysql_drop_user_at_host() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropUserStatement::new().user("app_user@localhost");

	let (sql, values) = builder.build_drop_user(&stmt);

	assert!(sql.contains(r#"DROP USER 'app_user'@'localhost'"#));
	assert!(values.is_empty());
}

#[rstest]
#[ignore = "Requires implementation of proper user@host parsing in MySQL DROP USER backend"]
fn test_mysql_drop_multiple_users() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropUserStatement::new()
		.user("user1@localhost")
		.user("user2@'192.168.1.1'")
		.user("user3@'example.com'");

	let (sql, values) = builder.build_drop_user(&stmt);

	assert!(sql.contains(r#"DROP USER"#));
	assert!(sql.contains("'user1'@'localhost'"));
	assert!(sql.contains("'user2'@'192.168.1.1'"));
	assert!(sql.contains("'user3'@'example.com'"));
	assert!(values.is_empty());
}

#[rstest]
fn test_mysql_drop_user_if_exists() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropUserStatement::new().user("test_user").if_exists(true);

	let (sql, values) = builder.build_drop_user(&stmt);

	assert!(sql.contains(r#"DROP USER IF EXISTS"#));
	assert!(values.is_empty());
}

#[rstest]
#[ignore = "Requires implementation of proper user@host parsing in MySQL DROP USER backend"]
fn test_mysql_drop_user_if_exists_at_host() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropUserStatement::new()
		.user("app_user@localhost")
		.if_exists(true);

	let (sql, values) = builder.build_drop_user(&stmt);

	assert!(sql.contains(r#"DROP USER IF EXISTS 'app_user'@'localhost'"#));
	assert!(values.is_empty());
}
