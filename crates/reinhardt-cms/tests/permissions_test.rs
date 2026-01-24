//! Tests for permission checker

use reinhardt_cms::permissions::{PermissionChecker, PermissionType, Principal};
use rstest::rstest;
use uuid::Uuid;

#[rstest]
#[tokio::test]
async fn test_grant_and_check_permission() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Grant permission
	let permission = checker
		.grant_permission(
			page_id,
			Principal::User(user_id),
			PermissionType::Edit,
			false,
		)
		.await
		.unwrap();

	assert_eq!(permission.page_id, page_id);
	assert_eq!(permission.permission, PermissionType::Edit);

	// Check permission
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::Edit)
		.await
		.unwrap();
	assert!(has_permission);

	// Check non-existent permission
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::Delete)
		.await
		.unwrap();
	assert!(!has_permission);
}

#[rstest]
#[tokio::test]
async fn test_revoke_permission() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Grant permission
	let permission = checker
		.grant_permission(
			page_id,
			Principal::User(user_id),
			PermissionType::Edit,
			false,
		)
		.await
		.unwrap();

	// Verify permission exists
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::Edit)
		.await
		.unwrap();
	assert!(has_permission);

	// Revoke permission
	checker.revoke_permission(permission.id).await.unwrap();

	// Verify permission is revoked
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::Edit)
		.await
		.unwrap();
	assert!(!has_permission);
}

#[rstest]
#[tokio::test]
async fn test_authenticated_principal() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Grant permission to all authenticated users
	checker
		.grant_permission(
			page_id,
			Principal::Authenticated,
			PermissionType::View,
			false,
		)
		.await
		.unwrap();

	// Any authenticated user should have the permission
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::View)
		.await
		.unwrap();
	assert!(has_permission);
}

#[rstest]
#[tokio::test]
async fn test_anyone_principal() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Grant permission to anyone
	checker
		.grant_permission(page_id, Principal::Anyone, PermissionType::View, false)
		.await
		.unwrap();

	// Anyone should have the permission
	let has_permission = checker
		.check_permission(user_id, page_id, PermissionType::View)
		.await
		.unwrap();
	assert!(has_permission);
}

#[rstest]
#[tokio::test]
async fn test_get_page_permissions() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id1 = Uuid::new_v4();
	let user_id2 = Uuid::new_v4();

	// Grant multiple permissions
	checker
		.grant_permission(
			page_id,
			Principal::User(user_id1),
			PermissionType::Edit,
			false,
		)
		.await
		.unwrap();

	checker
		.grant_permission(
			page_id,
			Principal::User(user_id2),
			PermissionType::View,
			false,
		)
		.await
		.unwrap();

	// Get all permissions for the page
	let permissions = checker.get_page_permissions(page_id).await.unwrap();
	assert_eq!(permissions.len(), 2);
}

#[rstest]
#[tokio::test]
async fn test_multiple_permission_types() {
	let mut checker = PermissionChecker::new();

	let page_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	// Grant multiple permission types to the same user
	checker
		.grant_permission(
			page_id,
			Principal::User(user_id),
			PermissionType::View,
			false,
		)
		.await
		.unwrap();

	checker
		.grant_permission(
			page_id,
			Principal::User(user_id),
			PermissionType::Edit,
			false,
		)
		.await
		.unwrap();

	// Check both permissions
	assert!(
		checker
			.check_permission(user_id, page_id, PermissionType::View)
			.await
			.unwrap()
	);
	assert!(
		checker
			.check_permission(user_id, page_id, PermissionType::Edit)
			.await
			.unwrap()
	);

	// Check non-granted permission
	assert!(
		!checker
			.check_permission(user_id, page_id, PermissionType::Delete)
			.await
			.unwrap()
	);
}
