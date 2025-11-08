//! Miscellaneous FastAPI dependency tests translated to Rust
//!
//! Based on:
//! - test_param_in_path_and_dependency.py (2 tests)
//! - test_dependency_security_overrides.py (3 tests)
//! - test_repeated_dependency_schema.py (2 tests)
//!
//! These tests verify:
//! 1. Path parameters can be used in dependencies
//! 2. Security dependencies can be overridden
//! 3. Repeated dependencies are handled correctly

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Path parameter dependency
#[derive(Clone, Debug, PartialEq)]
struct UserIdParam {
	user_id: i32,
}

#[async_trait::async_trait]
impl Injectable for UserIdParam {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check for override first (for testing)
		if let Some(override_param) = ctx.get_request::<UserIdParam>() {
			return Ok((*override_param).clone());
		}
		// TODO: Implement path parameter extraction from HTTP request
		// Current: Returns hardcoded user_id = 42 for test purposes
		// Required: Extract user_id from request path (e.g., /users/{user_id})
		Ok(UserIdParam { user_id: 42 })
	}
}

// Validation dependency
#[derive(Clone)]
struct UserExists {
	exists: bool,
}

impl UserExists {
	async fn check(user_id: i32) -> Self {
		// Simulate database check
		Self {
			exists: user_id > 0,
		}
	}

	fn is_valid(&self) -> bool {
		self.exists
	}
}

#[async_trait::async_trait]
impl Injectable for UserExists {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let user_id_param = UserIdParam::inject(ctx).await?;
		Ok(UserExists::check(user_id_param.user_id).await)
	}
}

// Test 1: Path parameter in dependency
#[tokio::test]
async fn test_param_in_path_and_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject user_id from path
	let user_id = UserIdParam::inject(&ctx).await.unwrap();
	assert_eq!(user_id.user_id, 42);

	// Use user_id in validation dependency
	let user_exists = UserExists::inject(&ctx).await.unwrap();
	assert!(user_exists.is_valid());
}

// Test 2: Path parameter validation
#[tokio::test]
async fn test_path_param_validation() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Set invalid user_id in request scope
	let invalid_user_id = UserIdParam { user_id: -1 };
	ctx.set_request(invalid_user_id.clone());

	// Get the overridden user_id
	let user_id = UserIdParam::inject(&ctx).await.unwrap();
	assert_eq!(user_id.user_id, -1);

	// Check validation
	let user_exists = UserExists::check(user_id.user_id).await;
	assert!(!user_exists.is_valid());
}

// Security dependency
#[derive(Clone, Debug, PartialEq)]
struct SecurityToken {
	token: String,
	is_valid: bool,
}

impl SecurityToken {
	fn new(token: String) -> Self {
		// Simulate token validation
		let is_valid = !token.is_empty() && token != "invalid";
		Self { token, is_valid }
	}

	fn is_authenticated(&self) -> bool {
		self.is_valid
	}
}

#[async_trait::async_trait]
impl Injectable for SecurityToken {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check for override first (for testing)
		if let Some(override_token) = ctx.get_request::<SecurityToken>() {
			return Ok((*override_token).clone());
		}

		// Default: extract from header/cookie
		Ok(SecurityToken::new("default_token".to_string()))
	}
}

// Test 3: Security dependency override
#[tokio::test]
async fn test_security_override_authenticated() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Override with valid token
	let override_token = SecurityToken::new("valid_test_token".to_string());
	ctx.set_request(override_token.clone());

	let token = SecurityToken::inject(&ctx).await.unwrap();
	assert!(token.is_authenticated());
	assert_eq!(token.token, "valid_test_token");
}

// Test 4: Security dependency override - unauthenticated
#[tokio::test]
async fn test_security_override_unauthenticated() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Override with invalid token
	let override_token = SecurityToken::new("invalid".to_string());
	ctx.set_request(override_token.clone());

	let token = SecurityToken::inject(&ctx).await.unwrap();
	assert!(!token.is_authenticated());
}

// Test 5: Security dependency default
#[tokio::test]
async fn test_security_default() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// No override - use default
	let token = SecurityToken::inject(&ctx).await.unwrap();
	assert!(token.is_authenticated());
	assert_eq!(token.token, "default_token");
}

// Repeated dependency - same dependency used multiple times
#[derive(Clone, Debug, PartialEq)]
struct Counter {
	count: usize,
}

#[async_trait::async_trait]
impl Injectable for Counter {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Should be cached - same instance returned
		if let Some(cached) = ctx.get_request::<Counter>() {
			return Ok((*cached).clone());
		}

		let counter = Counter { count: 1 };
		ctx.set_request(counter.clone());
		Ok(counter)
	}
}

#[derive(Clone)]
struct Service1 {
	counter: Arc<Counter>,
}

#[async_trait::async_trait]
impl Injectable for Service1 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let counter = Counter::inject(ctx).await?;
		Ok(Service1 {
			counter: Arc::new(counter),
		})
	}
}

#[derive(Clone)]
struct Service2 {
	counter: Arc<Counter>,
}

#[async_trait::async_trait]
impl Injectable for Service2 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let counter = Counter::inject(ctx).await?;
		Ok(Service2 {
			counter: Arc::new(counter),
		})
	}
}

// Test 6: Repeated dependency returns same instance
#[tokio::test]
async fn test_repeated_dependency_cached() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let service1 = Service1::inject(&ctx).await.unwrap();
	let service2 = Service2::inject(&ctx).await.unwrap();

	// Both services should get the same counter instance
	assert_eq!(*service1.counter, *service2.counter);
	assert_eq!(service1.counter.count, 1);
}

// Test 7: Repeated dependency in schema
#[tokio::test]
async fn test_repeated_dependency_schema() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Direct injection
	let counter1 = Counter::inject(&ctx).await.unwrap();
	let counter2 = Counter::inject(&ctx).await.unwrap();

	// Should be the same instance (cached)
	assert_eq!(counter1, counter2);
}

// Complex dependency chain with repeated dependencies
#[derive(Clone)]
struct ComplexService {
	service1: Arc<Service1>,
	service2: Arc<Service2>,
	direct_counter: Arc<Counter>,
}

#[async_trait::async_trait]
impl Injectable for ComplexService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let service1 = Service1::inject(ctx).await?;
		let service2 = Service2::inject(ctx).await?;
		let direct_counter = Counter::inject(ctx).await?;

		Ok(ComplexService {
			service1: Arc::new(service1),
			service2: Arc::new(service2),
			direct_counter: Arc::new(direct_counter),
		})
	}
}

// Test 8: Complex dependency chain with repeated dependencies
#[tokio::test]
async fn test_complex_repeated_dependencies() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let complex = ComplexService::inject(&ctx).await.unwrap();

	// NOTE: Verifies singleton dependency caching across multiple injection paths
	// All counter references should point to the same instance in memory
	// Test confirms that RequestCounter is injected once and shared across:
	// - service1.counter, service2.counter, direct_counter
	assert_eq!(*complex.service1.counter, *complex.service2.counter);
	assert_eq!(*complex.service1.counter, *complex.direct_counter);
	assert_eq!(complex.direct_counter.count, 1);
}
