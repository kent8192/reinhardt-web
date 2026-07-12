//! Unit tests for Depends<K, T> and DependsBuilder

use async_trait::async_trait;
use reinhardt_di::{
	DependencyRegistry, DependencyScope, Depends, DiResult, FactoryOutput, Injectable,
	InjectableKey, InjectionContext,
};
use reinhardt_test::fixtures::*;
use rstest::*;
use std::sync::Arc;
use std::sync::Once;
use std::sync::atomic::{AtomicU32, Ordering};

// Test type definitions
#[derive(Clone, Debug, PartialEq)]
struct Config {
	value: String,
}

struct ConfigKey;

impl InjectableKey for ConfigKey {}

#[async_trait]
impl Injectable for Config {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Config {
			value: "config_value".to_string(),
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
struct UncachedConfig {
	id: u32,
}

struct UncachedConfigKey;

impl InjectableKey for UncachedConfigKey {}

static UNCACHED_COUNTER: AtomicU32 = AtomicU32::new(0);
static REQUEST_COUNTER: AtomicU32 = AtomicU32::new(0);

#[async_trait]
impl Injectable for UncachedConfig {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		let id = UNCACHED_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(UncachedConfig { id })
	}

	async fn inject_uncached(ctx: &InjectionContext) -> DiResult<Self> {
		Self::inject(ctx).await
	}
}

#[derive(Clone, Debug, PartialEq)]
struct NestedService {
	config: Config,
}

#[derive(Clone, Debug, PartialEq)]
struct RequestCountedConfig {
	id: u32,
}

struct RequestCountedConfigKey;

impl InjectableKey for RequestCountedConfigKey {}

#[async_trait]
impl Injectable for NestedService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let config_depends = Depends::<ConfigKey, Config>::builder().resolve(ctx).await?;
		Ok(NestedService {
			config: config_depends.into_inner(),
		})
	}
}

/// Register test provider outputs in the global registry for keyed resolution.
fn register_test_types() {
	static REGISTER: Once = Once::new();
	REGISTER.call_once(|| {
		let registry = reinhardt_di::global_registry();
		registry.register_async::<FactoryOutput<ConfigKey, Config>, _, _>(
			reinhardt_di::DependencyScope::Request,
			|_ctx| async {
				Ok(FactoryOutput::new(Config {
					value: "config_value".to_string(),
				}))
			},
		);
		registry.register_async::<FactoryOutput<UncachedConfigKey, UncachedConfig>, _, _>(
			reinhardt_di::DependencyScope::Transient,
			|_ctx| async {
				let id = UNCACHED_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
				Ok(FactoryOutput::new(UncachedConfig { id }))
			},
		);
	});
}

#[rstest]
#[tokio::test]
async fn depends_builder_creates_instance(injection_context: InjectionContext) {
	register_test_types();

	// Act
	let depends = Depends::<ConfigKey, Config>::builder()
		.resolve(&injection_context)
		.await;

	// Assert
	assert!(depends.is_ok());
	assert_eq!(depends.unwrap().value, "config_value");
}

#[rstest]
#[tokio::test]
async fn depends_resolve_calls_injectable(injection_context: InjectionContext) {
	register_test_types();

	// Act
	let depends =
		Depends::<ConfigKey, Config>::resolve_from_registry(&injection_context, true).await;

	// Assert
	assert!(depends.is_ok());
	let config = depends.unwrap();
	assert_eq!(config.value, "config_value");
}

#[serial_test::serial(uncached_counter)]
#[tokio::test]
async fn depends_with_use_cache_true() {
	// Arrange
	register_test_types();
	let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
	let injection_context = InjectionContext::builder(singleton_scope).build();
	UNCACHED_COUNTER.store(0, Ordering::SeqCst);

	// Act — UncachedConfig is registered with Transient scope, so each
	// resolve creates a new instance regardless of use_cache flag.
	// The builder() (use_cache=true) no longer affects caching; scope does.
	let depends1 = Depends::<UncachedConfigKey, UncachedConfig>::builder()
		.resolve(&injection_context)
		.await
		.unwrap();

	let depends2 = Depends::<UncachedConfigKey, UncachedConfig>::builder()
		.resolve(&injection_context)
		.await
		.unwrap();

	// Assert — Transient scope: factory is called each time
	assert_ne!(depends1.id, depends2.id);
	assert_eq!(UNCACHED_COUNTER.load(Ordering::SeqCst), 2);
}

#[serial_test::serial(uncached_counter)]
#[tokio::test]
async fn depends_with_use_cache_false() {
	// Arrange
	register_test_types();
	let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
	let injection_context = InjectionContext::builder(singleton_scope).build();
	UNCACHED_COUNTER.store(0, Ordering::SeqCst);

	// Act — UncachedConfig is registered with Transient scope, so each
	// resolve creates a new instance. builder_no_cache() behaves the same
	// as builder() since caching is now scope-driven, not per-call.
	let depends1 = Depends::<UncachedConfigKey, UncachedConfig>::builder_no_cache()
		.resolve(&injection_context)
		.await
		.unwrap();

	let depends2 = Depends::<UncachedConfigKey, UncachedConfig>::builder_no_cache()
		.resolve(&injection_context)
		.await
		.unwrap();

	// Assert — Transient scope: factory is called each time
	assert_ne!(depends1.id, depends2.id);
	assert_eq!(UNCACHED_COUNTER.load(Ordering::SeqCst), 2);
}

#[serial_test::serial(request_counter)]
#[tokio::test]
async fn depends_with_use_cache_false_bypasses_request_scope_cache() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());
	registry.register_async::<FactoryOutput<RequestCountedConfigKey, RequestCountedConfig>, _, _>(
		DependencyScope::Request,
		|_ctx| async {
			let id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
			Ok(FactoryOutput::new(RequestCountedConfig { id }))
		},
	);
	let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
	let injection_context = InjectionContext::builder(singleton_scope)
		.with_registry(registry)
		.build();
	REQUEST_COUNTER.store(0, Ordering::SeqCst);

	// Act
	let cached = Depends::<RequestCountedConfigKey, RequestCountedConfig>::builder()
		.resolve(&injection_context)
		.await
		.unwrap();
	let fresh1 = Depends::<RequestCountedConfigKey, RequestCountedConfig>::builder_no_cache()
		.resolve(&injection_context)
		.await
		.unwrap();
	let fresh2 = Depends::<RequestCountedConfigKey, RequestCountedConfig>::builder_no_cache()
		.resolve(&injection_context)
		.await
		.unwrap();
	let cached_again = Depends::<RequestCountedConfigKey, RequestCountedConfig>::builder()
		.resolve(&injection_context)
		.await
		.unwrap();

	// Assert - uncached resolutions create fresh values and leave the cache intact.
	assert_eq!(cached.id, 1);
	assert_eq!(fresh1.id, 2);
	assert_eq!(fresh2.id, 3);
	assert_eq!(cached_again.id, 1);
	assert!(Arc::ptr_eq(cached.as_arc(), cached_again.as_arc()));
	assert_eq!(REQUEST_COUNTER.load(Ordering::SeqCst), 3);
}

#[rstest]
#[tokio::test]
async fn depends_nested_dependencies(injection_context: InjectionContext) {
	register_test_types();

	// Act
	let service = NestedService::inject(&injection_context).await;

	// Assert
	assert!(service.is_ok());
	let service = service.unwrap();
	assert_eq!(service.config.value, "config_value");
}
