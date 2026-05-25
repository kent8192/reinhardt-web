//! Tests for DI scope hierarchy enforcement (Issue #4651)

use reinhardt_di::{
	DependencyScope, DiError, InjectionContext, SingletonScope, global_registry,
};
use rstest::rstest;
use serial_test::serial;
use std::sync::Arc;

// --- outlives() unit tests ---

#[rstest]
#[case::singleton_resolves_singleton(DependencyScope::Singleton, DependencyScope::Singleton, true)]
#[case::singleton_resolves_request(DependencyScope::Request, DependencyScope::Singleton, false)]
#[case::singleton_resolves_transient(DependencyScope::Transient, DependencyScope::Singleton, true)]
#[case::request_resolves_singleton(DependencyScope::Singleton, DependencyScope::Request, true)]
#[case::request_resolves_request(DependencyScope::Request, DependencyScope::Request, true)]
#[case::request_resolves_transient(DependencyScope::Transient, DependencyScope::Request, true)]
#[case::transient_resolves_singleton(DependencyScope::Singleton, DependencyScope::Transient, true)]
#[case::transient_resolves_request(DependencyScope::Request, DependencyScope::Transient, true)]
#[case::transient_resolves_transient(DependencyScope::Transient, DependencyScope::Transient, true)]
fn outlives_returns_expected(
	#[case] dependency_scope: DependencyScope,
	#[case] dependent_scope: DependencyScope,
	#[case] expected: bool,
) {
	// Act
	let result = dependency_scope.outlives(dependent_scope);

	// Assert
	assert_eq!(
		result, expected,
		"{dependency_scope:?}.outlives({dependent_scope:?}) should be {expected}"
	);
}

// --- Runtime scope enforcement tests ---

#[derive(Clone, Debug, Default)]
struct RequestConfig;

#[derive(Clone, Debug, Default)]
struct SingletonService;

#[derive(Clone, Debug, Default)]
struct TransientWidget;

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn singleton_resolving_singleton_succeeds() {
	// Arrange
	let registry = global_registry();
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |_ctx| async {
		Ok(SingletonService)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<SingletonService>().await;

	// Assert
	assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn singleton_resolving_request_returns_scope_error() {
	// Arrange
	let registry = global_registry();
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig::default())
	});
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |ctx| async move {
		// This singleton factory tries to resolve a request-scoped dependency
		let _config = ctx.resolve::<RequestConfig>().await?;
		Ok(SingletonService)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<SingletonService>().await;

	// Assert
	assert!(
		matches!(result, Err(DiError::ScopeError(_))),
		"Expected ScopeError, got: {result:?}"
	);
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn request_resolving_singleton_succeeds() {
	// Arrange
	let registry = global_registry();
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |_ctx| async {
		Ok(SingletonService)
	});
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |ctx| async move {
		let _service = ctx.resolve::<SingletonService>().await?;
		Ok(RequestConfig::default())
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<RequestConfig>().await;

	// Assert
	assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn request_resolving_request_succeeds() {
	// Arrange
	#[derive(Clone, Default)]
	struct RequestConfigB;

	let registry = global_registry();
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig::default())
	});
	registry.register_async::<RequestConfigB, _, _>(DependencyScope::Request, |ctx| async move {
		let _config = ctx.resolve::<RequestConfig>().await?;
		Ok(RequestConfigB)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<RequestConfigB>().await;

	// Assert
	assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn root_resolving_any_scope_succeeds() {
	// Arrange
	let registry = global_registry();
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |_ctx| async {
		Ok(SingletonService)
	});
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig::default())
	});
	registry.register_async::<TransientWidget, _, _>(DependencyScope::Transient, |_ctx| async {
		Ok(TransientWidget)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act & Assert
	assert!(ctx.resolve::<SingletonService>().await.is_ok());
	assert!(ctx.resolve::<RequestConfig>().await.is_ok());
	assert!(ctx.resolve::<TransientWidget>().await.is_ok());
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn transient_resolving_any_scope_succeeds() {
	// Arrange
	#[derive(Clone, Default)]
	struct TransientConsumer;

	let registry = global_registry();
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |_ctx| async {
		Ok(SingletonService)
	});
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig::default())
	});
	registry.register_async::<TransientConsumer, _, _>(
		DependencyScope::Transient,
		|ctx| async move {
			let _singleton = ctx.resolve::<SingletonService>().await?;
			let _request = ctx.resolve::<RequestConfig>().await?;
			Ok(TransientConsumer)
		},
	);

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<TransientConsumer>().await;

	// Assert
	assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn scope_error_message_contains_type_names() {
	// Arrange
	let registry = global_registry();
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig::default())
	});
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |ctx| async move {
		let _config = ctx.resolve::<RequestConfig>().await?;
		Ok(SingletonService)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Act
	let result = ctx.resolve::<SingletonService>().await;

	// Assert
	let err = result.unwrap_err();
	let msg = err.to_string();
	assert!(
		msg.contains("Singleton"),
		"Error should mention Singleton scope: {msg}"
	);
	assert!(
		msg.contains("Request"),
		"Error should mention Request scope: {msg}"
	);
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn singleton_resolving_cached_request_still_returns_scope_error() {
	// Arrange: pre-populate the request cache so the singleton hits the fast path
	let registry = global_registry();
	registry.register_async::<RequestConfig, _, _>(DependencyScope::Request, |_ctx| async {
		Ok(RequestConfig)
	});
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |ctx| async move {
		let _config = ctx.resolve::<RequestConfig>().await?;
		Ok(SingletonService)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Warm the request cache from a root (Transient) context — this succeeds
	let _ = ctx.resolve::<RequestConfig>().await.expect("root can resolve request");

	// Act: singleton factory tries to resolve the now-cached RequestConfig
	let result = ctx.resolve::<SingletonService>().await;

	// Assert: scope violation must still be detected even on cache hit
	assert!(
		matches!(result, Err(DiError::ScopeError(_))),
		"Cached request-scoped dep must still fail for singleton: {result:?}"
	);
}

#[rstest]
#[tokio::test]
#[serial(di_registry)]
async fn singleton_resolving_preseeded_request_returns_scope_error() {
	// Arrange: pre-seed request cache directly (no registry entry)
	let registry = global_registry();
	registry.register_async::<SingletonService, _, _>(DependencyScope::Singleton, |ctx| async move {
		let _config = ctx.resolve::<RequestConfig>().await?;
		Ok(SingletonService)
	});

	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();
	ctx.set_request(RequestConfig);

	// Act: singleton factory resolves pre-seeded request-scoped type
	let result = ctx.resolve::<SingletonService>().await;

	// Assert
	assert!(
		matches!(result, Err(DiError::ScopeError(_))),
		"Pre-seeded request dep must still fail for singleton: {result:?}"
	);
}
