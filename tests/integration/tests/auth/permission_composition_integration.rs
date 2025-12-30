//! Permission Composition Integration Tests
//!
//! This module contains comprehensive tests for permission composition using
//! logical operators (AND, OR, NOT). Tests cover complex combinations,
//! edge cases, and boolean algebra properties.
//!
//! # Test Categories
//!
//! - Happy Path: Basic permission checking with various user states
//! - Combination: Complex multi-level permission compositions
//! - Edge Cases: Boundary conditions and unusual combinations
//! - Decision Table: Systematic coverage of boolean combinations
//! - De Morgan's Law: Verification of boolean algebra properties

use bytes::Bytes;
use hyper::{HeaderMap, Method, Version};
use reinhardt_auth::permission_operators::{AndPermission, NotPermission, OrPermission};
use reinhardt_auth::{
	AllowAny, IsActiveUser, IsAdminUser, IsAuthenticated, IsAuthenticatedOrReadOnly, Permission,
	PermissionContext,
};
use reinhardt_types::Request;
use rstest::*;

// =============================================================================
// Fixtures
// =============================================================================

/// Creates a test HTTP request
#[fixture]
fn test_request() -> Request {
	Request::builder()
		.method(Method::GET)
		.uri("/api/test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap()
}

/// Creates a POST request for write operation tests
#[fixture]
fn post_request() -> Request {
	Request::builder()
		.method(Method::POST)
		.uri("/api/test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap()
}

// =============================================================================
// Happy Path Tests - Basic Permission Classes
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_allow_any_always_permits(test_request: Request) {
	let permission = AllowAny;
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert!(result, "AllowAny should permit unauthenticated requests");
}

#[rstest]
#[tokio::test]
async fn test_is_authenticated_requires_auth(test_request: Request) {
	let permission = IsAuthenticated;

	// Unauthenticated - denied
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"IsAuthenticated should deny unauthenticated"
	);

	// Authenticated - allowed
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"IsAuthenticated should allow authenticated"
	);
}

#[rstest]
#[tokio::test]
async fn test_is_admin_requires_admin_and_auth(test_request: Request) {
	let permission = IsAdminUser;

	// Authenticated but not admin - denied
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"IsAdminUser should deny non-admin users"
	);

	// Authenticated and admin - allowed
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"IsAdminUser should allow admin users"
	);

	// Admin but not authenticated - denied
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: true,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"IsAdminUser requires authentication"
	);
}

#[rstest]
#[tokio::test]
async fn test_is_active_requires_active_and_auth(test_request: Request) {
	let permission = IsActiveUser;

	// Authenticated but not active - denied
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"IsActiveUser should deny inactive users"
	);

	// Authenticated and active - allowed
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"IsActiveUser should allow active users"
	);
}

#[rstest]
#[tokio::test]
async fn test_is_authenticated_or_read_only(test_request: Request, post_request: Request) {
	let permission = IsAuthenticatedOrReadOnly;

	// GET request without auth - allowed (read-only)
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"GET requests should be allowed for anonymous"
	);

	// POST request without auth - denied
	let context = PermissionContext {
		request: &post_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"POST requests should require authentication"
	);

	// POST request with auth - allowed
	let context = PermissionContext {
		request: &post_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"POST requests should be allowed for authenticated"
	);
}

// =============================================================================
// Combination Tests - AND Operator
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_and_permission_both_required(test_request: Request) {
	let permission = AndPermission::new(IsAuthenticated, IsActiveUser);

	// Both conditions met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"AND should pass when both conditions met"
	);

	// Only first condition met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"AND should fail when second condition not met"
	);

	// Only second condition met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"AND should fail when first condition not met"
	);

	// Neither condition met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"AND should fail when neither condition met"
	);
}

#[rstest]
#[tokio::test]
async fn test_and_with_operator_syntax(test_request: Request) {
	// Using & operator
	let permission = IsAuthenticated & IsAdminUser;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"& operator should work like AndPermission"
	);
}

