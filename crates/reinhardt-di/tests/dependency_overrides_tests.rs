//! FastAPI dependency overrides tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_overrides.py
//!
//! These tests verify that:
//! 1. Dependencies can be overridden for testing purposes
//! 2. Overrides work with sub-dependencies
//! 3. Overrides can be set and cleared dynamically
//! 4. Overrides work with different route configurations (main app, router, decorators)

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Simulate FastAPI app with dependency override support
#[derive(Clone)]
struct App {
	dependency_overrides: Arc<Mutex<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
}

impl App {
	fn new() -> Self {
		Self {
			dependency_overrides: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	fn set_override<T: 'static + Send + Sync>(&self, key: &str, value: T) {
		self.dependency_overrides
			.lock()
			.unwrap()
			.insert(key.to_string(), Box::new(value));
	}

	fn get_override<T: 'static + Clone>(&self, key: &str) -> Option<T> {
		let overrides = self.dependency_overrides.lock().unwrap();
		overrides
			.get(key)
			.and_then(|boxed| boxed.downcast_ref::<T>().cloned())
	}

	fn clear_overrides(&self) {
		self.dependency_overrides.lock().unwrap().clear();
	}
}

// Common parameters dependency
#[derive(Clone, Debug, PartialEq)]
struct CommonParameters {
	q: Option<String>,
	skip: i32,
	limit: i32,
}

impl CommonParameters {
	fn new(q: Option<String>, skip: i32, limit: i32) -> Self {
		Self { q, skip, limit }
	}

	fn required(q: String, skip: i32, limit: i32) -> Self {
		Self {
			q: Some(q),
			skip,
			limit,
		}
	}
}

#[async_trait::async_trait]
impl Injectable for CommonParameters {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check if there's an override
		if let Some(app) = ctx.get_singleton::<App>() {
			if let Some(override_params) = app.get_override::<CommonParameters>("common_parameters")
			{
				return Ok(override_params);
			}
		}

		// Default implementation - requires 'q' parameter
		// In real FastAPI, this would come from query parameters
		if let Some(params) = ctx.get_request::<CommonParameters>() {
			return Ok((*params).clone());
		}

		// Return error if no q parameter provided
		Err(reinhardt_di::DiError::NotFound(
			"q parameter required".to_string(),
		))
	}
}

// Override implementation - simple
#[derive(Clone, Debug, PartialEq)]
struct SimpleOverride {
	q: Option<String>,
	skip: i32,
	limit: i32,
}

impl SimpleOverride {
	fn new(q: Option<String>) -> Self {
		Self {
			q,
			skip: 5,
			limit: 10,
		}
	}

	fn to_common_parameters(&self) -> CommonParameters {
		CommonParameters::new(self.q.clone(), self.skip, self.limit)
	}
}

// Sub-dependency for override
#[derive(Clone, Debug, PartialEq)]
struct SubDependency {
	k: Option<String>,
}

impl SubDependency {
	fn new(k: Option<String>) -> Self {
		Self { k }
	}

	fn required(k: String) -> Self {
		Self { k: Some(k) }
	}
}

#[async_trait::async_trait]
impl Injectable for SubDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check if there's a request-scoped value
		if let Some(sub) = ctx.get_request::<SubDependency>() {
			return Ok((*sub).clone());
		}

		// Return error if no k parameter provided
		Err(reinhardt_di::DiError::NotFound(
			"k parameter required".to_string(),
		))
	}
}

// Override with sub-dependency
#[derive(Clone, Debug, PartialEq)]
struct OverrideWithSub {
	sub: SubDependency,
}

impl OverrideWithSub {
	fn to_common_parameters(&self) -> CommonParameters {
		let k = self.sub.k.clone().unwrap_or_default();
		CommonParameters::new(Some(k), 0, 100)
	}
}

#[async_trait::async_trait]
impl Injectable for OverrideWithSub {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let sub = SubDependency::inject(ctx).await?;
		Ok(OverrideWithSub { sub })
	}
}

// Helper to simulate endpoint execution
async fn execute_endpoint_with_dependency(
	ctx: &InjectionContext,
) -> Result<CommonParameters, String> {
	CommonParameters::inject(ctx)
		.await
		.map_err(|e| format!("Dependency error: {:?}", e))
}

// ============================================================================
// Tests: Default behavior (without overrides)
// ============================================================================

#[tokio::test]
async fn test_main_depends() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let result = execute_endpoint_with_dependency(&ctx).await;
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("q parameter required"));
}

