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
//! Each test exercises both `#[inject] x: T` and `#[inject] x: Depends<T>`
//! against an unregistered manually-Injectable type so the regression
//! cannot reappear on either codegen path.

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

/// Variant 2: `Depends<T>` form. Keeps the `resolve_from_registry` path
/// intact for factory-only types (no `Injectable` impl) while still
/// covering the manual-Injectable case via the registry's cache hit on
/// the second resolve. We register the manual type into the local
/// registry view (still no `#[injectable]` macro) so the assertion is
/// honest about which path runs.
#[derive(Clone, Debug, PartialEq)]
struct ManualClock {
	now_ms: u64,
}

#[async_trait]
impl Injectable for ManualClock {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(ManualClock {
			now_ms: 1_700_000_000_000,
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
struct StampedRequest {
	at_ms: u64,
}

#[injectable_factory(scope = "transient")]
async fn stamped_request_factory(#[inject] clock: Depends<ManualClock>) -> StampedRequest {
	StampedRequest {
		at_ms: clock.now_ms,
	}
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn factory_with_depends_factory_only_type_still_uses_registry_only_path() {
	// Arrange — register a factory for ManualClock so resolve_from_registry hits
	let registry = global_registry();
	let _guard = registry.register_override::<ManualClock, _, _>(
		reinhardt_di::DependencyScope::Transient,
		|_ctx| async { Ok(ManualClock { now_ms: 42 }) },
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	// Act
	let stamped = ctx
		.resolve::<StampedRequest>()
		.await
		.expect("Depends<T> factory parameter must resolve via the registry");

	// Assert — value comes from the overridden factory, not from
	// `ManualClock::inject` (which would yield `1_700_000_000_000`).
	assert_eq!(*stamped, StampedRequest { at_ms: 42 });
}