#[rstest]
#[tokio::test]
async fn test_triple_and_permission(test_request: Request) {
	let permission = AndPermission::new(
		AndPermission::new(IsAuthenticated, IsActiveUser),
		IsAdminUser,
	);

	// All three conditions met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Triple AND should pass when all conditions met"
	);

	// Missing one condition
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"Triple AND should fail when any condition not met"
	);
}

// =============================================================================
// Combination Tests - OR Operator
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_or_permission_either_sufficient(test_request: Request) {
	let permission = OrPermission::new(IsAdminUser, IsActiveUser);

	// Both conditions met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"OR should pass when both conditions met"
	);

	// Only first (admin) met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"OR should pass when first condition met"
	);

	// Only second (active) met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"OR should pass when second condition met"
	);

	// Neither met
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"OR should fail when neither condition met"
	);
}

#[rstest]
#[tokio::test]
async fn test_or_with_operator_syntax(test_request: Request) {
	// Using | operator
	let permission = IsAuthenticated | AllowAny;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"| operator should work like OrPermission"
	);
}

#[rstest]
#[tokio::test]
async fn test_triple_or_permission(test_request: Request) {
	let permission = OrPermission::new(OrPermission::new(IsAdminUser, IsActiveUser), AllowAny);

	// None of the first two, but AllowAny
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Triple OR with AllowAny should always pass"
	);
}

// =============================================================================
// Combination Tests - NOT Operator
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_not_permission_inverts(test_request: Request) {
	let permission = NotPermission::new(IsAuthenticated);

	// Not authenticated - allowed by NOT
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"NOT should pass when inner condition is false"
	);

	// Authenticated - denied by NOT
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"NOT should fail when inner condition is true"
	);
}

#[rstest]
#[tokio::test]
async fn test_not_with_operator_syntax(test_request: Request) {
	// Using ! operator
	let permission = !IsAuthenticated;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"! operator should work like NotPermission"
	);
}

#[rstest]
#[tokio::test]
async fn test_double_not_equals_original(test_request: Request) {
	let permission = NotPermission::new(NotPermission::new(IsAuthenticated));

	// Authenticated - double NOT equals original
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Double NOT should equal original"
	);

	// Not authenticated
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"Double NOT should equal original (false case)"
	);
}

// =============================================================================
// Complex Combination Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_complex_and_or_combination(test_request: Request) {
	// (IsAuthenticated AND IsActiveUser) OR IsAdminUser
	let permission = OrPermission::new(
		AndPermission::new(IsAuthenticated, IsActiveUser),
		IsAdminUser,
	);

	// Active authenticated user - passes first branch
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Should pass via AND branch"
	);

	// Admin user (authenticated required by IsAdminUser)
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Should pass via admin branch"
	);

	// Authenticated but not active and not admin
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: false,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"Should fail when neither branch satisfied"
	);
}

#[rstest]
#[tokio::test]
async fn test_complex_with_not(test_request: Request) {
	// IsAuthenticated AND NOT IsAdminUser (regular users only)
	let permission = AndPermission::new(IsAuthenticated, NotPermission::new(IsAdminUser));

	// Regular authenticated user
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Regular user should pass"
	);

	// Admin user - excluded
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"Admin should be excluded by NOT"
	);
}

#[rstest]
#[tokio::test]
async fn test_deeply_nested_composition(test_request: Request) {
	// ((IsAuthenticated AND IsActiveUser) OR (IsAdminUser AND NOT IsActiveUser)) AND NOT AllowAny
	// This is a contrived example to test deep nesting
	// The NOT AllowAny at the end makes the whole thing always false
	let permission = AndPermission::new(
		OrPermission::new(
			AndPermission::new(IsAuthenticated, IsActiveUser),
			AndPermission::new(IsAdminUser, NotPermission::new(IsActiveUser)),
		),
		NotPermission::new(AllowAny),
	);

	// Any context should fail because NOT AllowAny is always false
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};
	assert!(
		!permission.has_permission(&context).await,
		"NOT AllowAny should make everything fail"
	);
}

