//! Regression tests for kent8192/reinhardt-web#4685.
//!
//! `#[injectable_factory]` previously resolved non-`Depends` `#[inject]`
//! parameters with `ctx.resolve::<T>()` only — no fallback to
//! `Injectable::inject(ctx)` — so types whose only DI surface is a
//! manual `impl Injectable` (and that are not pre-registered) failed at
//! runtime with `DependencyNotRegistered`, surfacing as a generic 500 to
//! HTTP callers. The fix routes the non-`Depends` branch through
//! `Depends::<T>::resolve`, which performs the registry-first +
//! `T::inject` fallback used by `#[server_fn]` / `#[routes]`.
//!
//! The tests cover both `#[inject] x: T` and `#[inject] x: Depends<T>`:
//!
//! - **Variant 1** uses a manually-Injectable type that is *not* registered,
//!   so the only way it can resolve is through the new `Injectable::inject`
//!   fallback baked into the macro's non-`Depends` branch.
//! - **Variant 2** uses a *factory-only* type with **no** `impl Injectable`,
//!   registered via the global registry. This is both a runtime guard
//!   (only `resolve_from_registry` can satisfy it) and a compile-time guard:
//!   if the macro accidentally switches the `Depends<T>` branch to
//!   `Depends::resolve` (which would add a `T: Injectable` bound), the
//!   Variant 2 fixture stops compiling.

#![cfg(all(feature = "macros", feature = "testing"))]

use async_trait::async_trait;
use rstest::*;
use serial_test::serial;
use std::sync::Arc;

use reinhardt_di::{
	Depends, DiResult, Injectable, InjectionContext, SingletonScope, global_registry,
	injectable_factory,
};

/// Manually-Injectable type that is *not* registered in the global
/// registry — mirrors the framework shape used by `SessionData`,
/// `ServerFnRequest`, `AuthInfo`, etc.
#[derive(Clone, Debug, PartialEq)]
struct ManualSession {
	user_id: i64,
}

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

#[injectable_factory(scope = "transient")]
async fn derived_user_factory(#[inject] session: ManualSession) -> DerivedUser {
	DerivedUser {
		user_id: session.user_id,
	}
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
		.resolve::<DerivedUser>()
		.await
		.expect("factory must resolve via Injectable::inject fallback");

	// Assert
	assert_eq!(*derived, DerivedUser { user_id: 7 });
}

/// Variant 2: `Depends<T>` form against a *factory-only* type — i.e. one
/// that deliberately has **no** `impl Injectable`. This is the regression
/// guard the previous revision lacked: if the macro ever switches the
/// `Depends<T>` branch back to `Depends::resolve` (which requires
/// `T: Injectable`), `paged_request_factory` below stops compiling.
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

// NOTE: no `impl Injectable for FactoryOnlyConfig` on purpose. See doc above.

#[derive(Clone, Debug, PartialEq)]
struct PagedRequest {
	page_size: u32,
}

#[injectable_factory(scope = "transient")]
async fn paged_request_factory(
	#[inject] config: Depends<FactoryOnlyConfig>,
) -> PagedRequest {
	PagedRequest {
		page_size: config.page_size,
	}
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_with_depends_factory_only_type_still_uses_registry_only_path() {
	// Arrange — `FactoryOnlyConfig` has no `Injectable` impl, so the only
	// way `Depends<FactoryOnlyConfig>` can resolve is via the registry.
	// Register a factory for it so `resolve_from_registry` succeeds.
	let registry = global_registry();
	let _guard = registry.register_override::<FactoryOnlyConfig, _, _>(
		reinhardt_di::DependencyScope::Transient,
		|_ctx| async { Ok(FactoryOnlyConfig { page_size: 25 }) },
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let paged = ctx
		.resolve::<PagedRequest>()
		.await
		.expect("Depends<T> factory parameter must resolve via the registry");

	// Assert — value comes from the registry override (the only DI path
	// available for `FactoryOnlyConfig`).
	assert_eq!(*paged, PagedRequest { page_size: 25 });
}
