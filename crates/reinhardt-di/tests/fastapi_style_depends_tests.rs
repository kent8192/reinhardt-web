//! Tests for FastAPI-style Depends functionality

#![cfg(all(feature = "macros", feature = "testing"))]

use reinhardt_di::{
	DependencyScope, Depends, FactoryOutput, Injectable, InjectableKey, InjectionContext,
	OverrideGuard, SingletonScope, global_registry,
};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CommonQueryParamsKey;

impl InjectableKey for CommonQueryParamsKey {}

#[derive(Clone, Default, Debug, PartialEq)]
struct CommonQueryParams {
	q: Option<String>,
	skip: usize,
	limit: usize,
}

fn register_common_query_params() -> OverrideGuard {
	let registry = global_registry();
	registry.register_override::<FactoryOutput<CommonQueryParamsKey, CommonQueryParams>, _, _>(
		DependencyScope::Request,
		|_ctx| async { Ok(FactoryOutput::new(CommonQueryParams::default())) },
	)
}

struct DatabaseKey;

impl InjectableKey for DatabaseKey {}

#[derive(Clone, Default)]
struct Database {
	connection_count: usize,
}

struct ConfigKey;

impl InjectableKey for ConfigKey {}

#[derive(Clone, Default)]
struct Config {
	api_key: String,
}

struct CountedServiceKey;

impl InjectableKey for CountedServiceKey {}

// Custom Injectable with instance counter (thread-safe using AtomicUsize)
static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct CountedService {
	instance_id: usize,
}

#[async_trait::async_trait]
impl Injectable for CountedService {
	async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(CountedService { instance_id })
	}
}

/// Register CountedService output so `Depends<K, T>` can resolve it from the registry.
/// Uses Request scope: same instance within one `InjectionContext`, new instance per context.
fn register_counted_service() -> OverrideGuard {
	let registry = global_registry();
	registry.register_override::<FactoryOutput<CountedServiceKey, CountedService>, _, _>(
		DependencyScope::Request,
		|_ctx| async {
			let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
			Ok(FactoryOutput::new(CountedService { instance_id }))
		},
	)
}

fn register_config() -> OverrideGuard {
	let registry = global_registry();
	registry.register_override::<FactoryOutput<ConfigKey, Config>, _, _>(
		DependencyScope::Request,
		|_ctx| async {
			Ok(FactoryOutput::new(Config {
				api_key: String::new(),
			}))
		},
	)
}

#[tokio::test]
#[serial(di_registry)]
async fn test_injected_with_cache_default() {
	let _guard = register_common_query_params();
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Cache is enabled by default
	let params1 =
		Depends::<CommonQueryParamsKey, CommonQueryParams>::resolve_from_registry(&ctx, true)
			.await
			.unwrap();
	let params2 =
		Depends::<CommonQueryParamsKey, CommonQueryParams>::resolve_from_registry(&ctx, true)
			.await
			.unwrap();

	// Returns the same instance
	assert_eq!(*params1, *params2);
	assert!(std::sync::Arc::ptr_eq(params1.as_arc(), params2.as_arc()));
}

#[tokio::test]
#[serial(di_registry)]
async fn test_separate_contexts_create_new_instances() {
	// Reset counter for this test
	INSTANCE_COUNTER.store(0, Ordering::SeqCst);
	let _guard = register_counted_service();

	// With Request scope, separate InjectionContexts produce separate instances
	let singleton = SingletonScope::new();
	let ctx1 = InjectionContext::builder(singleton).build();
	let singleton2 = SingletonScope::new();
	let ctx2 = InjectionContext::builder(singleton2).build();

	let service1 = Depends::<CountedServiceKey, CountedService>::resolve_from_registry(&ctx1, true)
		.await
		.unwrap();
	let service2 = Depends::<CountedServiceKey, CountedService>::resolve_from_registry(&ctx2, true)
		.await
		.unwrap();

	// Different contexts produce different instances (IDs are sequential)
	assert_ne!(service1.instance_id, service2.instance_id);
	assert_eq!(service1.instance_id + 1, service2.instance_id);
}

#[tokio::test]
#[serial(di_registry)]
async fn test_injected_with_cache_enabled() {
	// Reset counter for this test
	INSTANCE_COUNTER.store(0, Ordering::SeqCst);
	let _guard = register_counted_service();

	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Cache enabled (default)
	let service1 = Depends::<CountedServiceKey, CountedService>::resolve_from_registry(&ctx, true)
		.await
		.unwrap();
	let service2 = Depends::<CountedServiceKey, CountedService>::resolve_from_registry(&ctx, true)
		.await
		.unwrap();

	// Returns the same instance (same ID)
	assert_eq!(service1.instance_id, service2.instance_id);
	assert!(std::sync::Arc::ptr_eq(service1.as_arc(), service2.as_arc()));
}

#[tokio::test]
async fn test_injected_from_value() {
	let db = Database {
		connection_count: 10,
	};
	let depends = Depends::<DatabaseKey, Database>::from_value(db);

	assert_eq!(depends.connection_count, 10);
}

#[tokio::test]
#[serial(di_registry)]
async fn test_injected_deref() {
	let _guard = register_common_query_params();
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let params =
		Depends::<CommonQueryParamsKey, CommonQueryParams>::resolve_from_registry(&ctx, true)
			.await
			.unwrap();

	// Can access fields directly via Deref
	assert_eq!(params.skip, 0);
	assert_eq!(params.limit, 0);
}

#[tokio::test]
#[serial(di_registry)]
async fn test_fastapi_injected_clone() {
	let _guard = register_common_query_params();
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let params1 =
		Depends::<CommonQueryParamsKey, CommonQueryParams>::resolve_from_registry(&ctx, true)
			.await
			.unwrap();
	let params2 = params1.clone();

	// Clone copies the reference (Arc::clone)
	assert_eq!(*params1, *params2);
	assert!(std::sync::Arc::ptr_eq(params1.as_arc(), params2.as_arc()));
}

// FastAPI-style usage example
#[tokio::test]
#[serial(di_registry)]
async fn test_fastapi_style_usage() {
	async fn endpoint_handler(
		config: Depends<ConfigKey, Config>,
		params: Depends<CommonQueryParamsKey, CommonQueryParams>,
	) -> String {
		format!("API Key: {}, Skip: {}", config.api_key, params.skip)
	}

	let _config_guard = register_config();
	let _params_guard = register_common_query_params();
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Simulate endpoint usage
	let config = Depends::<ConfigKey, Config>::resolve_from_registry(&ctx, true)
		.await
		.unwrap();
	let params =
		Depends::<CommonQueryParamsKey, CommonQueryParams>::resolve_from_registry(&ctx, true)
			.await
			.unwrap();

	let result = endpoint_handler(config, params).await;
	assert_eq!(result, "API Key: , Skip: 0");
}