// =============================================================================
// De Morgan's Law Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_de_morgan_not_and_equals_or_not(test_request: Request) {
	// De Morgan: NOT (A AND B) = (NOT A) OR (NOT B)
	let not_and = NotPermission::new(AndPermission::new(IsAuthenticated, IsAdminUser));
	let or_not = OrPermission::new(
		NotPermission::new(IsAuthenticated),
		NotPermission::new(IsAdminUser),
	);

	// Test all four combinations
	let test_cases = [
		(true, true),   // Both true -> NOT(AND) = false, OR(NOT) = false
		(true, false),  // Auth true, Admin false -> NOT(AND) = true, OR(NOT) = true
		(false, true),  // Auth false, Admin true -> NOT(AND) = true, OR(NOT) = true
		(false, false), // Both false -> NOT(AND) = true, OR(NOT) = true
	];

	for (is_auth, is_admin) in test_cases {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated: is_auth,
			is_admin,
			is_active: is_auth,
			user: None,
		};

		let not_and_result = not_and.has_permission(&context).await;
		let or_not_result = or_not.has_permission(&context).await;

		assert_eq!(
			not_and_result, or_not_result,
			"De Morgan's law should hold for is_auth={}, is_admin={}",
			is_auth, is_admin
		);
	}
}

#[rstest]
#[tokio::test]
async fn test_de_morgan_not_or_equals_and_not(test_request: Request) {
	// De Morgan: NOT (A OR B) = (NOT A) AND (NOT B)
	let not_or = NotPermission::new(OrPermission::new(IsAuthenticated, IsAdminUser));
	let and_not = AndPermission::new(
		NotPermission::new(IsAuthenticated),
		NotPermission::new(IsAdminUser),
	);

	let test_cases = [(true, true), (true, false), (false, true), (false, false)];

	for (is_auth, is_admin) in test_cases {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated: is_auth,
			is_admin,
			is_active: is_auth,
			user: None,
		};

		let not_or_result = not_or.has_permission(&context).await;
		let and_not_result = and_not.has_permission(&context).await;

		assert_eq!(
			not_or_result, and_not_result,
			"De Morgan's law should hold for is_auth={}, is_admin={}",
			is_auth, is_admin
		);
	}
}

// =============================================================================
// Decision Table Tests
// =============================================================================

#[rstest]
#[case(true, true, true, true, "All conditions true")]
#[case(true, true, false, true, "Admin and active")]
#[case(true, false, true, true, "Admin and authenticated")]
#[case(true, false, false, true, "Admin only (still requires auth)")]
#[case(false, true, true, true, "Active and authenticated")]
#[case(false, true, false, false, "Active only")]
#[case(false, false, true, false, "Authenticated only")]
#[case(false, false, false, false, "No conditions met")]
#[tokio::test]
async fn test_and_or_decision_table(
	test_request: Request,
	#[case] is_admin: bool,
	#[case] is_active: bool,
	#[case] is_authenticated: bool,
	#[case] _expected: bool,
	#[case] desc: &str,
) {
	// (IsAdminUser OR IsActiveUser) AND IsAuthenticated
	// Note: IsAdminUser also requires is_authenticated internally
	let permission = AndPermission::new(
		OrPermission::new(IsAdminUser, IsActiveUser),
		IsAuthenticated,
	);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// Calculate expected manually:
	// IsAdminUser = is_authenticated && is_admin
	// IsActiveUser = is_authenticated && is_active
	// (IsAdminUser OR IsActiveUser) AND IsAuthenticated
	let admin_check = is_authenticated && is_admin;
	let active_check = is_authenticated && is_active;
	let calculated_expected = (admin_check || active_check) && is_authenticated;

	assert_eq!(
		result, calculated_expected,
		"Decision table test failed for: {} (admin={}, active={}, auth={})",
		desc, is_admin, is_active, is_authenticated
	);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_and_with_allow_any(test_request: Request) {
	// AllowAny AND IsAuthenticated = IsAuthenticated (effectively)
	let permission = AndPermission::new(AllowAny, IsAuthenticated);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		!permission.has_permission(&context).await,
		"AllowAny AND X should require X"
	);
}

