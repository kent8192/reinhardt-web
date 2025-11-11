//! Tests for circular dependency detection

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Circular dependency structures
#[derive(Clone, Debug)]
struct ServiceA {
	_name: String,
}

#[derive(Clone, Debug)]
struct ServiceB {
	_service_a: Arc<ServiceA>,
}

// This would create a circular dependency if ServiceA tried to inject ServiceB
#[async_trait::async_trait]
impl Injectable for ServiceA {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(ServiceA {
			_name: "ServiceA".to_string(),
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let service_a = ServiceA::inject(ctx).await?;
		Ok(ServiceB {
			_service_a: Arc::new(service_a),
		})
	}
}

#[tokio::test]
async fn test_no_circular_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// This should work fine - no circular dependency
	let _service_b = ServiceB::inject(&ctx).await.unwrap();
}

#[tokio::test]
async fn test_deep_dependency_chain() {
	#[derive(Clone, Debug)]
	struct Level1;

	#[derive(Clone, Debug)]
	struct Level2 {
		_l1: Arc<Level1>,
	}

	#[derive(Clone, Debug)]
	struct Level3 {
		_l2: Arc<Level2>,
	}

	#[derive(Clone, Debug)]
	struct Level4 {
		_l3: Arc<Level3>,
	}

	#[async_trait::async_trait]
	impl Injectable for Level1 {
		async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
			Ok(Level1)
		}
	}

	#[async_trait::async_trait]
	impl Injectable for Level2 {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let l1 = Level1::inject(ctx).await?;
			Ok(Level2 { _l1: Arc::new(l1) })
		}
	}

	#[async_trait::async_trait]
	impl Injectable for Level3 {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let l2 = Level2::inject(ctx).await?;
			Ok(Level3 { _l2: Arc::new(l2) })
		}
	}

	#[async_trait::async_trait]
	impl Injectable for Level4 {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let l3 = Level3::inject(ctx).await?;
			Ok(Level4 { _l3: Arc::new(l3) })
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Deep dependency chain should work
	let _level4 = Level4::inject(&ctx).await.unwrap();
}

#[tokio::test]
async fn test_multiple_dependencies() {
	#[derive(Clone, Debug)]
	struct DbConnection;

	#[derive(Clone, Debug)]
	struct CacheConnection;

	#[derive(Clone, Debug)]
	struct MultiService {
		_db: Arc<DbConnection>,
		_cache: Arc<CacheConnection>,
	}

	#[async_trait::async_trait]
	impl Injectable for DbConnection {
		async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
			Ok(DbConnection)
		}
	}

	#[async_trait::async_trait]
	impl Injectable for CacheConnection {
		async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
			Ok(CacheConnection)
		}
	}

	#[async_trait::async_trait]
	impl Injectable for MultiService {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let db = DbConnection::inject(ctx).await?;
			let cache = CacheConnection::inject(ctx).await?;
			Ok(MultiService {
				_db: Arc::new(db),
				_cache: Arc::new(cache),
			})
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Service with multiple independent dependencies
	let _service = MultiService::inject(&ctx).await.unwrap();
}

#[tokio::test]
async fn test_optional_dependency() {
	use reinhardt_di::DiError;

	#[derive(Clone, Debug)]
	struct OptionalService {
		_value: Option<String>,
	}

	#[async_trait::async_trait]
	impl Injectable for OptionalService {
		async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
			// Simulate a dependency that might not be available
			Err(DiError::NotFound("Optional dependency".to_string()))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// This should fail gracefully
	let result = OptionalService::inject(&ctx).await;
	assert!(result.is_err());
}
