//! FastAPI param in path and dependency tests translated to Rust
//!
//! Based on: fastapi/tests/test_param_in_path_and_dependency.py
//!
//! These tests verify that:
//! 1. Path parameters can be used in both the endpoint and its dependencies
//! 2. Dependencies can access path parameters without duplication

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Path parameter wrapper
#[derive(Clone, Debug)]
struct UserId(i32);

#[async_trait::async_trait]
impl Injectable for UserId {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// TODO: Implement path parameter extraction from HTTP request
		// Current: Uses cached test value or mock value (UserId(42))
		// Required: Extract user_id from request path (e.g., /users/{user_id})
		if let Some(cached) = ctx.get_request::<UserId>() {
			return Ok((*cached).clone());
		}

		// For testing, we'll use a mock value
		let user_id = UserId(42);
		ctx.set_request(user_id.clone());
		Ok(user_id)
	}
}

// Dependency that validates user exists
struct UserValidator {
	user_id: i32,
	exists: bool,
}

#[async_trait::async_trait]
impl Injectable for UserValidator {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let user_id = UserId::inject(ctx).await?;

		// Validate user exists (mock implementation)
		let exists = user_id.0 > 0;

		Ok(UserValidator {
			user_id: user_id.0,
			exists,
		})
	}
}

#[tokio::test]
async fn test_path_param_available_to_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject validator which depends on path parameter
	let validator = UserValidator::inject(&ctx).await.unwrap();

	assert_eq!(validator.user_id, 42);
	assert!(validator.exists);
}

#[tokio::test]
async fn test_path_param_shared_between_dependency_and_endpoint() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject validator
	let validator = UserValidator::inject(&ctx).await.unwrap();

	// Also get the path parameter directly
	let user_id = UserId::inject(&ctx).await.unwrap();

	// Both should see the same value
	assert_eq!(validator.user_id, user_id.0);
}

// Test with different user IDs
#[tokio::test]
async fn test_different_path_params_in_different_requests() {
	// Request 1
	let singleton = Arc::new(SingletonScope::new());
	let ctx1 = InjectionContext::new(singleton.clone());

	// Set user_id for request 1
	ctx1.set_request(UserId(100));
	let validator1 = UserValidator::inject(&ctx1).await.unwrap();
	assert_eq!(validator1.user_id, 100);

	// Request 2
	let ctx2 = InjectionContext::new(singleton.clone());

	// Set user_id for request 2
	ctx2.set_request(UserId(200));
	let validator2 = UserValidator::inject(&ctx2).await.unwrap();
	assert_eq!(validator2.user_id, 200);

	// Verify they're different
	assert_ne!(validator1.user_id, validator2.user_id);
}

// Test dependency that uses path parameter for validation
struct PathParamDependency {
	value: i32,
}

#[async_trait::async_trait]
impl Injectable for PathParamDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let user_id = UserId::inject(ctx).await?;
		Ok(PathParamDependency { value: user_id.0 })
	}
}

#[tokio::test]
async fn test_multiple_dependencies_access_same_path_param() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set path parameter
	ctx.set_request(UserId(42));

	// Inject multiple dependencies that all use the path parameter
	let validator = UserValidator::inject(&ctx).await.unwrap();
	let dep = PathParamDependency::inject(&ctx).await.unwrap();

	// All should see the same value
	assert_eq!(validator.user_id, 42);
	assert_eq!(dep.value, 42);
}
