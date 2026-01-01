//! Permission Decision Table Tests
//!
//! This module contains systematic decision table tests for permission classes.
//! Decision tables ensure complete coverage of all possible input combinations
//! and their expected outputs.
//!
//! # Test Categories
//!
//! - Complete Truth Tables: All 2^n combinations for n boolean inputs
//! - Boundary Conditions: Edge cases at decision boundaries
//! - Permission Class Behavior: Individual permission class semantics

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

#[fixture]
fn test_request() -> Request {
	Request::builder()
		.method(Method::GET)
		.uri("/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap()
}

// =============================================================================
// IsAuthenticated Decision Table
// =============================================================================

#[rstest]
#[case(false, false, "Unauthenticated user denied")]
#[case(true, true, "Authenticated user allowed")]
#[tokio::test]
async fn test_is_authenticated_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = IsAuthenticated;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active: is_authenticated,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(result, expected, "IsAuthenticated failed for: {}", desc);
}

// =============================================================================
// IsAdminUser Decision Table
// =============================================================================

#[rstest]
#[case(false, false, false, "Anonymous, non-admin")]
#[case(false, true, false, "Anonymous but marked admin (should still fail)")]
#[case(true, false, false, "Authenticated non-admin")]
#[case(true, true, true, "Authenticated admin")]
#[tokio::test]
async fn test_is_admin_user_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_admin: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = IsAdminUser;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active: is_authenticated,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(result, expected, "IsAdminUser failed for: {}", desc);
}

// =============================================================================
// IsActiveUser Decision Table
// =============================================================================

#[rstest]
#[case(false, false, false, "Anonymous, inactive")]
#[case(false, true, false, "Anonymous but marked active (should still fail)")]
#[case(true, false, false, "Authenticated but inactive")]
#[case(true, true, true, "Authenticated and active")]
#[tokio::test]
async fn test_is_active_user_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = IsActiveUser;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(result, expected, "IsActiveUser failed for: {}", desc);
}

// =============================================================================
// AllowAny Decision Table
// =============================================================================

#[rstest]
#[case(false, false, false, true, "All false - still allowed")]
#[case(true, false, false, true, "Authenticated only")]
#[case(false, true, false, true, "Admin only")]
#[case(false, false, true, true, "Active only")]
#[case(true, true, true, true, "All true")]
#[tokio::test]
async fn test_allow_any_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_admin: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = AllowAny;

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(result, expected, "AllowAny failed for: {}", desc);
}

// =============================================================================
// IsAuthenticatedOrReadOnly Decision Table
// =============================================================================

#[rstest]
// Safe methods (read-only) - should pass regardless of auth
#[case(Method::GET, false, true, "GET anonymous")]
#[case(Method::GET, true, true, "GET authenticated")]
#[case(Method::HEAD, false, true, "HEAD anonymous")]
#[case(Method::HEAD, true, true, "HEAD authenticated")]
#[case(Method::OPTIONS, false, true, "OPTIONS anonymous")]
#[case(Method::OPTIONS, true, true, "OPTIONS authenticated")]
// Unsafe methods - require auth
#[case(Method::POST, false, false, "POST anonymous")]
#[case(Method::POST, true, true, "POST authenticated")]
#[case(Method::PUT, false, false, "PUT anonymous")]
#[case(Method::PUT, true, true, "PUT authenticated")]
#[case(Method::PATCH, false, false, "PATCH anonymous")]
#[case(Method::PATCH, true, true, "PATCH authenticated")]
#[case(Method::DELETE, false, false, "DELETE anonymous")]
#[case(Method::DELETE, true, true, "DELETE authenticated")]
#[tokio::test]
async fn test_is_authenticated_or_read_only_decision_table(
	#[case] method: Method,
	#[case] is_authenticated: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = IsAuthenticatedOrReadOnly;

	let request = Request::builder()
		.method(method.clone())
		.uri("/")
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
		"IsAuthenticatedOrReadOnly failed for: {} (method={:?})",
		desc, method
	);
}

// =============================================================================
// AND Permission Complete Truth Table
// =============================================================================

