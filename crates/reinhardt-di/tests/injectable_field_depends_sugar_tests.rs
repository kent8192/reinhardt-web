//! Regression tests for kent8192/reinhardt-web#4937.
//!
//! `DependsResult<T, E>` / `DependsOption<T>` are sugar aliases for
//! `Depends<Result<T, E>>` / `Depends<Option<T>>`. Their primary use case is a
//! factory that returns `Result<T, E>` / `Option<T>` to obtain a distinct DI
//! registry key (`TypeId`) from `T` while preserving `T`'s trait impls.
//!
//! Those inner types (`Result<T, E>` / `Option<T>`) are produced by
//! `#[injectable_factory]` and never implement `Injectable`. Therefore field
//! injection through `#[injectable]` MUST resolve them via
//! `Depends::resolve_from_registry` (no `T: Injectable` bound), not
//! `Depends::resolve` (which requires `T: Injectable`).
//!
//! These tests are both a compile-time guard (if the macro routed the sugar
//! aliases through `Depends::resolve`, the `#[injectable]` structs below would
//! fail to compile because `Result<..>` / `Option<..>` are not `Injectable`)
//! and a runtime guard (the only DI surface for the inner type is the
//! registry).

#![cfg(all(feature = "macros", feature = "testing"))]

use rstest::*;
use serial_test::serial;
use std::sync::Arc;

use reinhardt_di::{
	DependencyScope, DependsOption, DependsResult, Injectable, InjectionContext, SingletonScope,
	global_registry, injectable,
};

/// Factory-produced user type. No `impl Injectable` on purpose — it is only
/// ever produced wrapped in a `Result` by a factory and read back through a
/// `DependsResult` field.
#[derive(Clone, Debug, PartialEq)]
struct SessionUser {
	id: i64,
}

#[derive(Clone, Debug, PartialEq)]
struct SessionError {
	reason: String,
}

/// `#[injectable]` service whose field is the sugar alias over a
/// factory-produced `Result`. If `DependsResult` field injection routed
/// through `Depends::resolve` (which requires `Result<..>: Injectable`), this
/// struct would not compile.
#[injectable]
struct AuthService {
	#[inject]
	user: DependsResult<SessionUser, SessionError>,
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_result_resolves_from_registry() {
	// Arrange — register the factory-produced `Result<SessionUser, SessionError>`.
	// This is the only DI surface available for the inner type.
	let registry = global_registry();
	let _guard = registry.register_override::<Result<SessionUser, SessionError>, _, _>(
		DependencyScope::Transient,
		|_ctx| async { Ok(Ok(SessionUser { id: 42 })) },
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act — invoke the generated `inject` directly to exercise the field codegen.
	let service = <AuthService as Injectable>::inject(&ctx)
		.await
		.expect("AuthService `DependsResult` field must resolve from the registry");

	// Assert — the `DependsResult` field derefs to `Result<SessionUser, SessionError>`.
	assert_eq!(*service.user, Ok(SessionUser { id: 42 }));
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_result_preserves_err_variant() {
	// Arrange — the registered factory yields the Err variant.
	let registry = global_registry();
	let _guard = registry.register_override::<Result<SessionUser, SessionError>, _, _>(
		DependencyScope::Transient,
		|_ctx| async {
			Ok(Err(SessionError {
				reason: "anonymous".to_string(),
			}))
		},
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let service = <AuthService as Injectable>::inject(&ctx)
		.await
		.expect("AuthService `DependsResult` field must resolve from the registry");

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

/// `#[injectable]` service with a `DependsOption` field over a factory-produced
/// `Option`. Same compile-time + runtime guard as `AuthService`.
#[injectable]
struct CacheConsumer {
	#[inject]
	cache: DependsOption<CacheBackend>,
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn injectable_field_depends_option_resolves_from_registry() {
	// Arrange — register the factory-produced `Option<CacheBackend>`.
	let registry = global_registry();
	let _guard = registry.register_override::<Option<CacheBackend>, _, _>(
		DependencyScope::Transient,
		|_ctx| async {
			Ok(Some(CacheBackend {
				url: "redis://localhost".to_string(),
			}))
		},
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let consumer = <CacheConsumer as Injectable>::inject(&ctx)
		.await
		.expect("CacheConsumer `DependsOption` field must resolve from the registry");

	// Assert — the `DependsOption` field derefs to `Option<CacheBackend>`.
	assert_eq!(
		*consumer.cache,
		Some(CacheBackend {
			url: "redis://localhost".to_string(),
		})
	);
}
