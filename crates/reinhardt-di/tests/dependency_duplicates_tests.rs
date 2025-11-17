//! FastAPI duplicate dependency tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_duplicates.py
//!
//! These tests verify that:
//! 1. Dependencies with the same type can be reused (cached)
//! 2. Sub-dependencies with the same type are deduplicated
//! 3. Different dependency functions for the same type create separate instances

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Item type used in multiple dependencies
#[derive(Clone, Debug, PartialEq)]
struct Item {
	data: String,
}

impl Item {
	fn new(data: &str) -> Self {
		Self {
			data: data.to_string(),
		}
	}
}

// Primary injectable for Item
#[async_trait::async_trait]
impl Injectable for Item {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check cache first
		if let Some(cached) = ctx.get_request::<Item>() {
			return Ok((*cached).clone());
		}

		let item = Item::new("default_item");
		ctx.set_request(item.clone());
		Ok(item)
	}
}

// Duplicate dependency - returns the same Item via cache
#[derive(Clone)]
struct DuplicateDependency {
	item: Arc<Item>,
}

#[async_trait::async_trait]
impl Injectable for DuplicateDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let item = Item::inject(ctx).await?;
		Ok(DuplicateDependency {
			item: Arc::new(item),
		})
	}
}

#[tokio::test]
async fn test_duplicate_dependency_cached() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Create an item first
	let item1 = Item::inject(&ctx).await.unwrap();

	// DuplicateDependency should get the same item from cache
	let dup_dep = DuplicateDependency::inject(&ctx).await.unwrap();

	assert_eq!(*dup_dep.item, item1);
	assert_eq!(dup_dep.item.data, "default_item");
}

// Non-duplicate dependency - creates a new Item wrapper
#[derive(Clone)]
struct Item2 {
	data: String,
}

impl Item2 {
	fn new(data: &str) -> Self {
		Self {
			data: data.to_string(),
		}
	}
}

#[async_trait::async_trait]
impl Injectable for Item2 {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Item2::new("item2_data"))
	}
}

#[derive(Clone)]
struct NoDuplicateDependency {
	item: Arc<Item>,
	item2: Arc<Item2>,
}

#[async_trait::async_trait]
impl Injectable for NoDuplicateDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let item = Item::inject(ctx).await?;
		let item2 = Item2::inject(ctx).await?;
		Ok(NoDuplicateDependency {
			item: Arc::new(item),
			item2: Arc::new(item2),
		})
	}
}

#[tokio::test]
async fn test_no_duplicate_different_types() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let no_dup = NoDuplicateDependency::inject(&ctx).await.unwrap();

	assert_eq!(no_dup.item.data, "default_item");
	assert_eq!(no_dup.item2.data, "item2_data");
}

// Sub-dependency with duplicates
#[derive(Clone)]
struct SubDuplicateDependency {
	item1: Arc<Item>,
	item2: Arc<Item>,
}

#[async_trait::async_trait]
impl Injectable for SubDuplicateDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Both should get the same item from cache
		let item1 = Item::inject(ctx).await?;
		let item2 = Item::inject(ctx).await?;

		Ok(SubDuplicateDependency {
			item1: Arc::new(item1),
			item2: Arc::new(item2),
		})
	}
}

#[tokio::test]
async fn test_sub_duplicate_dependency() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let sub_dup = SubDuplicateDependency::inject(&ctx).await.unwrap();

	// Both items should be the same (from cache)
	assert_eq!(*sub_dup.item1, *sub_dup.item2);
	assert_eq!(sub_dup.item1.data, "default_item");
}

// Aggregate that uses item multiple times via different paths
#[derive(Clone)]
struct AggregateWithDuplicates {
	direct_item: Arc<Item>,
	dup_dep: Arc<DuplicateDependency>,
	sub_dup: Arc<SubDuplicateDependency>,
}

