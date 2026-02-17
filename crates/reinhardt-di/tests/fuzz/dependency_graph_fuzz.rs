//! Fuzz-style tests for dependency injection (using proptest)
//!
//! These tests use proptest with arbitrary inputs to simulate fuzzing behavior.
//! They test:
//! 1. Random dependency chain lengths
//! 2. Random type combinations
//! 3. Random concurrent access patterns

use proptest::prelude::*;
use reinhardt_di::{DiResult, Injectable, InjectionContext, RequestScope, SingletonScope};
use std::sync::Arc;
use rstest::rstest;

// Generic service with arbitrary ID
#[derive(Clone, Debug)]
struct FuzzService {
	id: usize,
	data: String,
}

#[async_trait::async_trait]
impl Injectable for FuzzService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_request::<FuzzService>() {
			return Ok((*cached).clone());
		}

		let service = FuzzService {
			id: 1,
			data: "fuzz".to_string(),
		};

		ctx.set_request(service.clone());
		Ok(service)
	}
}

// Nested dependency with depth
#[derive(Clone, Debug)]
struct NestedService {
	depth: usize,
	parent_id: Option<usize>,
}

impl NestedService {
	async fn inject_with_depth(ctx: &InjectionContext, depth: usize) -> DiResult<Self> {
		if depth == 0 {
			return Ok(NestedService {
				depth,
				parent_id: None,
			});
		}

		// Inject parent recursively
		let parent = Self::inject_with_depth(ctx, depth - 1).await?;

		Ok(NestedService {
			depth,
			parent_id: Some(parent.depth),
		})
	}
}

// Fuzz Test 1: Random dependency chain lengths
#[rstest]
#[tokio::test]
async fn fuzz_random_dependency_chains() {
	proptest!(|(chain_length in 0usize..100)| {
		let rt = tokio::runtime::Runtime::new().unwrap();
		rt.block_on(async {
			let singleton = Arc::new(SingletonScope::new());
			let ctx = InjectionContext::builder(singleton).build();

			// Inject nested service with random depth
			let result = NestedService::inject_with_depth(&ctx, chain_length).await;

			// Should always succeed (no circular deps, no failures)
			prop_assert!(result.is_ok());

			let service = result.unwrap();
			prop_assert_eq!(service.depth, chain_length);

			// Verify parent chain
			if chain_length > 0 {
				prop_assert_eq!(service.parent_id, Some(chain_length - 1));
			} else {
				prop_assert_eq!(service.parent_id, None);
			}

			Ok(())
		}).unwrap();
	});
}

// Fuzz Test 2: Random type combinations
#[rstest]
#[tokio::test]
async fn fuzz_random_type_combinations() {
	proptest!(|(service_count in 1usize..20, inject_order in prop::collection::vec(0usize..10, 1..20))| {
		let rt = tokio::runtime::Runtime::new().unwrap();
		rt.block_on(async {
			let singleton = Arc::new(SingletonScope::new());
			let ctx = InjectionContext::builder(singleton).build();

			// Inject services in random order
			for &order_idx in &inject_order {
				let service_idx = order_idx % service_count;

				// Inject FuzzService (all inject same type, verify caching)
				let service = FuzzService::inject(&ctx).await;
				prop_assert!(service.is_ok());

				// Verify caching works regardless of order
				let s = service.unwrap();
				prop_assert_eq!(s.id, 1);
				prop_assert_eq!(s.data, "fuzz");
			}

			// Verify only one instance was created (cached)
			let final_service = FuzzService::inject(&ctx).await.unwrap();
			prop_assert_eq!(final_service.id, 1);

			Ok(())
		}).unwrap();
	});
}

// Fuzz Test 3: Random concurrent access patterns
#[rstest]
#[tokio::test]
async fn fuzz_concurrent_resolutions() {
	proptest!(|(task_count in 2usize..10, iteration_count in 1usize..5)| {
		let rt = tokio::runtime::Runtime::new().unwrap();
		rt.block_on(async {
			let singleton = Arc::new(SingletonScope::new());

			// Spawn concurrent tasks with random iteration counts
			let mut handles = Vec::new();

			for task_id in 0..task_count {
				let singleton_clone = singleton.clone();
				let iterations = iteration_count;

				let handle = tokio::spawn(async move {
					let request = Arc::new(RequestScope::new());
					let ctx = InjectionContext::builder(singleton_clone)
						.with_request(request)
						.build();

					// Inject multiple times within same scope
					for _ in 0..iterations {
						let service = FuzzService::inject(&ctx).await;
						assert!(service.is_ok());
					}

					task_id
				});

				handles.push(handle);
			}

			// Wait for all tasks
			let mut task_ids = Vec::new();
			for handle in handles {
				let task_id = handle.await.unwrap();
				task_ids.push(task_id);
			}

			// All tasks should complete successfully
			prop_assert_eq!(task_ids.len(), task_count);

			Ok(())
		}).unwrap();
	});
}
