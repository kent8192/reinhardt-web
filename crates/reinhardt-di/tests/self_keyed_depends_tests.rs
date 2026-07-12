#![cfg(feature = "testing")]

use reinhardt_di::{
	DependencyScope, Depends, InjectableKey, InjectionContext, KeyedDepends, KeyedFactoryOutput,
	SelfKey, SingletonScope, global_registry,
};
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppConfig {
	host: &'static str,
}

struct PrimaryConfig;

impl InjectableKey for PrimaryConfig {}

struct ReplicaConfig;

impl InjectableKey for ReplicaConfig {}

#[serial(di_registry)]
#[tokio::test]
async fn depends_resolves_self_keyed_output() {
	let registry = global_registry();
	let _guard = registry
		.register_override::<KeyedFactoryOutput<SelfKey<AppConfig>, AppConfig>, _, _>(
			DependencyScope::Transient,
			|_ctx| async { Ok(KeyedFactoryOutput::new(AppConfig { host: "self" })) },
		);
	let ctx = InjectionContext::builder(Arc::new(SingletonScope::new())).build();

	let config = Depends::<AppConfig>::resolve_from_registry(&ctx, true)
		.await
		.expect("self-keyed config must resolve");

	assert_eq!(config.host, "self");
	assert_eq!(config.as_output().as_ref(), &AppConfig { host: "self" });
}

#[serial(di_registry)]
#[tokio::test]
async fn keyed_depends_resolves_multiple_bindings_for_same_value_type() {
	let registry = global_registry();
	let _primary = registry
		.register_override::<KeyedFactoryOutput<PrimaryConfig, AppConfig>, _, _>(
			DependencyScope::Transient,
			|_ctx| async { Ok(KeyedFactoryOutput::new(AppConfig { host: "primary" })) },
		);
	let _replica = registry
		.register_override::<KeyedFactoryOutput<ReplicaConfig, AppConfig>, _, _>(
			DependencyScope::Transient,
			|_ctx| async { Ok(KeyedFactoryOutput::new(AppConfig { host: "replica" })) },
		);
	let ctx = InjectionContext::builder(Arc::new(SingletonScope::new())).build();

	let primary = KeyedDepends::<PrimaryConfig, AppConfig>::resolve_from_registry(&ctx, true)
		.await
		.expect("primary config must resolve");
	let replica = KeyedDepends::<ReplicaConfig, AppConfig>::resolve_from_registry(&ctx, true)
		.await
		.expect("replica config must resolve");

	assert_eq!(primary.host, "primary");
	assert_eq!(replica.host, "replica");
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigError(&'static str);

type CheckedAppConfig = Result<AppConfig, ConfigError>;

#[serial(di_registry)]
#[tokio::test]
async fn depends_resolves_self_keyed_result_values_literally() {
	let registry = global_registry();
	let _guard = registry
		.register_override::<KeyedFactoryOutput<SelfKey<CheckedAppConfig>, CheckedAppConfig>, _, _>(
			DependencyScope::Transient,
			|_ctx| async { Ok(KeyedFactoryOutput::new(Ok(AppConfig { host: "checked" }))) },
		);
	let ctx = InjectionContext::builder(Arc::new(SingletonScope::new())).build();

	let checked = Depends::<CheckedAppConfig>::resolve_from_registry(&ctx, true)
		.await
		.expect("result dependency must resolve");

	assert_eq!(checked.as_ref(), &Ok(AppConfig { host: "checked" }));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RequestValue {
	id: usize,
}

static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[serial(di_registry)]
#[tokio::test]
async fn depends_builder_no_cache_bypasses_request_cache() {
	let registry = global_registry();
	let _guard = registry
		.register_override::<KeyedFactoryOutput<SelfKey<RequestValue>, RequestValue>, _, _>(
			DependencyScope::Request,
			|_ctx| async {
				let id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
				Ok(KeyedFactoryOutput::new(RequestValue { id }))
			},
		);
	let ctx = InjectionContext::builder(Arc::new(SingletonScope::new())).build();
	REQUEST_COUNTER.store(0, Ordering::SeqCst);

	let cached = Depends::<RequestValue>::builder()
		.resolve(&ctx)
		.await
		.unwrap();
	let fresh = Depends::<RequestValue>::builder_no_cache()
		.resolve(&ctx)
		.await
		.unwrap();
	let cached_again = Depends::<RequestValue>::builder()
		.resolve(&ctx)
		.await
		.unwrap();

	assert_eq!(cached.id, 1);
	assert_eq!(fresh.id, 2);
	assert_eq!(cached_again.id, 1);
	assert!(Arc::ptr_eq(cached.as_arc(), cached_again.as_arc()));
	assert_eq!(REQUEST_COUNTER.load(Ordering::SeqCst), 2);
}
