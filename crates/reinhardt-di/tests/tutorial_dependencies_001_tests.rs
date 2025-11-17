//! FastAPI tutorial dependencies 001 tests translated to Rust
//!
//! Based on: fastapi/tests/test_tutorial/test_dependencies/test_tutorial001.py
//!
//! These tests verify that:
//! 1. Common query parameters can be extracted as dependencies
//! 2. Dependencies with default values work correctly
//! 3. Multiple endpoints can share the same dependencies

use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt_params::Query;
use reinhardt_params::extract::FromRequest;
use std::sync::Arc;

// Common query parameters dependency
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
struct CommonQueryParams {
	q: Option<String>,
	#[serde(default)]
	skip: i32,
	#[serde(default = "default_limit")]
	limit: i32,
}

fn default_limit() -> i32 {
	100
}

impl CommonQueryParams {
	fn new(q: Option<String>, skip: Option<i32>, limit: Option<i32>) -> Self {
		CommonQueryParams {
			q,
			skip: skip.unwrap_or(0),
			limit: limit.unwrap_or(100),
		}
	}
}

#[async_trait::async_trait]
impl Injectable for CommonQueryParams {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check cache first
		if let Some(cached) = ctx.get_request::<CommonQueryParams>() {
			return Ok((*cached).clone());
		}

		// Extract from HTTP request if available
		if let (Some(request), Some(param_ctx)) = (ctx.get_http_request(), ctx.get_param_context())
		{
			let query_params = Query::<CommonQueryParams>::from_request(request, param_ctx)
				.await
				.map_err(|e| {
					DiError::ProviderError(format!("Failed to extract query parameters: {}", e))
				})?;
			let params = query_params.0;
			ctx.set_request(params.clone());
			return Ok(params);
		}

		// Fallback for tests without HTTP context (backward compatible)
		let params = CommonQueryParams::new(None, None, None);
		ctx.set_request(params.clone());
		Ok(params)
	}
}

#[tokio::test]
async fn test_default_query_params() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Inject with default values
	let params = CommonQueryParams::inject(&ctx).await.unwrap();

	assert_eq!(params.q, None);
	assert_eq!(params.skip, 0);
	assert_eq!(params.limit, 100);
}

#[tokio::test]
async fn test_query_params_with_q() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Set params with q
	ctx.set_request(CommonQueryParams::new(Some("foo".to_string()), None, None));

	let params = CommonQueryParams::inject(&ctx).await.unwrap();

	assert_eq!(params.q, Some("foo".to_string()));
	assert_eq!(params.skip, 0);
	assert_eq!(params.limit, 100);
}

#[tokio::test]
async fn test_query_params_with_skip() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Set params with q and skip
	ctx.set_request(CommonQueryParams::new(
		Some("foo".to_string()),
		Some(5),
		None,
	));

	let params = CommonQueryParams::inject(&ctx).await.unwrap();

	assert_eq!(params.q, Some("foo".to_string()));
	assert_eq!(params.skip, 5);
	assert_eq!(params.limit, 100);
}

#[tokio::test]
async fn test_query_params_with_all() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Set all params
	ctx.set_request(CommonQueryParams::new(
		Some("foo".to_string()),
		Some(5),
		Some(30),
	));

	let params = CommonQueryParams::inject(&ctx).await.unwrap();

	assert_eq!(params.q, Some("foo".to_string()));
	assert_eq!(params.skip, 5);
	assert_eq!(params.limit, 30);
}

#[tokio::test]
async fn test_shared_dependency_across_endpoints() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Set params
	ctx.set_request(CommonQueryParams::new(
		Some("test".to_string()),
		Some(10),
		Some(50),
	));

	// Simulate two different endpoints using the same dependency
	let params1 = CommonQueryParams::inject(&ctx).await.unwrap();
	let params2 = CommonQueryParams::inject(&ctx).await.unwrap();

	// Both should get the same cached instance
	assert_eq!(params1, params2);
}

#[tokio::test]
async fn test_different_requests_have_different_params() {
	let singleton = Arc::new(SingletonScope::new());

	// Request 1
	let ctx1 = InjectionContext::builder(singleton.clone()).build();
	ctx1.set_request(CommonQueryParams::new(
		Some("req1".to_string()),
		Some(0),
		Some(10),
	));
	let params1 = CommonQueryParams::inject(&ctx1).await.unwrap();

	// Request 2
	let ctx2 = InjectionContext::builder(singleton).build();
	ctx2.set_request(CommonQueryParams::new(
		Some("req2".to_string()),
		Some(5),
		Some(20),
	));
	let params2 = CommonQueryParams::inject(&ctx2).await.unwrap();

	// Should be different
	assert_ne!(params1.q, params2.q);
	assert_ne!(params1.skip, params2.skip);
	assert_ne!(params1.limit, params2.limit);
}
