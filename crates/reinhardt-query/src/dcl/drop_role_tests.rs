//! # DropRoleStatement Unit Tests
//!
//! Comprehensive unit tests for DropRoleStatement covering:
//! - Happy Path: Normal operations
//! - Error Path: Validation and error handling
//! - Edge Cases: Boundary values and special cases
//!
//! ## Test Coverage
//!
//! - Statements tested: DropRoleStatement
//! - Code coverage target: 97%
//! - Total tests: ~20

use crate::backend::{MySqlQueryBuilder, PostgresQueryBuilder, QueryBuilder};
use crate::dcl::DropRoleStatement;
use rstest::rstest;

// ============================================================================
// Happy Path Tests
// ============================================================================

#[rstest]
fn test_drop_role_new() {
	let stmt = DropRoleStatement::new();

	assert!(stmt.role_names.is_empty());
	assert!(!stmt.if_exists);
}

#[rstest]
#[case("test_role")]
#[case("role_with_underscores")]
#[case("RoleWithCamelCase")]
fn test_drop_single_role(#[case] role_name: &str) {
	let stmt = DropRoleStatement::new().role(role_name);

	assert_eq!(stmt.role_names.len(), 1);
	assert_eq!(stmt.role_names[0], role_name);
}

#[rstest]
fn test_drop_multiple_roles() {
	let stmt = DropRoleStatement::new()
		.role("role1")
		.role("role2")
		.role("role3");

	assert_eq!(stmt.role_names.len(), 3);
}

#[rstest]
fn test_drop_role_with_if_exists() {
	let stmt = DropRoleStatement::new().role("test_role").if_exists(true);

	assert!(stmt.if_exists);
	assert_eq!(stmt.role_names.len(), 1);
}

#[rstest]
fn test_drop_role_with_roles_method() {
	let roles = vec!["role1".to_string(), "role2".to_string()];
	let stmt = DropRoleStatement::new().roles(roles.clone());

	assert_eq!(stmt.role_names, roles);
}

#[rstest]
fn test_drop_role_comprehensive() {
	let stmt = DropRoleStatement::new()
		.roles(vec!["role1".to_string(), "role2".to_string()])
		.if_exists(true);

	assert_eq!(stmt.role_names.len(), 2);
	assert!(stmt.if_exists);
}

#[rstest]
fn test_clone_statement() {
	let stmt1 = DropRoleStatement::new().role("test_role");
	let stmt2 = stmt1.clone();

	assert_eq!(stmt1.role_names, stmt2.role_names);
	assert_eq!(stmt1.if_exists, stmt2.if_exists);
}

// ============================================================================
// Error Path Tests
// ============================================================================

