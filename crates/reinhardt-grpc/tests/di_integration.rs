//! Integration tests for gRPC dependency injection
//!
//! These tests verify that the `#[grpc_handler]` macro correctly integrates
//! with the DI system.

#![cfg(feature = "di")]

use reinhardt_di::{
	DependencyScope, Depends, DiError, Injectable, InjectionContext, KeyedFactoryOutput, SelfKey,
	SingletonScope, global_registry,
};
use reinhardt_grpc::grpc_handler;
use serial_test::serial;
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status};

/// Mock database connection for testing
#[derive(Clone)]
struct MockDatabase {
	calls: Arc<Mutex<Vec<String>>>,
}

impl MockDatabase {
	fn new() -> Self {
		Self {
			calls: Arc::new(Mutex::new(Vec::new())),
		}
	}

	fn mark_used(&mut self) {
		self.calls.lock().unwrap().push("used".to_string());
	}

	async fn fetch_user(&self, user_id: &str) -> Result<String, String> {
		self.calls
			.lock()
			.unwrap()
			.push(format!("fetch_user({})", user_id));
		Ok(format!("User:{}", user_id))
	}
}

#[async_trait::async_trait]
impl Injectable for MockDatabase {
	async fn inject(_ctx: &InjectionContext) -> Result<Self, DiError> {
		Ok(MockDatabase::new())
	}
}

/// Mock cache for testing
#[derive(Clone)]
struct MockCache {
	calls: Arc<Mutex<Vec<String>>>,
}

impl MockCache {
	fn new() -> Self {
		Self {
			calls: Arc::new(Mutex::new(Vec::new())),
		}
	}

	fn mark_used(&mut self) {
		self.calls.lock().unwrap().push("used".to_string());
	}

	async fn get(&self, key: &str) -> Option<String> {
		self.calls.lock().unwrap().push(format!("get({})", key));
		None
	}
}

#[derive(Clone)]
struct Wrapper<T>(T);

#[async_trait::async_trait]
impl Injectable for Wrapper<MockCache> {
	async fn inject(ctx: &InjectionContext) -> Result<Self, DiError> {
		Ok(Self(MockCache::inject(ctx).await?))
	}
}

#[async_trait::async_trait]
impl Injectable for MockCache {
	async fn inject(_ctx: &InjectionContext) -> Result<Self, DiError> {
		Ok(MockCache::new())
	}
}

/// Mock request type
#[derive(Debug)]
struct GetUserRequest {
	id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProviderConfig {
	prefix: &'static str,
}

/// Test service implementation
struct TestService {}

impl TestService {
	#[grpc_handler]
	async fn get_user(
		&self,
		request: Request<GetUserRequest>,
		#[inject] db: MockDatabase,
	) -> Result<Response<String>, Status> {
		let user_id = &request.into_inner().id;
		let user = db.fetch_user(user_id).await.map_err(Status::not_found)?;
		Ok(Response::new(user))
	}

	#[grpc_handler]
	async fn get_user_with_cache(
		&self,
		request: Request<GetUserRequest>,
		#[inject] db: MockDatabase,
		#[inject] cache: MockCache,
	) -> Result<Response<String>, Status> {
		let user_id = &request.into_inner().id;

		// Check cache first
		if let Some(user) = cache.get(user_id).await {
			return Ok(Response::new(user));
		}

		// Fetch from database
		let user = db.fetch_user(user_id).await.map_err(Status::not_found)?;
		Ok(Response::new(user))
	}

	#[grpc_handler]
	async fn get_user_uncached(
		&self,
		request: Request<GetUserRequest>,
		#[inject(cache = false)] db: MockDatabase,
	) -> Result<Response<String>, Status> {
		let user_id = &request.into_inner().id;
		let user = db.fetch_user(user_id).await.map_err(Status::not_found)?;
		Ok(Response::new(user))
	}

	#[grpc_handler]
	async fn get_provider_config(
		&self,
		request: Request<GetUserRequest>,
		#[inject] config: Depends<ProviderConfig>,
	) -> Result<Response<String>, Status> {
		let user_id = request.into_inner().id;
		Ok(Response::new(format!("{}:{}", config.prefix, user_id)))
	}

