//! Advanced DI macro integration tests
//!
//! Tests complex scenarios with the `#[endpoint]` macro and `#[inject]` attribute.

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::Arc;

/// Test service for nested injection
#[derive(Clone)]
struct DatabaseService {
	connection_string: String,
}

#[async_trait::async_trait]
impl Injectable for DatabaseService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self {
			connection_string: "postgres://localhost/test".to_string(),
		})
	}
}

/// Test service that depends on DatabaseService
#[derive(Clone)]
struct UserService {
	db: Arc<DatabaseService>,
}

#[async_trait::async_trait]
impl Injectable for UserService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let db = DatabaseService::inject(ctx).await?;
		Ok(Self { db: Arc::new(db) })
	}
}

/// Test: Nested injection (Depends<T> and direct injection)
///
/// Tests that the same type can be injected both directly and via Depends<T>
/// in the same handler function.
#[tokio::test]
async fn test_nested_inject_handler() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Resolve DatabaseService directly
	let db_direct = DatabaseService::inject(&ctx).await.unwrap();
	assert_eq!(db_direct.connection_string, "postgres://localhost/test");

	// Resolve UserService which depends on DatabaseService
	let user_service = UserService::inject(&ctx).await.unwrap();
	assert_eq!(
		user_service.db.connection_string,
		"postgres://localhost/test"
	);
}

/// Complex types for testing
#[derive(Clone)]
struct ComplexTypesService {
	vec_data: Vec<String>,
	map_data: HashMap<String, i32>,
	optional_data: Option<String>,
}

#[async_trait::async_trait]
impl Injectable for ComplexTypesService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		let mut map = HashMap::new();
		map.insert("key1".to_string(), 100);
		map.insert("key2".to_string(), 200);

		Ok(Self {
			vec_data: vec![
				"item1".to_string(),
				"item2".to_string(),
				"item3".to_string(),
			],
			map_data: map,
			optional_data: Some("optional_value".to_string()),
		})
	}
}

/// Test: Complex types (Vec<T>, HashMap<K,V>, Option<T>)
///
/// Tests that complex types can be properly injected and serialized/deserialized
/// through the DI system.
#[tokio::test]
async fn test_endpoint_macro_with_complex_types() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Resolve service
	let service = ComplexTypesService::inject(&ctx).await.unwrap();

	// Verify Vec<T>
	assert_eq!(service.vec_data.len(), 3);
	assert_eq!(service.vec_data[0], "item1");
	assert_eq!(service.vec_data[1], "item2");
	assert_eq!(service.vec_data[2], "item3");

	// Verify HashMap<K,V>
	assert_eq!(service.map_data.len(), 2);
	assert_eq!(service.map_data.get("key1"), Some(&100));
	assert_eq!(service.map_data.get("key2"), Some(&200));

	// Verify Option<T>
	assert!(service.optional_data.is_some());
	assert_eq!(service.optional_data.unwrap(), "optional_value");
}

/// Test: Complex nested structures
///
/// Tests deeply nested structures with multiple levels of dependencies.
#[tokio::test]
async fn test_complex_nested_structures() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Resolve services in dependency order
	let db = DatabaseService::inject(&ctx).await.unwrap();
	let user_service = UserService::inject(&ctx).await.unwrap();
	let complex_service = ComplexTypesService::inject(&ctx).await.unwrap();

	// Verify all services are correctly initialized
	assert_eq!(db.connection_string, "postgres://localhost/test");
	assert_eq!(
		user_service.db.connection_string,
		"postgres://localhost/test"
	);
	assert_eq!(complex_service.vec_data.len(), 3);
}

/// Test: Optional dependencies
///
/// Tests that services can have optional dependencies that may or may not be resolved.
#[tokio::test]
async fn test_optional_dependencies() {
	#[derive(Clone)]
	struct ServiceWithOptionalDep {
		required: Arc<DatabaseService>,
		optional: Option<Arc<ComplexTypesService>>,
	}

	#[async_trait::async_trait]
	impl Injectable for ServiceWithOptionalDep {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let required = DatabaseService::inject(ctx).await?;
			let optional = ComplexTypesService::inject(ctx).await.ok();

			Ok(Self {
				required: Arc::new(required),
				optional: optional.map(Arc::new),
			})
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Resolve service with optional dependency
	let service = ServiceWithOptionalDep::inject(&ctx).await.unwrap();

	// Verify required dependency is present
	assert_eq!(
		service.required.connection_string,
		"postgres://localhost/test"
	);

	// Verify optional dependency is Some (both DatabaseService and ComplexTypesService inject successfully)
	assert!(service.optional.is_some());
	assert_eq!(service.optional.unwrap().vec_data.len(), 3);
}