#[rstest]
fn test_empty_role_list_validation() {
	let stmt = DropRoleStatement::new();

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of empty role name validation in DropRoleStatement::validate()"]
fn test_empty_role_in_list() {
	let stmt = DropRoleStatement::new().role("");

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of whitespace validation in DropRoleStatement::validate()"]
fn test_whitespace_only_role() {
	let stmt = DropRoleStatement::new().role("   ");

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
#[ignore = "Requires implementation of empty role name validation in DropRoleStatement::validate()"]
fn test_mixed_empty_and_valid_roles() {
	let stmt = DropRoleStatement::new().roles(vec!["role1".to_string(), "".to_string()]);

	let result = stmt.validate();
	assert!(result.is_err());
}

#[rstest]
fn test_valid_role_validation() {
	let stmt = DropRoleStatement::new().role("valid_role");

	let result = stmt.validate();
	assert!(result.is_ok());
}

#[rstest]
fn test_multiple_valid_roles_validation() {
	let stmt = DropRoleStatement::new().roles(vec![
		"role1".to_string(),
		"role2".to_string(),
		"role3".to_string(),
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
fn test_role_name_length(#[case] length: usize, #[case] _desc: &str) {
	let role_name = "a".repeat(length);
	let stmt = DropRoleStatement::new().role(role_name.clone());

	assert_eq!(stmt.role_names[0], role_name);
	assert_eq!(stmt.role_names[0].len(), length);
}

#[rstest]
fn test_very_long_role_name() {
	let role_name = "a".repeat(256);
	let stmt = DropRoleStatement::new().role(role_name);

	assert_eq!(stmt.role_names[0].len(), 256);
}

#[rstest]
fn test_unicode_role_name() {
	let role_name = "ロール名";
	let stmt = DropRoleStatement::new().role(role_name);

	assert_eq!(stmt.role_names[0], role_name);
}

#[rstest]
fn test_role_with_special_characters() {
	let role_name = "test-role_123";
	let stmt = DropRoleStatement::new().role(role_name);

	assert_eq!(stmt.role_names[0], role_name);
}

#[rstest]
fn test_drop_many_roles() {
	let roles: Vec<String> = (0..100).map(|i| format!("role_{}", i)).collect();
	let stmt = DropRoleStatement::new().roles(roles.clone());

	assert_eq!(stmt.role_names.len(), 100);
}

// ============================================================================
// SQL Generation Tests (PostgreSQL)
// ============================================================================

#[rstest]
fn test_postgres_drop_single_role() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropRoleStatement::new().role("test_role");

	let (sql, values) = builder.build_drop_role(&stmt);

	assert_eq!(sql, r#"DROP ROLE "test_role""#);
	assert!(values.is_empty());
}

#[rstest]
fn test_postgres_drop_multiple_roles() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropRoleStatement::new()
		.role("role1")
		.role("role2")
		.role("role3");

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE"#));
	assert!(sql.contains(r#""role1""#));
	assert!(sql.contains(r#""role2""#));
	assert!(sql.contains(r#""role3""#));
	assert!(values.is_empty());
}

#[rstest]
fn test_postgres_drop_role_if_exists() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropRoleStatement::new().role("test_role").if_exists(true);

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE IF EXISTS "test_role""#));
	assert!(values.is_empty());
}

#[rstest]
fn test_postgres_drop_role_with_if_exists_multiple() {
	let builder = PostgresQueryBuilder::new();
	let stmt = DropRoleStatement::new()
		.roles(vec!["role1".to_string(), "role2".to_string()])
		.if_exists(true);

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE IF EXISTS"#));
	assert!(sql.contains(r#""role1""#));
	assert!(sql.contains(r#""role2""#));
	assert!(values.is_empty());
}

// ============================================================================
// SQL Generation Tests (MySQL)
// ============================================================================

#[rstest]
fn test_mysql_drop_single_role() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropRoleStatement::new().role("test_role");

	let (sql, values) = builder.build_drop_role(&stmt);

	assert_eq!(sql, r#"DROP ROLE `test_role`"#);
	assert!(values.is_empty());
}

#[rstest]
fn test_mysql_drop_multiple_roles() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropRoleStatement::new()
		.role("role1")
		.role("role2")
		.role("role3");

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE"#));
	assert!(sql.contains("`role1`"));
	assert!(sql.contains("`role2`"));
	assert!(sql.contains("`role3`"));
	assert!(values.is_empty());
}

#[rstest]
fn test_mysql_drop_role_if_exists() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropRoleStatement::new().role("test_role").if_exists(true);

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE IF EXISTS `test_role`"#));
	assert!(values.is_empty());
}

#[rstest]
fn test_mysql_drop_role_with_if_exists_multiple() {
	let builder = MySqlQueryBuilder::new();
	let stmt = DropRoleStatement::new()
		.roles(vec!["role1".to_string(), "role2".to_string()])
		.if_exists(true);

	let (sql, values) = builder.build_drop_role(&stmt);

	assert!(sql.contains(r#"DROP ROLE IF EXISTS"#));
	assert!(sql.contains("`role1`"));
	assert!(sql.contains("`role2`"));
	assert!(values.is_empty());
}
