//! Regression tests for kent8192/reinhardt-web#4937.
//!
//! `Depends<K, Result<T, E>>` / `Depends<K, Option<T>>` fields keep result and
//! optional provider outputs distinct from `T` while preserving `T`'s trait
//! impls.
//!
//! Those inner types (`Result<T, E>` / `Option<T>`) are produced by
//! `#[injectable]` providers and never implement `Injectable`. Therefore wrapper
//! injection MUST resolve them via `Depends::resolve_from_registry` (no
//! `T: Injectable` bound).
//!
//! These tests are both a compile-time guard (the wrapper fields below must not
//! require `Result<..>` / `Option<..>` to implement `Injectable`) and a runtime
//! guard (the only DI surface for the inner type is the registry).

#![cfg(all(feature = "macros", feature = "testing"))]

use rstest::*;
use serial_test::serial;
use std::sync::Arc;

use reinhardt_di::{
	DependencyScope, Depends, FactoryOutput, Injectable, InjectableKey, InjectableType,
	InjectionContext, SingletonScope, global_registry,
};

/// Factory-produced user type. No `impl Injectable` on purpose — it is only
/// ever produced wrapped in a `Result` by a factory and read back through a
/// keyed `Depends` field.
#[derive(Clone, Debug, PartialEq)]
struct SessionUser {
	id: i64,
}

#[derive(Clone, Debug, PartialEq)]
struct SessionError {
	reason: String,
}

struct SessionUserResultKey;

impl InjectableKey for SessionUserResultKey {}

/// Service whose field is the sugar alias over a factory-produced `Result`.
/// If wrapper injection required `Result<..>: Injectable`, this struct would
/// not compile.
struct AuthService {
	user: Depends<SessionUserResultKey, Result<SessionUser, SessionError>>,
}

#[async_trait::async_trait]
impl Injectable for AuthService {
	async fn inject(ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(Self {
			user: Depends::<SessionUserResultKey, Result<SessionUser, SessionError>>::resolve_from_registry(
				ctx, true,
			)
			.await?,
		})
	}
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_result_resolves_from_registry() {
	// Arrange — register the keyed factory-produced
	// `Result<SessionUser, SessionError>`. This is the only DI surface
	// available for the inner type.
	let registry = global_registry();
	let _guard = registry
		.register_override::<
			FactoryOutput<SessionUserResultKey, Result<SessionUser, SessionError>>,
			_,
			_,
		>(DependencyScope::Transient, |_ctx| async {
			Ok(FactoryOutput::new(Ok(SessionUser { id: 42 })))
		});
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act — invoke the generated `inject` directly to exercise the field codegen.
	let service = <AuthService as Injectable>::inject(&ctx)
		.await
		.expect("AuthService keyed result field must resolve from the registry");

	// Assert — the keyed `Depends` field derefs to `Result<SessionUser, SessionError>`.
	assert_eq!(*service.user, Ok(SessionUser { id: 42 }));
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_result_preserves_err_variant() {
	// Arrange — the registered factory yields the Err variant.
	let registry = global_registry();
	let _guard = registry
		.register_override::<
			FactoryOutput<SessionUserResultKey, Result<SessionUser, SessionError>>,
			_,
			_,
		>(DependencyScope::Transient, |_ctx| async {
			Ok(FactoryOutput::new(Err(SessionError {
				reason: "anonymous".to_string(),
			})))
		});
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let service = <AuthService as Injectable>::inject(&ctx)
		.await
		.expect("AuthService keyed result field must resolve from the registry");

	// Assert — the error state is preserved inside the resolved `Result`.
	assert_eq!(
		*service.user,
		Err(SessionError {
			reason: "anonymous".to_string(),
		})
	);
}

/// Factory-produced optional dependency. No `impl Injectable`.
#[derive(Clone, Debug, PartialEq)]
struct CacheBackend {
	url: String,
}

struct CacheBackendOptionKey;

impl InjectableKey for CacheBackendOptionKey {}

/// Service with a keyed `Depends` field over a factory-produced `Option`.
/// Same compile-time + runtime guard as `AuthService`.
struct CacheConsumer {
	cache: Depends<CacheBackendOptionKey, Option<CacheBackend>>,
}

#[async_trait::async_trait]
impl Injectable for CacheConsumer {
	async fn inject(ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(Self {
			cache: Depends::<CacheBackendOptionKey, Option<CacheBackend>>::resolve_from_registry(
				ctx, true,
			)
			.await?,
		})
	}
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_option_resolves_from_registry() {
	// Arrange — register the factory-produced `Option<CacheBackend>`.
	let registry = global_registry();
	let _guard = registry
		.register_override::<FactoryOutput<CacheBackendOptionKey, Option<CacheBackend>>, _, _>(
			DependencyScope::Transient,
			|_ctx| async {
				Ok(FactoryOutput::new(Some(CacheBackend {
					url: "redis://localhost".to_string(),
				})))
			},
		);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let consumer = <CacheConsumer as Injectable>::inject(&ctx)
		.await
		.expect("CacheConsumer keyed option field must resolve from the registry");

	// Assert — the keyed `Depends` field derefs to `Option<CacheBackend>`.
	assert_eq!(
		*consumer.cache,
		Some(CacheBackend {
			url: "redis://localhost".to_string(),
		})
	);
}

#[derive(Clone, Debug)]
struct Lazy<T>
where
	T: Send + Sync + 'static,
{
	inner: Arc<T>,
}

impl<T> std::ops::Deref for Lazy<T>
where
	T: Send + Sync + 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<T> InjectableType for Lazy<T>
where
	T: Send + Sync + 'static,
{
	type Inner = T;

	fn from_resolved(inner: Arc<Self::Inner>, _use_cache: bool) -> Self {
		Self { inner }
	}
}

#[derive(Clone, Debug, PartialEq)]
struct FieldOnlyConfig {
	value: String,
}

struct CustomWrapperConsumer {
	config: Lazy<FieldOnlyConfig>,
}

#[async_trait::async_trait]
impl Injectable for CustomWrapperConsumer {
	async fn inject(ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		let inner = ctx.resolve::<FieldOnlyConfig>().await?;
		Ok(Self {
			config: <Lazy<FieldOnlyConfig> as InjectableType>::from_resolved(inner, true),
		})
	}
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_accepts_custom_injectable_type_wrapper() {
	// Arrange — `FieldOnlyConfig` has no `Injectable` impl. The custom wrapper
	// field can only resolve through `InjectableType::Inner` and the registry.
	let registry = global_registry();
	let _guard = registry.register_override::<FieldOnlyConfig, _, _>(
		DependencyScope::Transient,
		|_ctx| async {
			Ok(FieldOnlyConfig {
				value: "custom".to_string(),
			})
		},
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let consumer = <CustomWrapperConsumer as Injectable>::inject(&ctx)
		.await
		.expect("custom InjectableType field must resolve from the registry");

	// Assert
	assert_eq!(
		*consumer.config,
		FieldOnlyConfig {
			value: "custom".to_string(),
		}
	);
}