#[rstest]
#[case(false, false, false, "F AND F = F")]
#[case(false, true, false, "F AND T = F")]
#[case(true, false, false, "T AND F = F")]
#[case(true, true, true, "T AND T = T")]
#[tokio::test]
async fn test_and_permission_truth_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	// IsAuthenticated AND IsActiveUser
	// Note: IsActiveUser internally requires is_authenticated, so we test
	// the combination of context flags directly
	let permission = AndPermission::new(IsAuthenticated, IsActiveUser);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// Expected = is_authenticated AND (is_authenticated AND is_active)
	// = is_authenticated AND is_active (since second requires first)
	let calculated_expected = is_authenticated && is_active;

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for: {}",
		desc
	);
	assert_eq!(result, expected, "AND truth table failed for: {}", desc);
}

// =============================================================================
// OR Permission Complete Truth Table
// =============================================================================

#[rstest]
#[case(false, false, false, false, "F OR F = F")]
#[case(false, true, true, true, "F OR T = T (via active)")]
#[case(true, true, false, true, "T OR F = T (via admin)")]
#[case(true, true, true, true, "T OR T = T")]
#[tokio::test]
async fn test_or_permission_truth_table(
	test_request: Request,
	#[case] is_admin: bool,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	// IsAdminUser OR IsActiveUser
	let permission = OrPermission::new(IsAdminUser, IsActiveUser);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// IsAdminUser = is_authenticated AND is_admin
	// IsActiveUser = is_authenticated AND is_active
	// Expected = (is_authenticated AND is_admin) OR (is_authenticated AND is_active)
	let admin_check = is_authenticated && is_admin;
	let active_check = is_authenticated && is_active;
	let calculated_expected = admin_check || active_check;

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for: {}",
		desc
	);
	assert_eq!(result, expected, "OR truth table failed for: {}", desc);
}

// =============================================================================
// NOT Permission Complete Truth Table
// =============================================================================

#[rstest]
#[case(false, true, "NOT F = T")]
#[case(true, false, "NOT T = F")]
#[tokio::test]
async fn test_not_permission_truth_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	let permission = NotPermission::new(IsAuthenticated);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active: is_authenticated,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	assert_eq!(result, expected, "NOT truth table failed for: {}", desc);
}

// =============================================================================
// Complex Expression Decision Tables
// =============================================================================

#[rstest]
// (A AND B) OR C - where A=IsAuthenticated, B=IsActiveUser, C=IsAdminUser
// Note: C (IsAdminUser) requires authentication
#[case(false, false, false, false, "000: All false")]
#[case(false, false, true, false, "001: Admin marked but not auth")]
#[case(false, true, false, false, "010: Active marked but not auth")]
#[case(false, true, true, false, "011: Active+Admin but not auth")]
#[case(true, false, false, false, "100: Auth only")]
#[case(true, false, true, true, "101: Auth+Admin (admin branch)")]
#[case(true, true, false, true, "110: Auth+Active (first branch)")]
#[case(true, true, true, true, "111: All true (both branches)")]
#[tokio::test]
async fn test_complex_and_or_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] is_admin: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	// (IsAuthenticated AND IsActiveUser) OR IsAdminUser
	let permission = OrPermission::new(
		AndPermission::new(IsAuthenticated, IsActiveUser),
		IsAdminUser,
	);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// Calculate expected:
	// First branch: is_authenticated AND (is_authenticated AND is_active)
	//             = is_authenticated AND is_active
	// Second branch: is_authenticated AND is_admin
	// Result: (is_authenticated AND is_active) OR (is_authenticated AND is_admin)
	let first_branch = is_authenticated && is_active;
	let second_branch = is_authenticated && is_admin;
	let calculated_expected = first_branch || second_branch;

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for: {}",
		desc
	);
	assert_eq!(result, expected, "Complex AND/OR failed for: {}", desc);
}

// =============================================================================
// Three-Variable Complete Decision Table
// =============================================================================

