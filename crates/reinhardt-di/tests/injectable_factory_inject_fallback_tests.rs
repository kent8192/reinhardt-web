//! Regression tests for kent8192/reinhardt-web#4685.
//!
//! `#[injectable]` previously resolved non-`Depends` `#[inject]`
//! parameters with `ctx.resolve::<T>()` only — no fallback to
//! `Injectable::inject(ctx)` — so types whose only DI surface is a
//! manual `impl Injectable` (and that are not pre-registered) failed at
//! runtime with `DependencyNotRegistered`, surfacing as a generic 500 to
//! HTTP callers. The fix routes the non-`Depends` branch through
//! the runtime trait dispatcher, which performs registry-first +
//! `T::inject` fallback for non-wrapper parameters.
//!
//! The tests cover both `#[inject] x: T` and `#[inject] x: Depends<K, T>`:
//!
//! - **Variant 1** uses a manually-Injectable type that is *not* registered,
//!   so the only way it can resolve is through the new `Injectable::inject`
//!   fallback baked into the macro's non-`Depends` branch.
//! - **Variant 2** uses a keyed *factory-only* type with **no** `impl
//!   Injectable`, registered via the global registry. This is both a runtime
//!   guard (only `resolve_from_registry` can satisfy it) and a compile-time
//!   guard that direct `Depends<K, T>` parameters do not add a `T:
//!   Injectable` bound.

#![cfg(all(feature = "macros", feature = "testing"))]

use async_trait::async_trait;
use rstest::*;
use serial_test::serial;
use std::sync::Arc;

use reinhardt_di::{
	Depends, DiResult, FactoryOutput, Injectable, InjectableKey, InjectableType, InjectionContext,
	SingletonScope, global_registry, injectable,
};

/// Manually-Injectable type that is *not* registered in the global
/// registry — mirrors the framework shape used by `SessionData`,
/// `ServerFnRequest`, `AuthInfo`, etc.
#[derive(Clone, Debug, PartialEq)]
struct ManualSession {
	user_id: i64,
}

struct ManualSessionKey;

impl InjectableKey for ManualSessionKey {}

#[async_trait]
impl Injectable for ManualSession {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(ManualSession { user_id: 7 })
	}
}

/// Factory output that downstream code reads via `ctx.resolve` — proves
/// the whole factory body executed (the buggy path returned an error
/// before the body ran).
#[derive(Clone, Debug, PartialEq)]
struct DerivedUser {
	user_id: i64,
}

struct DerivedUserKey;

impl InjectableKey for DerivedUserKey {}

#[derive(Clone, Debug, PartialEq)]
struct DerivedDependsUser {
	user_id: i64,
}

struct DerivedDependsUserKey;

impl InjectableKey for DerivedDependsUserKey {}

#[injectable(scope = "transient")]
async fn derived_user_factory(
	#[inject] session: ManualSession,
) -> FactoryOutput<DerivedUserKey, DerivedUser> {
	FactoryOutput::new(DerivedUser {
		user_id: session.user_id,
	})
}

