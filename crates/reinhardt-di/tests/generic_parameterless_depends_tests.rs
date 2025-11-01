//! FastAPI generic parameterless depends tests translated to Rust
//!
//! Based on: fastapi/tests/test_generic_parameterless_depends.py
//!
//! These tests verify that:
//! 1. Generic dependencies without parameters can infer the type from usage
//! 2. Different type parameters create different dependency instances

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::any::type_name;
use std::sync::Arc;

// Type A
struct A;

#[async_trait::async_trait]
impl Injectable for A {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(A)
	}
}

// Type B
struct B;

#[async_trait::async_trait]
impl Injectable for B {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(B)
	}
}

// Generic dependency wrapper (simulates Annotated[T, Depends()])
// This is handled automatically by Rust's type system
struct Dep<T> {
	inner: T,
}

impl<T> Dep<T> {
	fn new(inner: T) -> Self {
		Dep { inner }
	}

	fn type_name(&self) -> &'static str {
		type_name::<T>().split("::").last().unwrap_or("")
	}
}

#[tokio::test]
async fn test_generic_parameterless_depends_type_a() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject type A
	let a = A::inject(&ctx).await.unwrap();
	let dep = Dep::new(a);

	assert_eq!(dep.type_name(), "A");
}

#[tokio::test]
async fn test_generic_parameterless_depends_type_b() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject type B
	let b = B::inject(&ctx).await.unwrap();
	let dep = Dep::new(b);

	assert_eq!(dep.type_name(), "B");
}

#[tokio::test]
async fn test_different_types_create_different_instances() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject both types
	let a = A::inject(&ctx).await.unwrap();
	let b = B::inject(&ctx).await.unwrap();

	let dep_a = Dep::new(a);
	let dep_b = Dep::new(b);

	// Verify they have different types
	assert_eq!(dep_a.type_name(), "A");
	assert_eq!(dep_b.type_name(), "B");
	assert_ne!(dep_a.type_name(), dep_b.type_name());
}

// Test with more complex generic usage
struct GenericService<T> {
	dependency: T,
}

impl<T> GenericService<T> {
	fn new(dependency: T) -> Self {
		GenericService { dependency }
	}
}

#[async_trait::async_trait]
impl<T: Injectable + Send + Sync> Injectable for GenericService<T> {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let dependency = T::inject(ctx).await?;
		Ok(GenericService::new(dependency))
	}
}

#[tokio::test]
async fn test_generic_service_with_type_a() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject GenericService<A>
	let service = GenericService::<A>::inject(&ctx).await.unwrap();
	let dep = Dep::new(service.dependency);

	assert_eq!(dep.type_name(), "A");
}

#[tokio::test]
async fn test_generic_service_with_type_b() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Inject GenericService<B>
	let service = GenericService::<B>::inject(&ctx).await.unwrap();
	let dep = Dep::new(service.dependency);

	assert_eq!(dep.type_name(), "B");
}