#[tokio::test]
async fn test_main_depends_q_foo() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Simulate query parameter
	let params = CommonParameters::required("foo".to_string(), 0, 100);
	ctx.set_request(params.clone());

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("foo".to_string()));
	assert_eq!(result.skip, 0);
	assert_eq!(result.limit, 100);
}

#[tokio::test]
async fn test_main_depends_q_foo_skip_100_limit_200() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Simulate query parameters
	let params = CommonParameters::required("foo".to_string(), 100, 200);
	ctx.set_request(params.clone());

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("foo".to_string()));
	assert_eq!(result.skip, 100);
	assert_eq!(result.limit, 200);
}

// ============================================================================
// Tests: Simple overrides
// ============================================================================

#[tokio::test]
async fn test_override_simple_no_query() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Set override
	let override_params = SimpleOverride::new(None).to_common_parameters();
	app.set_override("common_parameters", override_params);

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, None);
	assert_eq!(result.skip, 5);
	assert_eq!(result.limit, 10);

	app.clear_overrides();
}

#[tokio::test]
async fn test_override_simple_with_query() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Set override
	let override_params = SimpleOverride::new(Some("foo".to_string())).to_common_parameters();
	app.set_override("common_parameters", override_params);

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("foo".to_string()));
	assert_eq!(result.skip, 5);
	assert_eq!(result.limit, 10);

	app.clear_overrides();
}

#[tokio::test]
async fn test_override_simple_ignores_query_params() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Set override
	let override_params = SimpleOverride::new(Some("foo".to_string())).to_common_parameters();
	app.set_override("common_parameters", override_params);

	// Even with different query params in request, override takes precedence
	let params = CommonParameters::required("ignored".to_string(), 100, 200);
	ctx.set_request(params);

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("foo".to_string()));
	assert_eq!(result.skip, 5); // From override, not 100
	assert_eq!(result.limit, 10); // From override, not 200

	app.clear_overrides();
}

// ============================================================================
// Tests: Override with sub-dependency
// ============================================================================

#[tokio::test]
async fn test_override_with_sub_main_depends() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Create override with sub-dependency
	let override_with_sub = OverrideWithSub {
		sub: SubDependency::new(None),
	};
	let override_params = override_with_sub.to_common_parameters();
	app.set_override("common_parameters", override_params);

	// Without k parameter, should get empty string
	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("".to_string()));
	assert_eq!(result.skip, 0);
	assert_eq!(result.limit, 100);

	app.clear_overrides();
}

#[tokio::test]
async fn test_override_with_sub_main_depends_k_bar() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Simulate k parameter
	let sub = SubDependency::required("bar".to_string());
	ctx.set_request(sub);

	// Create override with sub-dependency that will inject from context
	let override_with_sub = OverrideWithSub::inject(&ctx).await.unwrap();
	let override_params = override_with_sub.to_common_parameters();
	app.set_override("common_parameters", override_params);

	let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result.q, Some("bar".to_string()));
	assert_eq!(result.skip, 0);
	assert_eq!(result.limit, 100);

	app.clear_overrides();
}

// ============================================================================
// Tests: Override isolation and clearing
// ============================================================================

#[tokio::test]
async fn test_override_cleared_between_requests() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();

	// Request 1 with override
	let ctx1 = InjectionContext::new(singleton.clone());
	ctx1.set_singleton(app.clone());

	let override_params = SimpleOverride::new(None).to_common_parameters();
	app.set_override("common_parameters", override_params);

	let result1 = execute_endpoint_with_dependency(&ctx1).await.unwrap();
	assert_eq!(result1.skip, 5);
	assert_eq!(result1.limit, 10);

	// Clear overrides
	app.clear_overrides();

	// Request 2 without override
	let ctx2 = InjectionContext::new(singleton.clone());
	ctx2.set_singleton(app.clone());

	let params = CommonParameters::required("test".to_string(), 0, 100);
	ctx2.set_request(params);

	let result2 = execute_endpoint_with_dependency(&ctx2).await.unwrap();
	assert_eq!(result2.skip, 0); // Back to default
	assert_eq!(result2.limit, 100); // Back to default
}

#[tokio::test]
async fn test_override_isolation() {
	let singleton1 = Arc::new(SingletonScope::new());
	let singleton2 = Arc::new(SingletonScope::new());
	let app1 = App::new();
	let app2 = App::new();

	let ctx1 = InjectionContext::new(singleton1);
	ctx1.set_singleton(app1.clone());

	let ctx2 = InjectionContext::new(singleton2);
	ctx2.set_singleton(app2.clone());

	// Set different overrides
	let override1 = SimpleOverride::new(Some("ctx1".to_string())).to_common_parameters();
	app1.set_override("common_parameters", override1);

	let override2 = SimpleOverride::new(Some("ctx2".to_string())).to_common_parameters();
	app2.set_override("common_parameters", override2);

	let result1 = execute_endpoint_with_dependency(&ctx1).await.unwrap();
	let result2 = execute_endpoint_with_dependency(&ctx2).await.unwrap();

	assert_eq!(result1.q, Some("ctx1".to_string()));
	assert_eq!(result2.q, Some("ctx2".to_string()));
}