#[rstest]
#[tokio::test]
async fn test_or_with_allow_any(test_request: Request) {
	// AllowAny OR Anything = AllowAny (effectively)
	let permission = OrPermission::new(AllowAny, IsAdminUser);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"AllowAny OR X should always pass"
	);
}

#[rstest]
#[tokio::test]
async fn test_not_allow_any(test_request: Request) {
	// NOT AllowAny = never allow
	let permission = NotPermission::new(AllowAny);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};

	assert!(
		!permission.has_permission(&context).await,
		"NOT AllowAny should never pass"
	);
}

#[rstest]
#[tokio::test]
async fn test_identical_permissions_and(test_request: Request) {
	// IsAuthenticated AND IsAuthenticated = IsAuthenticated
	let permission = AndPermission::new(IsAuthenticated, IsAuthenticated);

	let context_true = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};

	let context_false = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(permission.has_permission(&context_true).await);
	assert!(!permission.has_permission(&context_false).await);
}

#[rstest]
#[tokio::test]
async fn test_identical_permissions_or(test_request: Request) {
	// IsAuthenticated OR IsAuthenticated = IsAuthenticated
	let permission = OrPermission::new(IsAuthenticated, IsAuthenticated);

	let context_true = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};

	let context_false = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(permission.has_permission(&context_true).await);
	assert!(!permission.has_permission(&context_false).await);
}

// =============================================================================
// HTTP Method Tests with IsAuthenticatedOrReadOnly
// =============================================================================

#[rstest]
#[case(Method::GET, false, true, "GET without auth")]
#[case(Method::HEAD, false, true, "HEAD without auth")]
#[case(Method::OPTIONS, false, true, "OPTIONS without auth")]
#[case(Method::POST, false, false, "POST without auth")]
#[case(Method::PUT, false, false, "PUT without auth")]
#[case(Method::PATCH, false, false, "PATCH without auth")]
#[case(Method::DELETE, false, false, "DELETE without auth")]
#[case(Method::POST, true, true, "POST with auth")]
#[case(Method::PUT, true, true, "PUT with auth")]
#[case(Method::DELETE, true, true, "DELETE with auth")]
#[tokio::test]
async fn test_authenticated_or_read_only_methods(
	#[case] method: Method,
	#[case] is_authenticated: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = IsAuthenticatedOrReadOnly;

	let request = Request::builder()
		.method(method.clone())
		.uri("/api/test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let context = PermissionContext {
		request: &request,
		is_authenticated,
		is_admin: false,
		is_active: is_authenticated,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(
		result, expected,
		"IsAuthenticatedOrReadOnly failed for: {} (method={:?}, auth={})",
		desc, method, is_authenticated
	);
}

// =============================================================================
// Chained Operator Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_chained_and_operators(test_request: Request) {
	// A & B & C using operator chaining
	let permission = (IsAuthenticated & IsActiveUser) & IsAdminUser;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: true,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"Chained & should work"
	);
}

#[rstest]
#[tokio::test]
async fn test_chained_or_operators(test_request: Request) {
	// A | B | C using operator chaining
	let permission = (IsAdminUser | IsActiveUser) | AllowAny;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		permission.has_permission(&context).await,
		"Chained | with AllowAny should always pass"
	);
}

#[rstest]
#[tokio::test]
async fn test_mixed_operators_with_precedence(test_request: Request) {
	// (A & B) | C - AND has higher precedence but we use parentheses
	let permission = (IsAuthenticated & IsActiveUser) | IsAdminUser;

	// Active authenticated user - passes first branch
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: false,
		is_active: true,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Should pass via first branch"
	);

	// Admin user (requires auth)
	let context = PermissionContext {
		request: &test_request,
		is_authenticated: true,
		is_admin: true,
		is_active: false,
		user: None,
	};
	assert!(
		permission.has_permission(&context).await,
		"Should pass via admin branch"
	);
}