#[rstest]
// A AND B AND C - complete 8-row truth table
#[case(false, false, false, false)]
#[case(false, false, true, false)]
#[case(false, true, false, false)]
#[case(false, true, true, false)]
#[case(true, false, false, false)]
#[case(true, false, true, false)]
#[case(true, true, false, false)]
#[case(true, true, true, true)]
#[tokio::test]
async fn test_triple_and_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] is_admin: bool,
	#[case] expected: bool,
) {
	// IsAuthenticated AND IsActiveUser AND IsAdminUser
	let permission = AndPermission::new(
		AndPermission::new(IsAuthenticated, IsActiveUser),
		IsAdminUser,
	);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// All three base permissions require authentication
	// So the result is: is_authenticated AND is_active AND is_admin
	let calculated_expected = is_authenticated && is_active && is_admin;

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for auth={}, active={}, admin={}",
		is_authenticated, is_active, is_admin
	);
	assert_eq!(
		result, expected,
		"Triple AND failed for auth={}, active={}, admin={}",
		is_authenticated, is_active, is_admin
	);
}

// =============================================================================
// NOT Combined with AND/OR Decision Tables
// =============================================================================

#[rstest]
// NOT (A AND B) - De Morgan equivalent to (NOT A) OR (NOT B)
#[case(false, false, true, "NOT(F AND F) = T")]
#[case(false, true, true, "NOT(F AND T) = T")]
#[case(true, false, true, "NOT(T AND F) = T")]
#[case(true, true, false, "NOT(T AND T) = F")]
#[tokio::test]
async fn test_not_and_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	// NOT (IsAuthenticated AND IsActiveUser)
	let permission = NotPermission::new(AndPermission::new(IsAuthenticated, IsActiveUser));

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// NOT (is_authenticated AND is_active)
	let calculated_expected = !(is_authenticated && is_active);

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for: {}",
		desc
	);
	assert_eq!(result, expected, "NOT AND failed for: {}", desc);
}

#[rstest]
// NOT (A OR B) - De Morgan equivalent to (NOT A) AND (NOT B)
#[case(false, false, true, "NOT(F OR F) = T")]
#[case(false, true, true, "NOT(F OR F) = T (IsActiveUser requires auth)")]
#[case(true, false, false, "NOT(T OR F) = F")]
#[case(true, true, false, "NOT(T OR T) = F")]
#[tokio::test]
async fn test_not_or_decision_table(
	test_request: Request,
	#[case] is_authenticated: bool,
	#[case] is_active: bool,
	#[case] expected: bool,
	#[case] desc: &str,
) {
	// NOT (IsAuthenticated OR IsActiveUser)
	// Note: IsActiveUser requires auth, so we need to be careful with interpretation
	let permission = NotPermission::new(OrPermission::new(IsAuthenticated, IsActiveUser));

	let context = PermissionContext {
		request: &test_request,
		is_authenticated,
		is_admin: false,
		is_active,
		user: None,
	};

	let result = permission.has_permission(&context).await;

	// IsAuthenticated = is_authenticated
	// IsActiveUser = is_authenticated AND is_active
	// NOT (IsAuthenticated OR IsActiveUser)
	// = NOT (is_authenticated OR (is_authenticated AND is_active))
	// = NOT is_authenticated (since OR with subset is just the superset)
	let calculated_expected = !is_authenticated;

	// Verify test case data is consistent
	assert_eq!(
		expected, calculated_expected,
		"Test case expected value mismatch for: {}",
		desc
	);
	assert_eq!(result, expected, "NOT OR failed for: {}", desc);
}

// =============================================================================
// Edge Cases Decision Tables
// =============================================================================

#[rstest]
// AllowAny in combinations
#[case(true, true, "AllowAny AND AllowAny")]
#[case(true, true, "AllowAny OR AllowAny")]
#[tokio::test]
async fn test_allow_any_combinations(
	test_request: Request,
	#[case] _expected_and: bool,
	#[case] _expected_or: bool,
	#[case] _desc: &str,
) {
	let and_permission = AndPermission::new(AllowAny, AllowAny);
	let or_permission = OrPermission::new(AllowAny, AllowAny);

	let context = PermissionContext {
		request: &test_request,
		is_authenticated: false,
		is_admin: false,
		is_active: false,
		user: None,
	};

	assert!(
		and_permission.has_permission(&context).await,
		"AllowAny AND AllowAny should always pass"
	);
	assert!(
		or_permission.has_permission(&context).await,
		"AllowAny OR AllowAny should always pass"
	);
}