#[async_trait::async_trait]
impl Injectable for AggregateWithDuplicates {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let direct_item = Item::inject(ctx).await?;
		let dup_dep = DuplicateDependency::inject(ctx).await?;
		let sub_dup = SubDuplicateDependency::inject(ctx).await?;

		Ok(AggregateWithDuplicates {
			direct_item: Arc::new(direct_item),
			dup_dep: Arc::new(dup_dep),
			sub_dup: Arc::new(sub_dup),
		})
	}
}

#[tokio::test]
async fn test_aggregate_with_duplicates() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let aggregate = AggregateWithDuplicates::inject(&ctx).await.unwrap();

	// All items should be the same (from cache)
	assert_eq!(*aggregate.direct_item, *aggregate.dup_dep.item);
	assert_eq!(*aggregate.direct_item, *aggregate.sub_dup.item1);
	assert_eq!(*aggregate.direct_item, *aggregate.sub_dup.item2);
	assert_eq!(aggregate.direct_item.data, "default_item");
}

// Custom item dependency (bypasses cache intentionally)
#[derive(Clone)]
struct CustomItemDependency {
	item: Item,
}

#[async_trait::async_trait]
impl Injectable for CustomItemDependency {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		// Create a new item without using cache
		Ok(CustomItemDependency {
			item: Item::new("custom_item"),
		})
	}
}

#[tokio::test]
async fn test_custom_item_dependency_no_cache() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Get default cached item
	let default_item = Item::inject(&ctx).await.unwrap();

	// Get custom item (not cached)
	let custom = CustomItemDependency::inject(&ctx).await.unwrap();

	// Should be different
	assert_ne!(default_item, custom.item);
	assert_eq!(default_item.data, "default_item");
	assert_eq!(custom.item.data, "custom_item");
}

// List of items dependency
#[derive(Clone)]
struct ItemList {
	items: Vec<Item>,
}

#[async_trait::async_trait]
impl Injectable for ItemList {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// All should get the same cached item
		let item1 = Item::inject(ctx).await?;
		let item2 = Item::inject(ctx).await?;
		let item3 = Item::inject(ctx).await?;

		Ok(ItemList {
			items: vec![item1, item2, item3],
		})
	}
}

#[tokio::test]
async fn test_item_list_all_cached() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let list = ItemList::inject(&ctx).await.unwrap();

	// All items should be the same
	assert_eq!(list.items.len(), 3);
	assert_eq!(list.items[0], list.items[1]);
	assert_eq!(list.items[1], list.items[2]);
	assert_eq!(list.items[0].data, "default_item");
}

// Testing with different data types that happen to have same shape
#[derive(Clone, Debug, PartialEq)]
struct User {
	name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Product {
	name: String,
}

#[async_trait::async_trait]
impl Injectable for User {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_request::<User>() {
			return Ok((*cached).clone());
		}
		let user = User {
			name: "John".to_string(),
		};
		ctx.set_request(user.clone());
		Ok(user)
	}
}

#[async_trait::async_trait]
impl Injectable for Product {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_request::<Product>() {
			return Ok((*cached).clone());
		}
		let product = Product {
			name: "Widget".to_string(),
		};
		ctx.set_request(product.clone());
		Ok(product)
	}
}

#[derive(Clone)]
struct UserProductService {
	user: Arc<User>,
	product: Arc<Product>,
}

#[async_trait::async_trait]
impl Injectable for UserProductService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let user = User::inject(ctx).await?;
		let product = Product::inject(ctx).await?;
		Ok(UserProductService {
			user: Arc::new(user),
			product: Arc::new(product),
		})
	}
}

#[tokio::test]
async fn test_different_types_separate_caches() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let service = UserProductService::inject(&ctx).await.unwrap();

	// Different types, different caches
	assert_eq!(service.user.name, "John");
	assert_eq!(service.product.name, "Widget");

	// Getting them again should return cached values
	let user2 = User::inject(&ctx).await.unwrap();
	let product2 = Product::inject(&ctx).await.unwrap();

	assert_eq!(*service.user, user2);
	assert_eq!(*service.product, product2);
}
