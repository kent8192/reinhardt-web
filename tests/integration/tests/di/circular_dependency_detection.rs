//! Circular dependency detection integration tests.
//!
//! This test suite verifies runtime circular dependency detection for manual
//! `Injectable` implementations.

use super::test_helpers::resolve_injectable;
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

fn test_context() -> InjectionContext {
	let singleton_scope = Arc::new(SingletonScope::new());
	InjectionContext::builder(singleton_scope).build()
}

#[derive(Clone, Debug)]
struct ServiceA {
	_b: Arc<ServiceB>,
}

#[derive(Clone, Debug)]
struct ServiceB {
	_c: Arc<ServiceC>,
}

#[derive(Clone, Debug)]
struct ServiceC {
	_a: Arc<ServiceA>,
}

#[async_trait::async_trait]
impl Injectable for ServiceA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_b: resolve_injectable::<ServiceB>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_c: resolve_injectable::<ServiceC>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ServiceC {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_a: resolve_injectable::<ServiceA>(ctx).await?,
		})
	}
}

#[derive(Clone, Debug)]
struct DirectA {
	_b: Arc<DirectB>,
}

#[derive(Clone, Debug)]
struct DirectB {
	_a: Arc<DirectA>,
}

#[async_trait::async_trait]
impl Injectable for DirectA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_b: resolve_injectable::<DirectB>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for DirectB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_a: resolve_injectable::<DirectA>(ctx).await?,
		})
	}
}

#[derive(Clone, Debug)]
struct SelfDependent {
	_inner: Arc<SelfDependent>,
}

#[async_trait::async_trait]
impl Injectable for SelfDependent {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_inner: resolve_injectable::<SelfDependent>(ctx).await?,
		})
	}
}

#[derive(Clone, Debug)]
struct ComplexA {
	_b: Arc<ComplexB>,
}

#[derive(Clone, Debug)]
struct ComplexB {
	_c: Arc<ComplexC>,
}

#[derive(Clone, Debug)]
struct ComplexC {
	_d: Arc<ComplexD>,
}

#[derive(Clone, Debug)]
struct ComplexD {
	_b: Arc<ComplexB>,
}

#[async_trait::async_trait]
impl Injectable for ComplexA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_b: resolve_injectable::<ComplexB>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ComplexB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_c: resolve_injectable::<ComplexC>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ComplexC {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_d: resolve_injectable::<ComplexD>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ComplexD {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_b: resolve_injectable::<ComplexB>(ctx).await?,
		})
	}
}

#[derive(Clone, Debug)]
struct NoCycleA {
	_b: Arc<NoCycleB>,
}

#[derive(Clone, Debug, Default)]
struct NoCycleB {
	value: i32,
}

#[async_trait::async_trait]
impl Injectable for NoCycleA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_b: resolve_injectable::<NoCycleB>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for NoCycleB {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self { value: 0 })
	}
}

#[derive(Clone, Debug)]
struct Level1;

#[derive(Clone, Debug)]
struct Level2 {
	_dep: Arc<Level1>,
}

#[derive(Clone, Debug)]
struct Level3 {
	_dep: Arc<Level2>,
}

#[derive(Clone, Debug)]
struct Level4 {
	_dep: Arc<Level3>,
}

#[derive(Clone, Debug)]
struct Level5 {
	_dep: Arc<Level4>,
}

#[async_trait::async_trait]
impl Injectable for Level1 {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self)
	}
}

#[async_trait::async_trait]
impl Injectable for Level2 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_dep: resolve_injectable::<Level1>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for Level3 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_dep: resolve_injectable::<Level2>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for Level4 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_dep: resolve_injectable::<Level3>(ctx).await?,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for Level5 {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			_dep: resolve_injectable::<Level4>(ctx).await?,
		})
	}
}

#[tokio::test]
async fn test_direct_circular_dependency() {
	let ctx = test_context();
	let result = resolve_injectable::<DirectA>(&ctx).await;

	assert!(
		result.is_err(),
		"Direct circular dependency should be detected"
	);

	if let Err(DiError::CircularDependency(msg)) = result {
		assert!(
			msg.contains("DirectA") || msg.contains("DirectB"),
			"Error message should contain circular types: {}",
			msg
		);
	} else {
		panic!("Expected CircularDependency error");
	}
}

#[tokio::test]
async fn test_indirect_circular_dependency() {
	let ctx = test_context();
	let result = resolve_injectable::<ServiceA>(&ctx).await;

	assert!(
		result.is_err(),
		"Indirect circular dependency should be detected"
	);

	if let Err(DiError::CircularDependency(msg)) = result {
		let contains_services =
			msg.contains("ServiceA") || msg.contains("ServiceB") || msg.contains("ServiceC");
		assert!(
			contains_services,
			"Error message should contain circular types: {}",
			msg
		);
	} else {
		panic!("Expected CircularDependency error");
	}
}

#[tokio::test]
async fn test_self_dependency() {
	let ctx = test_context();
	let result = resolve_injectable::<SelfDependent>(&ctx).await;

	assert!(result.is_err(), "Self-dependency should be detected");
	assert!(
		matches!(result, Err(DiError::CircularDependency(_))),
		"Expected CircularDependency error"
	);
}

#[tokio::test]
async fn test_complex_circular_dependency() {
	let ctx = test_context();
	let result = resolve_injectable::<ComplexA>(&ctx).await;

	assert!(
		result.is_err(),
		"Complex circular dependency should be detected"
	);
	assert!(
		matches!(result, Err(DiError::CircularDependency(_))),
		"Expected CircularDependency error"
	);
}

#[tokio::test]
async fn test_no_circular_dependency_succeeds() {
	let ctx = test_context();
	let result = resolve_injectable::<NoCycleA>(&ctx).await;

	assert!(result.is_ok(), "Non-circular dependency should succeed");
	assert_eq!(result.unwrap()._b.value, 0);
}

#[tokio::test]
async fn test_deep_dependency_chain_without_cycle() {
	let ctx = test_context();
	let result = resolve_injectable::<Level5>(&ctx).await;

	assert!(
		result.is_ok(),
		"Deep dependency chain without cycle should succeed"
	);
}