	#[grpc_handler]
	async fn get_mutable_user(
		&self,
		__reinhardt_injected_0: Request<GetUserRequest>,
		#[inject] mut db: MockDatabase,
		#[inject] Wrapper(mut cache): Wrapper<MockCache>,
	) -> Result<Response<String>, Status> {
		db.mark_used();
		cache.mark_used();
		let user = db
			.fetch_user(&__reinhardt_injected_0.into_inner().id)
			.await
			.map_err(Status::not_found)?;
		Ok(Response::new(user))
	}
}

#[tokio::test]
async fn test_grpc_handler_forwards_mutable_and_destructured_dependencies() {
	let ctx = Arc::new(InjectionContext::builder(SingletonScope::new()).build());
	let service = TestService {};
	let mut request = Request::new(GetUserRequest {
		id: "pattern".to_string(),
	});
	request.extensions_mut().insert(ctx);

	let response = service
		.get_mutable_user(request)
		.await
		.expect("mutable and destructured dependencies should be forwarded");

	assert_eq!(response.into_inner(), "User:pattern");
}

#[tokio::test]
async fn test_grpc_handler_basic_di() {
	// Setup
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	let service = TestService {};

	// Create request with DI context
	let mut request = Request::new(GetUserRequest {
		id: "123".to_string(),
	});
	request.extensions_mut().insert(ctx.clone());

	// Call handler
	let response = service.get_user(request).await;

	// Verify
	assert!(response.is_ok());
	let user = response.unwrap().into_inner();
	assert_eq!(user, "User:123");
}

#[tokio::test]
async fn test_grpc_handler_multiple_dependencies() {
	// Setup
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	let service = TestService {};

	// Create request with DI context
	let mut request = Request::new(GetUserRequest {
		id: "456".to_string(),
	});
	request.extensions_mut().insert(ctx.clone());

	// Call handler
	let response = service.get_user_with_cache(request).await;

	// Verify
	assert!(response.is_ok());
	let user = response.unwrap().into_inner();
	assert_eq!(user, "User:456");
}

#[tokio::test]
async fn test_grpc_handler_missing_di_context() {
	// Setup
	let singleton_scope = Arc::new(SingletonScope::new());
	let _ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	let service = TestService {};

	// Create request WITHOUT DI context
	let request = Request::new(GetUserRequest {
		id: "789".to_string(),
	});

	// Call handler - should fail
	let response = service.get_user(request).await;

	// Verify
	assert!(response.is_err());
	let status = response.unwrap_err();
	assert_eq!(status.code(), tonic::Code::Internal);
	// Security: error message should be generic, not exposing DI internals
	assert_eq!(status.message(), "Internal server error");
}

#[tokio::test]
async fn test_grpc_handler_cache_control() {
	// Setup
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	let service = TestService {};

	// Create first request with DI context
	let mut request1 = Request::new(GetUserRequest {
		id: "111".to_string(),
	});
	request1.extensions_mut().insert(ctx.clone());

	// Call handler first time
	let response1 = service.get_user_uncached(request1).await;
	assert!(response1.is_ok());

	// Create second request with DI context
	let mut request2 = Request::new(GetUserRequest {
		id: "222".to_string(),
	});
	request2.extensions_mut().insert(ctx.clone());

	// Call handler second time
	let response2 = service.get_user_uncached(request2).await;
	assert!(response2.is_ok());
}

#[serial(di_registry)]
#[tokio::test]
async fn test_grpc_handler_self_keyed_depends() {
	let registry = global_registry();
	registry.register_async::<KeyedFactoryOutput<SelfKey<ProviderConfig>, ProviderConfig>, _, _>(
		DependencyScope::Request,
		|_ctx| async {
			Ok(KeyedFactoryOutput::new(ProviderConfig {
				prefix: "provider",
			}))
		},
	);
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	let service = TestService {};

	let mut request = Request::new(GetUserRequest {
		id: "123".to_string(),
	});
	request.extensions_mut().insert(ctx);

	let response = service
		.get_provider_config(request)
		.await
		.expect("self-keyed Depends<T> should resolve in grpc_handler");

	assert_eq!(response.into_inner(), "provider:123");
}
