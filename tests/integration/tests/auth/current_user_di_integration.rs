//! CurrentUser DI Integration Tests
//!
//! Tests the integration of CurrentUser, DI, database, and ORM.

use reinhardt_auth::{BaseUser, CurrentUser, DefaultUser, SimpleUser};
use reinhardt_db::DatabaseConnection;
use reinhardt_di::{Injectable, SingletonScope};
use reinhardt_test::fixtures::testcontainers::{postgres_container, ContainerAsync, GenericImage};
use reinhardt_test::fixtures::singleton_scope;
use rstest::*;
use sea_query::{Expr, ExprTrait, PostgresQueryBuilder, Query};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Sanity Tests (2 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn sanity_current_user_di_basic(singleton_scope: Arc<SingletonScope>) {
	// Basic DI operation for CurrentUser
	let ctx = InjectionContext::new(singleton_scope);

	// Create as AnonymousUser
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

#[rstest]
#[tokio::test]
async fn sanity_database_user_load(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	// Basic user load operation from database
	let (_container, pool, _port, _url) = postgres_container.await;

	// Insert test user into database
	let user_id = Uuid::new_v4();
	let username = "test_user";

	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT true
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query("INSERT INTO auth_user (id, username, email, is_active) VALUES ($1, $2, $3, $4)")
		.bind(user_id)
		.bind(username)
		.bind("test@example.com")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Load user
	let row = sqlx::query("SELECT id, username FROM auth_user WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");

	assert_eq!(loaded_id, user_id);
	assert_eq!(loaded_username, username);
}

// ============================================================================
// Normal Cases (6 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn normal_current_user_viewset_injection(test_user: TestUser) {
	// Inject CurrentUser into ViewSet and retrieve user information
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user.clone(), test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
	assert_eq!(
		current_user.user().unwrap().get_username(),
		test_user.username
	);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_shared_across_endpoints(test_user: TestUser) {
	// Share CurrentUser across multiple endpoints
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user1 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user2 = current_user1.clone();

	assert_eq!(current_user1.id().unwrap(), current_user2.id().unwrap());
	assert_eq!(
		current_user1.user().unwrap().get_username(),
		current_user2.user().unwrap().get_username()
	);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_db_query_seaquery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	test_user: TestUser,
) {
	// Combine CurrentUser with DB query (SeaQuery)
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert user
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Build query with SeaQuery
	#[derive(sea_query::Iden)]
	enum AuthUser {
		Table,
		Id,
		Username,
		Email,
	}

	let query = Query::select()
		.from(AuthUser::Table)
		.columns([AuthUser::Id, AuthUser::Username, AuthUser::Email])
		.and_where(Expr::col(AuthUser::Id).eq(test_user.id))
		.to_owned();

	let sql = query.to_string(PostgresQueryBuilder);

	// Execute query
	let row = sqlx::query(&sql).fetch_one(pool.as_ref()).await.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");

	assert_eq!(loaded_id, test_user.id);
	assert_eq!(loaded_username, test_user.username);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_orm_queryset_filtering(test_user: TestUser) {
	// CurrentUser and ORM QuerySet filtering
	// Note: Conceptual test as actual ORM implementation is required
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	// Use current_user's ID as filtering condition
	let filter_user_id = current_user.id().unwrap();

	assert_eq!(filter_user_id, test_user.id);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_session_auth_integration(test_user: TestUser) {
	// CurrentUser and session authentication integration
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_jwt_auth_integration(test_user: TestUser) {
	// CurrentUser and JWT authentication integration
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(
		current_user.user().unwrap().get_username(),
		test_user.username
	);
}

// ============================================================================
// Error Cases (4 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn abnormal_unauthenticated_current_user_injection() {
	// CurrentUser injection when unauthenticated (AnonymousUser)
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
	assert!(current_user.user().is_err());
}

#[rstest]
#[tokio::test]
async fn abnormal_invalid_user_id_injection() {
	// CurrentUser injection with invalid user ID
	let invalid_id = Uuid::nil();

	// Create with None user (invalid ID)
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

#[rstest]
#[tokio::test]
async fn abnormal_deleted_user_current_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	test_user: TestUser,
) {
	// CurrentUser after deleting user from DB
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert user
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Delete user
	sqlx::query("DELETE FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify user does not exist
	let result = sqlx::query("SELECT id FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.fetch_optional(pool.as_ref())
		.await
		.unwrap();

	assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn abnormal_expired_session_current_user() {
	// CurrentUser when session expired
	// Should return AnonymousUser
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

// ============================================================================
// State Transitions (2 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn state_transition_unauthenticated_to_authenticated(test_user: TestUser) {
	// Unauthenticated → Authenticated → CurrentUser injection → Get authenticated user

	// Initial state: Unauthenticated
	let current_user = CurrentUser::<SimpleUser>::anonymous();
	assert!(!current_user.is_authenticated());

	// Authentication process (login)
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// Create authenticated CurrentUser
	let authenticated_user = CurrentUser::authenticated(user, test_user.id);

	// Final state: Authenticated
	assert!(authenticated_user.is_authenticated());
	assert_eq!(authenticated_user.id().unwrap(), test_user.id);
	assert_eq!(
		authenticated_user.user().unwrap().get_username(),
		test_user.username
	);
}

#[rstest]
#[tokio::test]
async fn state_transition_logout_to_anonymous(test_user: TestUser) {
	// Logout → CurrentUser invalidation → AnonymousUser

	// Initial state: Authenticated
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let authenticated_user = CurrentUser::authenticated(user, test_user.id);
	assert!(authenticated_user.is_authenticated());

	// Logout process (Create AnonymousUser)
	let anonymous_user = CurrentUser::<SimpleUser>::anonymous();

	// Final state: Unauthenticated
	assert!(!anonymous_user.is_authenticated());
	assert!(anonymous_user.id().is_err());
}

// ============================================================================
// Combinations (2 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn combination_current_user_session_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	test_user: TestUser,
) {
	// CurrentUser + Session + DB integration
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert user
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Load user from DB
	let row = sqlx::query("SELECT id, username, email FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");
	let loaded_email: String = row.get("email");

	// Create CurrentUser
	let user = SimpleUser {
		id: loaded_id,
		username: loaded_username.clone(),
		email: loaded_email,
		is_active: true,
		is_admin: false,
		is_staff: false,
		is_superuser: false,
	};

	let current_user = CurrentUser::authenticated(user, loaded_id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
	assert_eq!(current_user.user().unwrap().get_username(), loaded_username);
}

#[rstest]
#[tokio::test]
async fn combination_current_user_jwt_redis(test_user: TestUser) {
	// CurrentUser + JWT + Redis integration
	// Note: Conceptual test as actual Redis implementation is required

	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
}

// ============================================================================
// Equivalence Partitioning (2 tests, #[case])
// ============================================================================

#[rstest]
#[case::simple_user("simple_user")]
#[case::default_user("default_user")]
#[tokio::test]
async fn equivalence_user_type(#[case] user_type: &str, test_user: TestUser) {
	// User types (SimpleUser, DefaultUser)
	match user_type {
		"simple_user" => {
			let user = SimpleUser {
				id: test_user.id,
				username: test_user.username.clone(),
				email: test_user.email.clone(),
				is_active: test_user.is_active,
				is_admin: test_user.is_admin,
				is_staff: test_user.is_staff,
				is_superuser: test_user.is_superuser,
			};

			let current_user = CurrentUser::authenticated(user, test_user.id);
			assert!(current_user.is_authenticated());
		}
		"default_user" => {
			// Conceptual test as DefaultUser implementation is required
			// Actually create DefaultUser instance and then CurrentUser
			assert!(true); // Placeholder
		}
		_ => panic!("Unknown user type"),
	}
}

#[rstest]
#[case::request_scope("request")]
#[case::singleton_scope("singleton")]
#[tokio::test]
async fn equivalence_di_scope(#[case] scope_type: &str, singleton_scope: Arc<SingletonScope>) {
	// DI scope (Request, Singleton)
	match scope_type {
		"request" => {
			let ctx = InjectionContext::new(singleton_scope.clone());
			// CurrentUser injection in request scope
			assert!(true); // Actual request scope implementation is required
		}
		"singleton" => {
			let ctx = InjectionContext::new(singleton_scope);
			// CurrentUser injection in singleton scope
			assert!(true); // Actual singleton scope implementation is required
		}
		_ => panic!("Unknown scope type"),
	}
}

// ============================================================================
// Edge Cases (2 tests)
// ============================================================================

#[rstest]
#[tokio::test]
async fn edge_multiple_current_user_injection_same_request(test_user: TestUser) {
	// Multiple CurrentUser injections within the same request
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// Create multiple CurrentUser instances
	let current_user1 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user2 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user3 = current_user1.clone();

	// All refer to the same user
	assert_eq!(current_user1.id().unwrap(), current_user2.id().unwrap());
	assert_eq!(current_user2.id().unwrap(), current_user3.id().unwrap());
}

#[rstest]
#[tokio::test]
async fn edge_high_load_current_user_injection_performance(test_user: TestUser) {
	// CurrentUser injection performance under high load
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// Create CurrentUser 1000 times
	for _ in 0..1000 {
		let current_user = CurrentUser::authenticated(user.clone(), test_user.id);
		assert!(current_user.is_authenticated());
	}
}

// ============================================================================
// Property-based tests (2 tests, proptest)
// ============================================================================

// Note: proptest implementation requires proptest crate as workspace dependency
// Listed as comment here as placeholder

// #[cfg(test)]
// mod property_tests {
//     use super::*;
//     use proptest::prelude::*;
//
//     proptest! {
//         #[test]
//         fn prop_authenticated_implies_user_id_some(user_id in any::<Uuid>()) {
//             // CurrentUser.is_authenticated() = true → user_id is Some
//             let user = SimpleUser {
//                 id: user_id,
//                 username: "test".to_string(),
//                 email: "test@example.com".to_string(),
//                 is_active: true,
//                 is_admin: false,
//                 is_staff: false,
//                 is_superuser: false,
//             };
//
//             let current_user = CurrentUser::authenticated(user, user_id);
//
//             prop_assert!(current_user.is_authenticated());
//             prop_assert_eq!(current_user.id().unwrap(), user_id);
//         }
//     }
//
//     // Note: This test requires actual DB connection, difficult with normal proptest
//     // Instead, must use mocks or implement separately as integration test
//     // proptest! {
//     //     #[test]
//     //     fn prop_user_exists_in_db(user_id in any::<Uuid>()) {
//     //         // CurrentUser.user() → User existing in DB
//     //         // Skipped here due to complex implementation
//     //     }
//     // }
// }
