//! Tests for the testing-only `register_override` API.
//!
//! These tests mutate the global registry and therefore must run serially
//! within the `di_registry` group.

#![cfg(feature = "testing")]

use std::sync::Arc;

use rstest::*;
use serial_test::serial;

use reinhardt_di::{
	DependencyRegistry, DependencyScope, DiResult, InjectionContext, SingletonScope,
};

#[derive(Clone, Debug, PartialEq)]
struct Greeting(&'static str);

async fn prod_greeting(_ctx: Arc<InjectionContext>) -> DiResult<Greeting> {
	Ok(Greeting("production"))
}

async fn mock_greeting(_ctx: Arc<InjectionContext>) -> DiResult<Greeting> {
	Ok(Greeting("mock"))
}

fn fresh_registry() -> Arc<DependencyRegistry> {
	Arc::new(DependencyRegistry::new())
}

fn fresh_ctx() -> InjectionContext {
	let scope = Arc::new(SingletonScope::new());
	InjectionContext::builder(scope).build()
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn register_override_does_not_panic_when_type_already_registered() {
	// Arrange
	let registry = fresh_registry();
	registry.register_async::<Greeting, _, _>(DependencyScope::Singleton, prod_greeting);

	// Act
	let _guard = registry.register_override::<Greeting, _, _>(
		DependencyScope::Singleton,
		mock_greeting,
	);

	// Assert
	assert!(registry.is_registered::<Greeting>());
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn override_guard_restores_previous_factory_on_drop() {
	// Arrange
	let registry = fresh_registry();
	registry.register_async::<Greeting, _, _>(DependencyScope::Singleton, prod_greeting);
	let ctx = fresh_ctx();

	// Act -- install override, then drop it
	{
		let _guard = registry.register_override::<Greeting, _, _>(
			DependencyScope::Singleton,
			mock_greeting,
		);
		let arc = registry.create::<Greeting>(&ctx).await.unwrap();
		assert_eq!(*arc, Greeting("mock"));
	}

	// Assert -- after guard drop, the production factory is back
	let arc = registry.create::<Greeting>(&ctx).await.unwrap();
	assert_eq!(*arc, Greeting("production"));
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn override_guard_removes_entry_when_no_previous_registration() {
	// Arrange
	let registry = fresh_registry();
	assert!(!registry.is_registered::<Greeting>());

	// Act
	{
		let _guard = registry.register_override::<Greeting, _, _>(
			DependencyScope::Singleton,
			mock_greeting,
		);
		assert!(registry.is_registered::<Greeting>());
	}

	// Assert -- entry removed after drop
	assert!(!registry.is_registered::<Greeting>());
}

use reinhardt_di::global_registry;

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn ctx_resolve_returns_override_value_for_transient_scope() {
	// Arrange
	let registry = global_registry().clone();
	let _guard = registry.register_override::<Greeting, _, _>(
		DependencyScope::Transient,
		mock_greeting,
	);
	let ctx = fresh_ctx();

	// Act
	let arc: std::sync::Arc<Greeting> = ctx.resolve::<Greeting>().await.unwrap();

	// Assert
	assert_eq!(*arc, Greeting("mock"));
}

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn override_guard_drop_after_registry_drop_is_a_noop() {
	// Arrange
	let registry = fresh_registry();
	let guard = registry.register_override::<Greeting, _, _>(
		DependencyScope::Singleton,
		mock_greeting,
	);

	// Act -- drop the registry, then the guard
	drop(registry);
	drop(guard);

	// Assert -- if we reach here, no panic occurred (Weak upgrade returned None)
}