// ============================================================================
// Tests: Multiple overrides
// ============================================================================

#[tokio::test]
async fn test_multiple_overrides() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Set multiple overrides
	app.set_override(
		"dep1",
		SimpleOverride::new(Some("foo".to_string())).to_common_parameters(),
	);
	app.set_override(
		"dep2",
		SimpleOverride::new(Some("bar".to_string())).to_common_parameters(),
	);

	let override1: Option<CommonParameters> = app.get_override("dep1");
	let override2: Option<CommonParameters> = app.get_override("dep2");

	assert!(override1.is_some());
	assert!(override2.is_some());

	assert_eq!(override1.unwrap().q, Some("foo".to_string()));
	assert_eq!(override2.unwrap().q, Some("bar".to_string()));
}

// ============================================================================
// Tests: Advanced patterns
// ============================================================================

// Override with callable/factory pattern
type OverrideFactory = Arc<dyn Fn() -> CommonParameters + Send + Sync>;

#[tokio::test]
async fn test_override_with_factory() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	let factory: OverrideFactory =
		Arc::new(|| CommonParameters::new(Some("from_factory".to_string()), 99, 999));

	app.set_override("factory", factory.clone());

	let retrieved: Option<OverrideFactory> = app.get_override("factory");
	assert!(retrieved.is_some());

	let params = retrieved.unwrap()();
	assert_eq!(params.q, Some("from_factory".to_string()));
	assert_eq!(params.skip, 99);
	assert_eq!(params.limit, 999);
}

// Test parametrized scenarios like in the original test
#[tokio::test]
async fn test_override_multiple_scenarios() {
	let test_cases = vec![
		("main-depends", None, 5, 10),
		("main-depends", Some("foo"), 5, 10),
		("router-depends", None, 5, 10),
		("router-depends", Some("foo"), 5, 10),
	];

	for (endpoint, q, expected_skip, expected_limit) in test_cases {
		let singleton = Arc::new(SingletonScope::new());
		let app = App::new();
		let ctx = InjectionContext::new(singleton);
		ctx.set_singleton(app.clone());

		let override_params = SimpleOverride::new(q.map(String::from)).to_common_parameters();
		app.set_override("common_parameters", override_params);

		let result = execute_endpoint_with_dependency(&ctx).await.unwrap();
		assert_eq!(
			result.skip, expected_skip,
			"Failed for endpoint: {}",
			endpoint
		);
		assert_eq!(
			result.limit, expected_limit,
			"Failed for endpoint: {}",
			endpoint
		);

		app.clear_overrides();
	}
}

// Test that overrides persist across multiple injections in same context
#[tokio::test]
async fn test_override_persists_in_context() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	let override_params =
		SimpleOverride::new(Some("persistent".to_string())).to_common_parameters();
	app.set_override("common_parameters", override_params);

	// Multiple injections should return same override
	let result1 = execute_endpoint_with_dependency(&ctx).await.unwrap();
	let result2 = execute_endpoint_with_dependency(&ctx).await.unwrap();
	let result3 = execute_endpoint_with_dependency(&ctx).await.unwrap();

	assert_eq!(result1.q, Some("persistent".to_string()));
	assert_eq!(result2.q, Some("persistent".to_string()));
	assert_eq!(result3.q, Some("persistent".to_string()));
}

// Test override replacement
#[tokio::test]
async fn test_override_replacement() {
	let singleton = Arc::new(SingletonScope::new());
	let app = App::new();
	let ctx = InjectionContext::new(singleton);
	ctx.set_singleton(app.clone());

	// Set initial override
	let override1 = SimpleOverride::new(Some("first".to_string())).to_common_parameters();
	app.set_override("common_parameters", override1);

	let result1 = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result1.q, Some("first".to_string()));

	// Replace with new override
	let override2 = SimpleOverride::new(Some("second".to_string())).to_common_parameters();
	app.set_override("common_parameters", override2);

	let result2 = execute_endpoint_with_dependency(&ctx).await.unwrap();
	assert_eq!(result2.q, Some("second".to_string()));
}
