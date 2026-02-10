//! Unit tests for Injectable trait

use async_trait::async_trait;
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext};
use reinhardt_test::fixtures::*;
use rstest::*;
use std::sync::Arc;

// Test type definitions
#[derive(Clone, Debug, PartialEq)]
struct SimpleService {
	value: String,
}

#[async_trait]
impl Injectable for SimpleService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(SimpleService {
			value: "simple".to_string(),
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
struct DependentService {
	dependency: SimpleService,
}

#[async_trait]
impl Injectable for DependentService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let dependency = SimpleService::inject(ctx).await?;
		Ok(DependentService { dependency })
	}
}

#[derive(Clone, Debug)]
struct FailingService;

#[async_trait]
impl Injectable for FailingService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Err(DiError::Internal {
			message: "intentional failure".to_string(),
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
struct CachedService {
	id: u32,
}

static mut CACHED_SERVICE_COUNTER: u32 = 0;

#[async_trait]
impl Injectable for CachedService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		unsafe {
			CACHED_SERVICE_COUNTER += 1;
			Ok(CachedService {
				id: CACHED_SERVICE_COUNTER,
			})
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
struct SingletonCachedService {
	id: u32,
}

static mut SINGLETON_COUNTER: u32 = 0;

#[async_trait]
impl Injectable for SingletonCachedService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check singleton scope first
		if let Some(cached) = ctx.get_singleton::<SingletonCachedService>() {
			return Ok((*cached).clone());
		}

		// Create new instance
		unsafe {
			SINGLETON_COUNTER += 1;
			let service = SingletonCachedService {
				id: SINGLETON_COUNTER,
			};
			ctx.set_singleton(service.clone());
			Ok(service)
		}
	}
}

#[rstest]
#[tokio::test]
async fn injectable_inject_called_successfully(injection_context: InjectionContext) {
	// Act
	let service = SimpleService::inject(&injection_context).await;

	// Assert
	assert!(service.is_ok());
	assert_eq!(service.unwrap().value, "simple");
}

#[rstest]
#[tokio::test]
async fn injectable_with_dependencies(injection_context: InjectionContext) {
	// Act
	let service = DependentService::inject(&injection_context).await;

	// Assert
	assert!(service.is_ok());
	let service = service.unwrap();
	assert_eq!(service.dependency.value, "simple");
}

#[rstest]
#[tokio::test]
async fn injectable_returns_error(injection_context: InjectionContext) {
	// Act
	let result = FailingService::inject(&injection_context).await;

	// Assert
	assert!(result.is_err());
	match result {
		Err(DiError::Internal { message }) => {
			assert_eq!(message, "intentional failure");
		}
		_ => panic!("Expected DiError::Internal"),
	}
}

#[serial_test::serial(cached_service_counter)]
#[tokio::test]
async fn injectable_cached_in_request_scope() {
	// Arrange
	let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
	let injection_context = InjectionContext::builder(singleton_scope).build();
	unsafe {
		CACHED_SERVICE_COUNTER = 0;
	}

	// Act - First injection
	let service1 = CachedService::inject(&injection_context).await.unwrap();
	injection_context.set_request(service1.clone());

	// Get from request scope
	let cached: Option<Arc<CachedService>> = injection_context.get_request();

	// Assert
	assert!(cached.is_some());
	assert_eq!(cached.unwrap().id, service1.id);
	unsafe {
		assert_eq!(CACHED_SERVICE_COUNTER, 1);
	}
}

#[serial_test::serial(singleton_counter)]
#[tokio::test]
async fn injectable_singleton_cached() {
	// Arrange
	let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
	let injection_context = InjectionContext::builder(singleton_scope).build();
	unsafe {
		SINGLETON_COUNTER = 0;
	}

	// Act - First injection
	let service1 = SingletonCachedService::inject(&injection_context)
		.await
		.unwrap();

	// Second injection - get cache from singleton scope
	let service2 = SingletonCachedService::inject(&injection_context)
		.await
		.unwrap();

	// Assert
	assert_eq!(service1.id, service2.id);
	unsafe {
		// Counter is incremented only once
		assert_eq!(SINGLETON_COUNTER, 1);
	}
}