#[injectable(scope = "transient")]
async fn derived_depends_user_factory(
	#[inject] session: Depends<ManualSessionKey, ManualSession>,
) -> FactoryOutput<DerivedDependsUserKey, DerivedDependsUser> {
	FactoryOutput::new(DerivedDependsUser {
		user_id: session.user_id,
	})
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_resolves_non_depends_manual_injectable_via_inject_fallback() {
	// Arrange
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();
	assert!(
		!global_registry().is_registered::<ManualSession>(),
		"ManualSession must remain unregistered to exercise the inject fallback",
	);

	// Act
	let derived = ctx
		.resolve::<FactoryOutput<DerivedUserKey, DerivedUser>>()
		.await
		.expect("factory must resolve via Injectable::inject fallback");

	// Assert
	assert_eq!(derived.as_ref().as_ref(), &DerivedUser { user_id: 7 });
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_resolves_keyed_depends_manual_injectable_via_registry_output() {
	// Arrange
	let registry = global_registry();
	let _guard = registry
		.register_override::<FactoryOutput<ManualSessionKey, ManualSession>, _, _>(
			reinhardt_di::DependencyScope::Transient,
			|_ctx| async { Ok(FactoryOutput::new(ManualSession { user_id: 7 })) },
		);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();
	assert!(
		!global_registry().is_registered::<ManualSession>(),
		"ManualSession must remain unregistered to exercise keyed Depends registry resolution",
	);

	// Act
	let derived = ctx
		.resolve::<FactoryOutput<DerivedDependsUserKey, DerivedDependsUser>>()
		.await
		.expect("Depends<K, T> parameter must resolve keyed factory output");

	// Assert
	assert_eq!(
		derived.as_ref().as_ref(),
		&DerivedDependsUser { user_id: 7 }
	);
}

/// Variant 2: `Depends<K, T>` form against a *factory-only* type — i.e. one
/// that deliberately has **no** `impl Injectable`. This is the regression
/// guard the previous revision lacked: if the macro ever switches the
/// direct `Depends<K, T>` branch back to a fallback path that requires
/// `T: Injectable`, `paged_request_factory` below stops compiling.
///
/// At the same time, the only DI surface available for `FactoryOnlyConfig`
/// is the registry, so the runtime path exercised here is unambiguously
/// `Depends::resolve_from_registry` — exactly the path used by
/// factory-produced types (see
/// `tests/integration/tests/di/ui/pass/factory_depends_in_server_fn.rs`).
#[derive(Clone, Debug, PartialEq)]
struct FactoryOnlyConfig {
	page_size: u32,
}

struct FactoryOnlyConfigKey;

impl InjectableKey for FactoryOnlyConfigKey {}

// NOTE: no `impl Injectable for FactoryOnlyConfig` on purpose. See doc above.

#[derive(Clone, Debug, PartialEq)]
struct PagedRequest {
	page_size: u32,
}

struct PagedRequestKey;

impl InjectableKey for PagedRequestKey {}

#[injectable(scope = "transient")]
async fn paged_request_factory(
	#[inject] config: Depends<FactoryOnlyConfigKey, FactoryOnlyConfig>,
) -> FactoryOutput<PagedRequestKey, PagedRequest> {
	FactoryOutput::new(PagedRequest {
		page_size: config.page_size,
	})
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_with_depends_factory_only_type_still_uses_registry_only_path() {
	// Arrange — `FactoryOnlyConfig` has no `Injectable` impl, so the only
	// way `Depends<FactoryOnlyConfigKey, FactoryOnlyConfig>` can resolve is
	// via the registry.
	// Register a factory for it so `resolve_from_registry` succeeds.
	let registry = global_registry();
	let _guard = registry
		.register_override::<FactoryOutput<FactoryOnlyConfigKey, FactoryOnlyConfig>, _, _>(
			reinhardt_di::DependencyScope::Transient,
			|_ctx| async { Ok(FactoryOutput::new(FactoryOnlyConfig { page_size: 25 })) },
		);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let paged = ctx
		.resolve::<FactoryOutput<PagedRequestKey, PagedRequest>>()
		.await
		.expect("Depends<K, T> factory parameter must resolve via the registry");

	// Assert — value comes from the registry override (the only DI path
	// available for `FactoryOnlyConfig`).
	assert_eq!(paged.as_ref().as_ref(), &PagedRequest { page_size: 25 });
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
struct LazyRequest {
	page_size: u32,
}

struct LazyRequestKey;

impl InjectableKey for LazyRequestKey {}

#[injectable(scope = "transient")]
async fn lazy_request_factory(
	#[inject] config: Lazy<FactoryOnlyConfig>,
) -> FactoryOutput<LazyRequestKey, LazyRequest> {
	FactoryOutput::new(LazyRequest {
		page_size: config.page_size,
	})
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_accepts_custom_injectable_type_wrapper_without_name_matching() {
	// Arrange — `Lazy<T>` is not named `Depends`, so this only works when
	// `#[injectable]` delegates wrapper detection to `InjectableType`.
	let registry = global_registry();
	let _guard = registry.register_override::<FactoryOnlyConfig, _, _>(
		reinhardt_di::DependencyScope::Transient,
		|_ctx| async { Ok(FactoryOnlyConfig { page_size: 50 }) },
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let lazy = ctx
		.resolve::<FactoryOutput<LazyRequestKey, LazyRequest>>()
		.await
		.expect("custom InjectableType wrapper must resolve via the registry");

	// Assert
	assert_eq!(lazy.as_ref().as_ref(), &LazyRequest { page_size: 50 });
}

#[derive(Clone, Debug, PartialEq)]
struct DualConfig {
	source: &'static str,
}

#[derive(Clone, Debug)]
struct DualMode {
	inner: Arc<DualConfig>,
}

impl std::ops::Deref for DualMode {
	type Target = DualConfig;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl InjectableType for DualMode {
	type Inner = DualConfig;

	fn from_resolved(inner: Arc<Self::Inner>, _use_cache: bool) -> Self {
		Self { inner }
	}
}

#[async_trait]
impl Injectable for DualMode {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			inner: Arc::new(DualConfig {
				source: "injectable",
			}),
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
struct DualModeReport {
	source: &'static str,
}

struct DualModeReportKey;

impl InjectableKey for DualModeReportKey {}

#[injectable(scope = "transient")]
async fn dual_mode_report_factory(
	#[inject] mode: DualMode,
) -> FactoryOutput<DualModeReportKey, DualModeReport> {
	FactoryOutput::new(DualModeReport {
		source: mode.source,
	})
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_prefers_injectable_type_wrapper_over_injectable_fallback() {
	// Arrange — `DualMode` implements both traits. The resolved value must come
	// from the registry-backed `InjectableType` path, not `Injectable::inject`.
	let registry = global_registry();
	let _guard = registry.register_override::<DualConfig, _, _>(
		reinhardt_di::DependencyScope::Transient,
		|_ctx| async { Ok(DualConfig { source: "registry" }) },
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let report = ctx
		.resolve::<FactoryOutput<DualModeReportKey, DualModeReport>>()
		.await
		.expect("InjectableType must take precedence over Injectable fallback");

	// Assert
	assert_eq!(
		report.as_ref().as_ref(),
		&DualModeReport { source: "registry" }
	);
}