#[rstest]
#[tokio::test]
async fn test_not_allow_any_decision_table(test_request: Request) {
	// NOT AllowAny should always fail
	let permission = NotPermission::new(AllowAny);

	let test_cases = [
		(false, false, false),
		(true, false, false),
		(false, true, false),
		(true, true, true),
	];

	for (is_authenticated, is_admin, is_active) in test_cases {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated,
			is_admin,
			is_active,
			user: None,
		};

		let result = permission.has_permission(&context).await;

		assert!(
			!result,
			"NOT AllowAny should always fail for auth={}, admin={}, active={}",
			is_authenticated, is_admin, is_active
		);
	}
}

// =============================================================================
// Absorption Law Decision Tables
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_absorption_law_and_or(test_request: Request) {
	// A AND (A OR B) = A (Absorption Law)
	let absorption = AndPermission::new(
		IsAuthenticated,
		OrPermission::new(IsAuthenticated, IsAdminUser),
	);
	let simple = IsAuthenticated;

	let test_cases = [(false, false), (false, true), (true, false), (true, true)];

	for (is_authenticated, is_admin) in test_cases {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated,
			is_admin,
			is_active: is_authenticated,
			user: None,
		};

		let absorption_result = absorption.has_permission(&context).await;
		let simple_result = simple.has_permission(&context).await;

		assert_eq!(
			absorption_result, simple_result,
			"Absorption law A AND (A OR B) = A failed for auth={}, admin={}",
			is_authenticated, is_admin
		);
	}
}

#[rstest]
#[tokio::test]
async fn test_absorption_law_or_and(test_request: Request) {
	// A OR (A AND B) = A (Absorption Law)
	let absorption = OrPermission::new(
		IsAuthenticated,
		AndPermission::new(IsAuthenticated, IsAdminUser),
	);
	let simple = IsAuthenticated;

	let test_cases = [(false, false), (false, true), (true, false), (true, true)];

	for (is_authenticated, is_admin) in test_cases {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated,
			is_admin,
			is_active: is_authenticated,
			user: None,
		};

		let absorption_result = absorption.has_permission(&context).await;
		let simple_result = simple.has_permission(&context).await;

		assert_eq!(
			absorption_result, simple_result,
			"Absorption law A OR (A AND B) = A failed for auth={}, admin={}",
			is_authenticated, is_admin
		);
	}
}

// =============================================================================
// Idempotent Law Decision Tables
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_idempotent_law_and(test_request: Request) {
	// A AND A = A (Idempotent Law)
	let idempotent = AndPermission::new(IsAuthenticated, IsAuthenticated);
	let simple = IsAuthenticated;

	for is_authenticated in [false, true] {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated,
			is_admin: false,
			is_active: is_authenticated,
			user: None,
		};

		let idempotent_result = idempotent.has_permission(&context).await;
		let simple_result = simple.has_permission(&context).await;

		assert_eq!(
			idempotent_result, simple_result,
			"Idempotent law A AND A = A failed for auth={}",
			is_authenticated
		);
	}
}

#[rstest]
#[tokio::test]
async fn test_idempotent_law_or(test_request: Request) {
	// A OR A = A (Idempotent Law)
	let idempotent = OrPermission::new(IsAuthenticated, IsAuthenticated);
	let simple = IsAuthenticated;

	for is_authenticated in [false, true] {
		let context = PermissionContext {
			request: &test_request,
			is_authenticated,
			is_admin: false,
			is_active: is_authenticated,
			user: None,
		};

		let idempotent_result = idempotent.has_permission(&context).await;
		let simple_result = simple.has_permission(&context).await;

		assert_eq!(
			idempotent_result, simple_result,
			"Idempotent law A OR A = A failed for auth={}",
			is_authenticated
		);
	}
}
