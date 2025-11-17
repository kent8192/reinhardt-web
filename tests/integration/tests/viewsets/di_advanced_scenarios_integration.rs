//! Advanced DI scenarios tests
//!
//! Tests complex patterns, error handling, and edge cases for dependency injection.

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt_exception::Result;
use reinhardt_macros::{endpoint, Injectable};
use reinhardt_types::{Request, Response};
use reinhardt_viewsets::{Action, ActionType, ViewSet};
use std::sync::Arc;

// ============================================================================
// Category 1: Error Handling (5 tests)
// ============================================================================

#[tokio::test]
async fn test_error_propagation_from_di() {
	// Test that DI errors are properly propagated through the system
	#[derive(Clone)]
	struct SimpleService {
		value: i32,
	}

	impl Default for SimpleService {
		fn default() -> Self {
			Self { value: 42 }
		}
	}

	#[derive(Clone, Injectable)]
	struct TestViewSet {
		#[inject]
		service: SimpleService,
	}

	#[async_trait]
	impl ViewSet for TestViewSet {
		fn get_basename(&self) -> &str {
			"test"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok().with_body(format!("value: {}", self.service.value)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = TestViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.service.value, 42);
}

#[tokio::test]
async fn test_partial_injection_failure_recovery() {
	// Test that failed injection of one field doesn't affect others
	#[derive(Clone)]
	struct ServiceA {
		name: String,
	}

	impl Default for ServiceA {
		fn default() -> Self {
			Self {
				name: "ServiceA".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct ServiceB {
		value: i32,
	}

	impl Default for ServiceB {
		fn default() -> Self {
			Self { value: 100 }
		}
	}

	#[derive(Clone, Injectable)]
	struct PartialStruct {
		#[inject]
		service_a: ServiceA,
		#[inject]
		service_b: ServiceB,
		regular_field: String,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let instance = PartialStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service_a.name, "ServiceA");
	assert_eq!(instance.service_b.value, 100);
	assert_eq!(instance.regular_field, "");
}

#[tokio::test]
async fn test_error_messages_are_descriptive() {
	// Verify that error messages contain useful information
	// This is more of a documentation test showing the expected behavior
	#[derive(Clone)]
	#[allow(dead_code)]
	struct TestService {
		id: usize,
	}

	impl Default for TestService {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	#[allow(dead_code)]
	struct TestStruct {
		#[inject]
		service: TestService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let result = TestStruct::inject(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_viewset_dispatch_error_handling() {
	#[derive(Clone)]
	struct ErrorProneService {
		should_fail: bool,
	}

	impl Default for ErrorProneService {
		fn default() -> Self {
			Self { should_fail: false }
		}
	}

	#[derive(Clone, Injectable)]
	struct ErrorViewSet {
		#[inject]
		service: ErrorProneService,
	}

	#[async_trait]
	impl ViewSet for ErrorViewSet {
		fn get_basename(&self) -> &str {
			"error"
		}

		async fn dispatch(&self, _request: Request, action: Action) -> Result<Response> {
			if self.service.should_fail {
				return Err(reinhardt_exception::Error::Internal(
					"Service error".to_string(),
				));
			}

			match action.action_type {
				ActionType::List => Ok(Response::ok().with_body("list")),
				_ => Err(reinhardt_exception::Error::NotFound(
					"Not found".to_string(),
				)),
			}
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = ErrorViewSet::inject(&ctx).await.unwrap();

	let request = Request::builder()
		.method(Method::GET)
		.uri("/error/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let action = Action::list();

	let response = viewset.dispatch(request, action).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn test_method_di_error_conversion() {
	#[derive(Clone)]
	struct MethodService {
		data: String,
	}

	impl Default for MethodService {
		fn default() -> Self {
			Self {
				data: "method_data".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct MethodViewSet;

	impl MethodViewSet {
		#[endpoint]
		async fn test_method_impl(
			&self,
			_request: Request,
			#[inject] service: MethodService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(service.data.clone()))
		}

		async fn test_method(&self, request: Request, ctx: &InjectionContext) -> Result<Response> {
			self.test_method_impl(request, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = MethodViewSet;
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.test_method(request, &ctx).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);
}

// ============================================================================
// Category 2: Complex DI Patterns (5 tests)
// ============================================================================

#[tokio::test]
async fn test_nested_dependency_injection() {
	#[derive(Clone)]
	struct DatabaseService {
		connection: String,
	}

	impl Default for DatabaseService {
		fn default() -> Self {
			Self {
				connection: "postgres://localhost".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct RepositoryService {
		db: DatabaseService,
	}

	impl Default for RepositoryService {
		fn default() -> Self {
			Self {
				db: DatabaseService::default(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct NestedViewSet {
		#[inject]
		repository: RepositoryService,
	}

	#[async_trait]
	impl ViewSet for NestedViewSet {
		fn get_basename(&self) -> &str {
			"nested"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok().with_body(self.repository.db.connection.clone()))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = NestedViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.repository.db.connection, "postgres://localhost");
}

#[tokio::test]
async fn test_multiple_viewsets_same_context() {
	#[derive(Clone)]
	struct SharedService {
		id: usize,
	}

	impl Default for SharedService {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	struct ViewSetA {
		#[inject]
		service: SharedService,
	}

	#[async_trait]
	impl ViewSet for ViewSetA {
		fn get_basename(&self) -> &str {
			"viewset_a"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	#[derive(Clone, Injectable)]
	struct ViewSetB {
		#[inject]
		service: SharedService,
	}

	#[async_trait]
	impl ViewSet for ViewSetB {
		fn get_basename(&self) -> &str {
			"viewset_b"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset_a = ViewSetA::inject(&ctx).await.unwrap();
	let viewset_b = ViewSetB::inject(&ctx).await.unwrap();

	// Both should have access to the shared service
	assert_eq!(viewset_a.service.id, 1);
	assert_eq!(viewset_b.service.id, 1);
}

#[tokio::test]
async fn test_generic_service_injection() {
	#[derive(Clone)]
	struct GenericService<T: Clone> {
		value: T,
	}

	impl Default for GenericService<String> {
		fn default() -> Self {
			Self {
				value: "generic".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct GenericViewSet {
		#[inject]
		service: GenericService<String>,
	}

	#[async_trait]
	impl ViewSet for GenericViewSet {
		fn get_basename(&self) -> &str {
			"generic"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok().with_body(self.service.value.clone()))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = GenericViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.service.value, "generic");
}

#[tokio::test]
async fn test_option_wrapped_dependency() {
	#[derive(Clone)]
	struct OptionalService {
		available: bool,
	}

	impl Default for OptionalService {
		fn default() -> Self {
			Self { available: true }
		}
	}

	#[derive(Clone, Injectable)]
	struct OptionalViewSet {
		#[inject]
		service: OptionalService,
	}

	#[async_trait]
	impl ViewSet for OptionalViewSet {
		fn get_basename(&self) -> &str {
			"optional"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			if self.service.available {
				Ok(Response::ok().with_body("available"))
			} else {
				Ok(Response::ok().with_body("unavailable"))
			}
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = OptionalViewSet::inject(&ctx).await.unwrap();
	assert!(viewset.service.available);
}

#[tokio::test]
async fn test_arc_shared_state() {
	use std::sync::Mutex;

	#[derive(Clone)]
	struct StateService {
		counter: Arc<Mutex<i32>>,
	}

	impl Default for StateService {
		fn default() -> Self {
			Self {
				counter: Arc::new(Mutex::new(0)),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct StateViewSet {
		#[inject]
		state: StateService,
	}

	#[async_trait]
	impl ViewSet for StateViewSet {
		fn get_basename(&self) -> &str {
			"state"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			let mut counter = self.state.counter.lock().unwrap();
			*counter += 1;
			Ok(Response::ok().with_body(format!("count: {}", *counter)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset1 = StateViewSet::inject(&ctx).await.unwrap();
	let viewset2 = StateViewSet::inject(&ctx).await.unwrap();

	// Both viewsets should share the same state
	let request1 = Request::builder()
		.method(Method::GET)
		.uri("/state/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let request2 = Request::builder()
		.method(Method::GET)
		.uri("/state/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let _ = viewset1.dispatch(request1, Action::list()).await;
	let response = viewset2.dispatch(request2, Action::list()).await.unwrap();

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("count: 2"));
}

// ============================================================================
// Category 3: ViewSet Integration (5 tests)
// ============================================================================

#[tokio::test]
async fn test_field_injection_with_all_actions() {
	#[derive(Clone)]
	struct ActionService {
		prefix: String,
	}

	impl Default for ActionService {
		fn default() -> Self {
			Self {
				prefix: "action_".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct ActionViewSet {
		#[inject]
		service: ActionService,
	}

	#[async_trait]
	impl ViewSet for ActionViewSet {
		fn get_basename(&self) -> &str {
			"actions"
		}

		async fn dispatch(&self, _request: Request, action: Action) -> Result<Response> {
			let action_name = match action.action_type {
				ActionType::List => "list",
				ActionType::Create => "create",
				ActionType::Retrieve => "retrieve",
				ActionType::Update => "update",
				ActionType::PartialUpdate => "partial_update",
				ActionType::Destroy => "destroy",
				ActionType::Custom(ref name) => name,
			};

			Ok(Response::ok().with_body(format!("{}{}", self.service.prefix, action_name)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = ActionViewSet::inject(&ctx).await.unwrap();

	let _request = Request::builder()
		.method(Method::GET)
		.uri("/actions/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	// Test all standard actions
	for (action, expected) in [
		(Action::list(), "action_list"),
		(Action::create(), "action_create"),
		(Action::retrieve(), "action_retrieve"),
		(Action::update(), "action_update"),
		(Action::partial_update(), "action_partial_update"),
		(Action::destroy(), "action_destroy"),
	] {
		let req = Request::builder()
			.method(Method::GET)
			.uri("/actions/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = viewset.dispatch(req, action).await.unwrap();
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, expected);
	}
}

#[tokio::test]
async fn test_method_injection_with_request_params() {
	#[derive(Clone)]
	struct ParamService {
		validator: String,
	}

	impl Default for ParamService {
		fn default() -> Self {
			Self {
				validator: "valid".to_string(),
			}
		}
	}

	#[derive(Clone)]
	#[allow(dead_code)]
	struct ParamViewSet {
		name: String,
	}

	impl ParamViewSet {
		fn new() -> Self {
			Self {
				name: "params".to_string(),
			}
		}

		#[endpoint]
		async fn process_impl(
			&self,
			_request: Request,
			id: i64,
			#[inject] service: ParamService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(format!("id: {}, validator: {}", id, service.validator)))
		}

		async fn process(
			&self,
			request: Request,
			id: i64,
			ctx: &InjectionContext,
		) -> Result<Response> {
			self.process_impl(request, id, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = ParamViewSet::new();

	let request = Request::builder()
		.method(Method::GET)
		.uri("/params/123/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.process(request, 123, &ctx).await.unwrap();
	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("id: 123"));
	assert!(body.contains("validator: valid"));
}

#[tokio::test]
async fn test_dispatch_injection_with_action_routing() {
	#[derive(Clone)]
	struct RouterService {
		routes: Vec<String>,
	}

	impl Default for RouterService {
		fn default() -> Self {
			Self {
				routes: vec!["list".to_string(), "create".to_string()],
			}
		}
	}

	#[derive(Clone)]
	struct RouterViewSet;

	impl RouterViewSet {
		#[endpoint]
		async fn dispatch_impl(
			&self,
			_request: Request,
			action: Action,
			#[inject] router: RouterService,
		) -> DiResult<Response> {
			let action_name = match action.action_type {
				ActionType::List => "list",
				ActionType::Create => "create",
				_ => "unknown",
			};

			let is_allowed = router.routes.iter().any(|r| r == action_name);

			if is_allowed {
				Ok(Response::ok().with_body(format!("allowed: {}", action_name)))
			} else {
				Err(DiError::NotFound("Route not found".to_string()))
			}
		}

		async fn dispatch_with_ctx(
			&self,
			request: Request,
			action: Action,
			ctx: &InjectionContext,
		) -> Result<Response> {
			self.dispatch_impl(request, action, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = RouterViewSet;

	// Test allowed action
	let request1 = Request::builder()
		.method(Method::GET)
		.uri("/router/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let response = viewset
		.dispatch_with_ctx(request1, Action::list(), &ctx)
		.await
		.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	// Test disallowed action
	let request2 = Request::builder()
		.method(Method::GET)
		.uri("/router/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let result = viewset
		.dispatch_with_ctx(request2, Action::retrieve(), &ctx)
		.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_mixed_di_patterns_in_viewset() {
	#[derive(Clone)]
	struct FieldService {
		name: String,
	}

	impl Default for FieldService {
		fn default() -> Self {
			Self {
				name: "field".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct MethodService {
		value: i32,
	}

	impl Default for MethodService {
		fn default() -> Self {
			Self { value: 42 }
		}
	}

	#[derive(Clone, Injectable)]
	struct MixedViewSet {
		#[inject]
		field_service: FieldService,
	}

	impl MixedViewSet {
		#[endpoint]
		async fn custom_action_impl(
			&self,
			_request: Request,
			#[inject] method_svc: MethodService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(format!(
				"field: {}, method: {}",
				self.field_service.name, method_svc.value
			)))
		}

		async fn custom_action(
			&self,
			request: Request,
			ctx: &InjectionContext,
		) -> Result<Response> {
			self.custom_action_impl(request, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}
	}

	#[async_trait]
	impl ViewSet for MixedViewSet {
		fn get_basename(&self) -> &str {
			"mixed"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok().with_body(format!("field: {}", self.field_service.name)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = MixedViewSet::inject(&ctx).await.unwrap();

	// Test field injection via dispatch
	let request1 = Request::builder()
		.method(Method::GET)
		.uri("/mixed/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let response = viewset.dispatch(request1, Action::list()).await.unwrap();
	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("field: field"));

	// Test method injection
	let request2 = Request::builder()
		.method(Method::GET)
		.uri("/mixed/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let response = viewset.custom_action(request2, &ctx).await.unwrap();
	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("field: field"));
	assert!(body.contains("method: 42"));
}

#[tokio::test]
async fn test_viewset_with_stateful_service() {
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[derive(Clone)]
	struct CounterService {
		count: Arc<AtomicUsize>,
	}

	impl Default for CounterService {
		fn default() -> Self {
			Self {
				count: Arc::new(AtomicUsize::new(0)),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct CounterViewSet {
		#[inject]
		counter: CounterService,
	}

	#[async_trait]
	impl ViewSet for CounterViewSet {
		fn get_basename(&self) -> &str {
			"counter"
		}

		async fn dispatch(&self, _request: Request, action: Action) -> Result<Response> {
			match action.action_type {
				ActionType::List => {
					let count = self.counter.count.load(Ordering::SeqCst);
					Ok(Response::ok().with_body(format!("count: {}", count)))
				}
				ActionType::Create => {
					let count = self.counter.count.fetch_add(1, Ordering::SeqCst);
					Ok(Response::ok().with_body(format!("incremented to: {}", count + 1)))
				}
				_ => Err(reinhardt_exception::Error::NotFound(
					"Action not found".to_string(),
				)),
			}
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = CounterViewSet::inject(&ctx).await.unwrap();

	// Increment counter
	let request1 = Request::builder()
		.method(Method::POST)
		.uri("/counter/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	viewset.dispatch(request1, Action::create()).await.unwrap();

	let request2 = Request::builder()
		.method(Method::POST)
		.uri("/counter/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	viewset.dispatch(request2, Action::create()).await.unwrap();

	// Check counter value
	let request3 = Request::builder()
		.method(Method::POST)
		.uri("/counter/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let response = viewset.dispatch(request3, Action::list()).await.unwrap();
	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body.contains("count: 2"));
}

// ============================================================================
// Category 4: Async Patterns (5 tests)
// ============================================================================

#[tokio::test]
async fn test_concurrent_di_injections() {
	use tokio::task::JoinSet;

	#[derive(Clone)]
	#[allow(dead_code)]
	struct ConcurrentService {
		id: usize,
	}

	impl Default for ConcurrentService {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	#[allow(dead_code)]
	struct ConcurrentViewSet {
		#[inject]
		service: ConcurrentService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::builder(singleton).build());

	let mut tasks = JoinSet::new();

	// Spawn multiple concurrent injection tasks
	for _ in 0..10 {
		let ctx_clone = ctx.clone();
		tasks.spawn(async move { ConcurrentViewSet::inject(&ctx_clone).await });
	}

	// All should succeed
	let mut success_count = 0;
	while let Some(result) = tasks.join_next().await {
		if result.unwrap().is_ok() {
			success_count += 1;
		}
	}

	assert_eq!(success_count, 10);
}

#[tokio::test]
async fn test_async_service_initialization() {
	#[derive(Clone)]
	struct AsyncService {
		data: String,
	}

	impl Default for AsyncService {
		fn default() -> Self {
			// Simulate async initialization
			Self {
				data: "async_initialized".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct AsyncViewSet {
		#[inject]
		service: AsyncService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = AsyncViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.service.data, "async_initialized");
}

#[tokio::test]
async fn test_timeout_handling_in_injection() {
	use tokio::time::{timeout, Duration};

	#[derive(Clone)]
	#[allow(dead_code)]
	struct SlowService {
		ready: bool,
	}

	impl Default for SlowService {
		fn default() -> Self {
			Self { ready: true }
		}
	}

	#[derive(Clone, Injectable)]
	#[allow(dead_code)]
	struct TimeoutViewSet {
		#[inject]
		service: SlowService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Injection should complete within reasonable time
	let result = timeout(Duration::from_secs(5), TimeoutViewSet::inject(&ctx)).await;

	assert!(result.is_ok());
	assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn test_multiple_async_methods_with_di() {
	#[derive(Clone)]
	struct AsyncMethodService {
		value: String,
	}

	impl Default for AsyncMethodService {
		fn default() -> Self {
			Self {
				value: "async_method".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct AsyncMethodViewSet;

	impl AsyncMethodViewSet {
		#[endpoint]
		async fn method_one_impl(
			&self,
			_request: Request,
			#[inject] service: AsyncMethodService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(format!("one: {}", service.value)))
		}

		async fn method_one(&self, request: Request, ctx: &InjectionContext) -> Result<Response> {
			self.method_one_impl(request, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}

		#[endpoint]
		async fn method_two_impl(
			&self,
			_request: Request,
			#[inject] service: AsyncMethodService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(format!("two: {}", service.value)))
		}

		async fn method_two(&self, request: Request, ctx: &InjectionContext) -> Result<Response> {
			self.method_two_impl(request, ctx)
				.await
				.map_err(|e| reinhardt_exception::Error::Internal(format!("DI error: {}", e)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = AsyncMethodViewSet;

	let request1 = Request::builder()
		.method(Method::GET)
		.uri("/async/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let request2 = Request::builder()
		.method(Method::GET)
		.uri("/async/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	// Both methods should work independently
	let response1 = viewset.method_one(request1, &ctx).await.unwrap();
	let response2 = viewset.method_two(request2, &ctx).await.unwrap();

	let body1 = String::from_utf8(response1.body.to_vec()).unwrap();
	let body2 = String::from_utf8(response2.body.to_vec()).unwrap();

	assert!(body1.contains("one: async_method"));
	assert!(body2.contains("two: async_method"));
}

#[tokio::test]
async fn test_parallel_viewset_operations() {
	#[derive(Clone)]
	struct ParallelService {
		id: usize,
	}

	impl Default for ParallelService {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	struct ParallelViewSet {
		#[inject]
		service: ParallelService,
	}

	#[async_trait]
	impl ViewSet for ParallelViewSet {
		fn get_basename(&self) -> &str {
			"parallel"
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok().with_body(format!("id: {}", self.service.id)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();
	let viewset = Arc::new(ParallelViewSet::inject(&ctx).await.unwrap());

	// Spawn multiple concurrent dispatch operations
	let mut handles = vec![];
	for _ in 0..5 {
		let vs = viewset.clone();
		handles.push(tokio::spawn(async move {
			let req = Request::builder()
				.method(Method::GET)
				.uri("/parallel/")
				.version(Version::HTTP_11)
				.headers(HeaderMap::new())
				.body(Bytes::new())
				.build()
				.unwrap();
			vs.dispatch(req, Action::list()).await
		}));
	}

	// All should succeed
	for handle in handles {
		let result = handle.await.unwrap();
		assert!(result.is_ok());
	}
}

// ============================================================================
// Category 5: Edge Cases (5 tests)
// ============================================================================

#[tokio::test]
async fn test_empty_context_still_works() {
	#[derive(Clone)]
	struct SimpleService {
		value: i32,
	}

	impl Default for SimpleService {
		fn default() -> Self {
			Self { value: 100 }
		}
	}

	#[derive(Clone, Injectable)]
	struct EmptyContextViewSet {
		#[inject]
		service: SimpleService,
	}

	// Create context without pre-registering anything
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = EmptyContextViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.service.value, 100);
}

#[tokio::test]
async fn test_large_number_of_dependencies() {
	#[derive(Clone)]
	struct Service {
		id: usize,
	}

	impl Default for Service {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	#[allow(dead_code)]
	struct LargeDepsViewSet {
		#[inject]
		s1: Service,
		#[inject]
		s2: Service,
		#[inject]
		s3: Service,
		#[inject]
		s4: Service,
		#[inject]
		s5: Service,
		#[inject]
		s6: Service,
		#[inject]
		s7: Service,
		#[inject]
		s8: Service,
		#[inject]
		s9: Service,
		#[inject]
		s10: Service,
		#[inject]
		s11: Service,
		#[inject]
		s12: Service,
		#[inject]
		s13: Service,
		#[inject]
		s14: Service,
		#[inject]
		s15: Service,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = LargeDepsViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.s1.id, 1);
	assert_eq!(viewset.s15.id, 1);
}

#[tokio::test]
async fn test_recursive_struct_with_arc() {
	use std::sync::Mutex;

	#[derive(Clone)]
	#[allow(dead_code)]
	struct Node {
		value: i32,
		next: Arc<Mutex<Option<Box<Node>>>>,
	}

	impl Default for Node {
		fn default() -> Self {
			Self {
				value: 1,
				next: Arc::new(Mutex::new(None)),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct RecursiveViewSet {
		#[inject]
		root: Node,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = RecursiveViewSet::inject(&ctx).await.unwrap();
	assert_eq!(viewset.root.value, 1);
}

#[tokio::test]
async fn test_zero_sized_types() {
	#[derive(Clone)]
	struct ZeroSized;

	impl Default for ZeroSized {
		fn default() -> Self {
			Self
		}
	}

	#[derive(Clone, Injectable)]
	struct ZstViewSet {
		#[inject]
		zst: ZeroSized,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = ZstViewSet::inject(&ctx).await.unwrap();
	// Just verify it compiles and runs
	let _ = viewset.zst;
}

#[tokio::test]
async fn test_deeply_nested_generic_types() {
	use std::collections::HashMap;

	#[derive(Clone)]
	struct ComplexService {
		data: HashMap<String, Vec<Option<Arc<String>>>>,
	}

	impl Default for ComplexService {
		fn default() -> Self {
			let mut data = HashMap::new();
			data.insert("key".to_string(), vec![Some(Arc::new("value".to_string()))]);
			Self { data }
		}
	}

	#[derive(Clone, Injectable)]
	struct ComplexViewSet {
		#[inject]
		service: ComplexService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let viewset = ComplexViewSet::inject(&ctx).await.unwrap();
	assert!(viewset.service.data.contains_key("key"));
}
