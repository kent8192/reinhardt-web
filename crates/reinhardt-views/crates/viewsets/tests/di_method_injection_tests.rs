//! Tests for method-level dependency injection with #[inject]

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::{Request, Response, Result};
use reinhardt_di::{DiResult, InjectionContext, SingletonScope};
use reinhardt_macros::endpoint;
use reinhardt_viewsets::{Action, ViewSet};
use std::sync::Arc;

/// Mock email service dependency
#[derive(Clone)]
struct EmailService {
	smtp_host: String,
}

impl Default for EmailService {
	fn default() -> Self {
		Self {
			smtp_host: "smtp.example.com".to_string(),
		}
	}
}

/// Mock database dependency
#[derive(Clone)]
struct Database {
	connection_string: String,
}

impl Default for Database {
	fn default() -> Self {
		Self {
			connection_string: "postgres://localhost/testdb".to_string(),
		}
	}
}

/// ViewSet with method-level DI in custom action
#[derive(Clone)]
struct ProductViewSet {
	name: String,
}

impl ProductViewSet {
	fn new() -> Self {
		Self {
			name: "products".to_string(),
		}
	}

	/// Custom action with method-level DI
	#[endpoint]
	async fn send_notification_impl(
		&self,
		_request: Request,
		#[inject] email: EmailService,
	) -> DiResult<Response> {
		let body = format!(r#"{{"email_sent_via":"{}"}}"#, email.smtp_host);
		Ok(Response::ok().with_body(body))
	}

	async fn send_notification(
		&self,
		request: Request,
		ctx: &InjectionContext,
	) -> Result<Response> {
		self.send_notification_impl(request, ctx)
			.await
			.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
	}

	/// List action with method-level DI
	#[endpoint]
	async fn list_products_impl(
		&self,
		_request: Request,
		#[inject] db: Database,
	) -> DiResult<Response> {
		let body = format!(r#"{{"db":"{}"}}"#, db.connection_string);
		Ok(Response::ok().with_body(body))
	}

	async fn list_products(&self, request: Request, ctx: &InjectionContext) -> Result<Response> {
		self.list_products_impl(request, ctx)
			.await
			.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
	}

	/// Action with multiple injected dependencies
	#[endpoint]
	async fn create_and_notify_impl(
		&self,
		_request: Request,
		#[inject] db: Database,
		#[inject] email: EmailService,
	) -> DiResult<Response> {
		let body = format!(
			r#"{{"db":"{}","email":"{}"}}"#,
			db.connection_string, email.smtp_host
		);
		Ok(Response::ok().with_body(body))
	}

	async fn create_and_notify(
		&self,
		request: Request,
		ctx: &InjectionContext,
	) -> Result<Response> {
		self.create_and_notify_impl(request, ctx)
			.await
			.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
	}

	/// Action with mixed injected and regular parameters
	#[endpoint]
	async fn get_product_by_id_impl(
		&self,
		_request: Request,
		product_id: i64,
		#[inject] db: Database,
	) -> DiResult<Response> {
		let body = format!(
			r#"{{"product_id":{},"db":"{}"}}"#,
			product_id, db.connection_string
		);
		Ok(Response::ok().with_body(body))
	}

	async fn get_product_by_id(
		&self,
		request: Request,
		product_id: i64,
		ctx: &InjectionContext,
	) -> Result<Response> {
		self.get_product_by_id_impl(request, product_id, ctx)
			.await
			.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
	}
}

#[async_trait]
impl ViewSet for ProductViewSet {
	fn get_basename(&self) -> &str {
		&self.name
	}

	async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
		Ok(Response::ok())
	}
}

#[tokio::test]
async fn test_method_injection_single_dependency() {
	// Create DI context
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Create ViewSet
	let viewset = ProductViewSet::new();

	// Create request
	let request = Request::new(
		Method::POST,
		Uri::from_static("/products/notify/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// Call method with DI
	let response = viewset.send_notification(request, &ctx).await.unwrap();

	// Verify response
	assert_eq!(response.status, StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("smtp.example.com"));
}

#[tokio::test]
async fn test_method_injection_in_list_action() {
	// Create DI context
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Create ViewSet
	let viewset = ProductViewSet::new();

	// Create request
	let request = Request::new(
		Method::GET,
		Uri::from_static("/products/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// Call list method with DI
	let response = viewset.list_products(request, &ctx).await.unwrap();

	// Verify response
	assert_eq!(response.status, StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("postgres://localhost/testdb"));
}

#[tokio::test]
async fn test_method_injection_multiple_dependencies() {
	// Create DI context
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Create ViewSet
	let viewset = ProductViewSet::new();

	// Create request
	let request = Request::new(
		Method::POST,
		Uri::from_static("/products/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// Call method with multiple DI
	let response = viewset.create_and_notify(request, &ctx).await.unwrap();

	// Verify response
	assert_eq!(response.status, StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("postgres://localhost/testdb"));
	assert!(body.contains("smtp.example.com"));
}

#[tokio::test]
async fn test_method_injection_mixed_parameters() {
	// Create DI context
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Create ViewSet
	let viewset = ProductViewSet::new();

	// Create request
	let request = Request::new(
		Method::GET,
		Uri::from_static("/products/42/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// Call method with regular param + DI
	let response = viewset.get_product_by_id(request, 42, &ctx).await.unwrap();

	// Verify response
	assert_eq!(response.status, StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("\"product_id\":42"));
	assert!(body.contains("postgres://localhost/testdb"));
}

/// ViewSet with cache control on method injection
#[derive(Clone)]
struct CacheTestViewSet;

impl CacheTestViewSet {
	#[endpoint]
	async fn test_cache_control_impl(
		&self,
		_request: Request,
		#[inject] cached: Database,
		#[inject(cache = false)] fresh: Database,
	) -> DiResult<Response> {
		let body = format!(
			r#"{{"cached":"{}","fresh":"{}"}}"#,
			cached.connection_string, fresh.connection_string
		);
		Ok(Response::ok().with_body(body))
	}

	async fn test_cache_control(
		&self,
		request: Request,
		ctx: &InjectionContext,
	) -> Result<Response> {
		self.test_cache_control_impl(request, ctx)
			.await
			.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
	}
}

#[async_trait]
impl ViewSet for CacheTestViewSet {
	fn get_basename(&self) -> &str {
		"cache_test"
	}

	async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
		Ok(Response::ok())
	}
}

#[tokio::test]
async fn test_method_injection_cache_control() {
	// Create DI context
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Create ViewSet
	let viewset = CacheTestViewSet;

	// Create request
	let request = Request::new(
		Method::GET,
		Uri::from_static("/cache_test/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// Call method with cache control
	let response = viewset.test_cache_control(request, &ctx).await.unwrap();

	// Verify response
	assert_eq!(response.status, StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("postgres://localhost/testdb"));
}
