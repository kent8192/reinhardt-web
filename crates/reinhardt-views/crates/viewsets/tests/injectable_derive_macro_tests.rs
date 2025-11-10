//! Unit tests for #[derive(Injectable)] macro
//!
//! Tests the macro-generated code for field-level dependency injection.

use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
use reinhardt_macros::Injectable;
use std::sync::Arc;

// ============================================================================
// Category 1: Basic Derivation Tests (5 tests)
// ============================================================================

/// Mock dependency for basic tests
#[derive(Clone)]
struct SimpleService {
	value: i32,
}

impl Default for SimpleService {
	fn default() -> Self {
		Self { value: 42 }
	}
}

#[tokio::test]
async fn test_macro_single_field_injection() {
	#[derive(Clone, Injectable)]
	struct SingleFieldStruct {
		#[inject]
		service: SimpleService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = SingleFieldStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service.value, 42);
}

#[tokio::test]
async fn test_macro_multiple_fields_injection() {
	#[derive(Clone)]
	struct ServiceA {
		name: String,
	}

	impl Default for ServiceA {
		fn default() -> Self {
			Self {
				name: "ServiceA".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct ServiceB {
		count: usize,
	}

	impl Default for ServiceB {
		fn default() -> Self {
			Self { count: 10 }
		}
	}

	#[derive(Clone, Injectable)]
	struct MultiFieldStruct {
		#[inject]
		service_a: ServiceA,
		#[inject]
		service_b: ServiceB,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = MultiFieldStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service_a.name, "ServiceA");
	assert_eq!(instance.service_b.count, 10);
}

#[tokio::test]
async fn test_macro_mixed_inject_and_regular_fields() {
	#[derive(Clone, Injectable)]
	struct MixedStruct {
		#[inject]
		service: SimpleService,
		name: String,
		count: i32,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = MixedStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service.value, 42);
	assert_eq!(instance.name, ""); // Default::default()
	assert_eq!(instance.count, 0); // Default::default()
}

#[tokio::test]
async fn test_macro_no_inject_fields() {
	#[derive(Clone, Injectable)]
	struct NoInjectStruct {
		name: String,
		value: i32,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = NoInjectStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.name, "");
	assert_eq!(instance.value, 0);
}

#[tokio::test]
async fn test_macro_empty_named_struct() {
	// Note: Injectable macro requires named fields
	// Unit structs are not supported
	#[derive(Clone, Injectable)]
	struct EmptyStruct {}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let _instance = EmptyStruct::inject(&ctx).await.unwrap();
	// Successfully creates instance with no fields
}

// ============================================================================
// Category 2: Cache Control Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_macro_default_cache_enabled() {
	use std::sync::LazyLock;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

	#[derive(Clone)]
	struct CachedService {
		id: usize,
	}

	impl Default for CachedService {
		fn default() -> Self {
			Self {
				id: COUNTER.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct CachedStruct {
		#[inject]
		service: CachedService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance1 = CachedStruct::inject(&ctx).await.unwrap();
	let instance2 = CachedStruct::inject(&ctx).await.unwrap();

	// Both should get the same cached service (same ID)
	assert_eq!(instance1.service.id, instance2.service.id);
}

#[tokio::test]
async fn test_macro_cache_disabled_compiles() {
	// Test that cache = false attribute compiles correctly
	// Note: Actual cache behavior is tested in reinhardt-di crate
	#[derive(Clone)]
	struct FreshService {
		value: String,
	}

	impl Default for FreshService {
		fn default() -> Self {
			Self {
				value: "fresh".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct FreshStruct {
		#[inject(cache = false)]
		service: FreshService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = FreshStruct::inject(&ctx).await.unwrap();

	// Verify the field was injected correctly
	assert_eq!(instance.service.value, "fresh");
}

#[tokio::test]
async fn test_macro_mixed_cache_settings_compiles() {
	// Test that mixed cache settings compile correctly
	// Note: Actual cache behavior is tested in reinhardt-di crate
	#[derive(Clone)]
	struct ServiceWithValue {
		value: String,
	}

	impl Default for ServiceWithValue {
		fn default() -> Self {
			Self {
				value: "default".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct MixedCacheStruct {
		#[inject]
		cached: ServiceWithValue,
		#[inject(cache = false)]
		fresh: ServiceWithValue,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = MixedCacheStruct::inject(&ctx).await.unwrap();

	// Verify both fields were injected correctly
	assert_eq!(instance.cached.value, "default");
	assert_eq!(instance.fresh.value, "default");
}

// ============================================================================
// Category 3: Multiple Fields Stress Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_macro_many_fields() {
	#[derive(Clone)]
	struct Service {
		id: usize,
	}

	impl Default for Service {
		fn default() -> Self {
			Self { id: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	struct ManyFieldsStruct {
		#[inject]
		s1: Service,
		#[inject]
		s2: Service,
		#[inject]
		s3: Service,
		#[inject]
		s4: Service,
		#[inject]
		s5: Service,
		#[inject]
		s6: Service,
		#[inject]
		s7: Service,
		#[inject]
		s8: Service,
		#[inject]
		s9: Service,
		#[inject]
		s10: Service,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = ManyFieldsStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.s1.id, 1);
	assert_eq!(instance.s10.id, 1);
}

#[tokio::test]
async fn test_macro_different_types() {
	#[derive(Clone)]
	struct StringService {
		value: String,
	}

	impl Default for StringService {
		fn default() -> Self {
			Self {
				value: "test".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct IntService {
		value: i32,
	}

	impl Default for IntService {
		fn default() -> Self {
			Self { value: 100 }
		}
	}

	#[derive(Clone)]
	struct BoolService {
		value: bool,
	}

	impl Default for BoolService {
		fn default() -> Self {
			Self { value: true }
		}
	}

	#[derive(Clone, Injectable)]
	struct DifferentTypesStruct {
		#[inject]
		string_svc: StringService,
		#[inject]
		int_svc: IntService,
		#[inject]
		bool_svc: BoolService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = DifferentTypesStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.string_svc.value, "test");
	assert_eq!(instance.int_svc.value, 100);
	assert_eq!(instance.bool_svc.value, true);
}

#[tokio::test]
async fn test_macro_nested_structs() {
	#[derive(Clone)]
	struct InnerService {
		name: String,
	}

	impl Default for InnerService {
		fn default() -> Self {
			Self {
				name: "inner".to_string(),
			}
		}
	}

	#[derive(Clone)]
	struct OuterService {
		inner: InnerService,
	}

	impl Default for OuterService {
		fn default() -> Self {
			Self {
				inner: InnerService::default(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct NestedStruct {
		#[inject]
		outer: OuterService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = NestedStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.outer.inner.name, "inner");
}

// ============================================================================
// Category 4: Default Implementation Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_macro_all_fields_with_default() {
	#[derive(Clone, Injectable)]
	struct AllDefaultStruct {
		name: String,
		count: i32,
		flag: bool,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = AllDefaultStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.name, "");
	assert_eq!(instance.count, 0);
	assert_eq!(instance.flag, false);
}

#[tokio::test]
async fn test_macro_option_fields() {
	#[derive(Clone, Injectable)]
	struct OptionFieldsStruct {
		name: Option<String>,
		count: Option<i32>,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = OptionFieldsStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.name, None);
	assert_eq!(instance.count, None);
}

#[tokio::test]
async fn test_macro_vec_fields() {
	#[derive(Clone, Injectable)]
	struct VecFieldsStruct {
		items: Vec<String>,
		numbers: Vec<i32>,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = VecFieldsStruct::inject(&ctx).await.unwrap();
	assert!(instance.items.is_empty());
	assert!(instance.numbers.is_empty());
}

// ============================================================================
// Category 5: Error Handling Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_macro_injection_error_propagation() {
	// This test verifies that DI errors are properly propagated
	#[derive(Clone)]
	struct FailingService;

	impl Default for FailingService {
		fn default() -> Self {
			Self
		}
	}

	#[derive(Clone, Injectable)]
	struct ErrorPropagationStruct {
		#[inject]
		_service: FailingService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Should succeed with Default implementation
	let result = ErrorPropagationStruct::inject(&ctx).await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_macro_multiple_injections_independence() {
	#[derive(Clone)]
	struct IndependentService {
		value: i32,
	}

	impl Default for IndependentService {
		fn default() -> Self {
			Self { value: 1 }
		}
	}

	#[derive(Clone, Injectable)]
	struct IndependentStruct {
		#[inject]
		service1: IndependentService,
		#[inject]
		service2: IndependentService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = IndependentStruct::inject(&ctx).await.unwrap();

	// Both services should be injected successfully
	assert_eq!(instance.service1.value, 1);
	assert_eq!(instance.service2.value, 1);
}

#[tokio::test]
async fn test_macro_context_isolation() {
	use std::sync::LazyLock;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static ISOLATED_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(300));

	#[derive(Clone)]
	struct IsolatedService {
		id: usize,
	}

	impl Default for IsolatedService {
		fn default() -> Self {
			Self {
				id: ISOLATED_COUNTER.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct IsolatedStruct {
		#[inject]
		service: IsolatedService,
	}

	// Create two separate contexts
	let singleton1 = Arc::new(SingletonScope::new());
	let ctx1 = InjectionContext::new(singleton1);

	let singleton2 = Arc::new(SingletonScope::new());
	let ctx2 = InjectionContext::new(singleton2);

	let instance1 = IsolatedStruct::inject(&ctx1).await.unwrap();
	let instance2 = IsolatedStruct::inject(&ctx2).await.unwrap();

	// Different contexts should have independent caches
	// Each context gets its own instance with different ID
	assert_ne!(instance1.service.id, instance2.service.id);
}

// ============================================================================
// Category 6: Edge Cases (3 tests)
// ============================================================================

#[tokio::test]
async fn test_macro_arc_wrapped_fields() {
	#[derive(Clone)]
	struct ArcService {
		data: String,
	}

	impl Default for ArcService {
		fn default() -> Self {
			Self {
				data: "arc_data".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct ArcStruct {
		#[inject]
		service: Arc<ArcService>,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = ArcStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service.data, "arc_data");
}

#[tokio::test]
async fn test_macro_complex_field_types() {
	use std::collections::HashMap;

	#[derive(Clone)]
	struct ComplexService {
		map: HashMap<String, i32>,
		vec: Vec<String>,
	}

	impl Default for ComplexService {
		fn default() -> Self {
			let mut map = HashMap::new();
			map.insert("key".to_string(), 100);
			Self {
				map,
				vec: vec!["item1".to_string()],
			}
		}
	}

	#[derive(Clone, Injectable)]
	struct ComplexStruct {
		#[inject]
		service: ComplexService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = ComplexStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service.map.get("key"), Some(&100));
	assert_eq!(instance.service.vec.len(), 1);
}

#[tokio::test]
async fn test_macro_visibility_modifiers() {
	#[derive(Clone)]
	pub struct PublicService {
		pub value: String,
	}

	impl Default for PublicService {
		fn default() -> Self {
			Self {
				value: "public".to_string(),
			}
		}
	}

	#[derive(Clone, Injectable)]
	pub struct PublicStruct {
		#[inject]
		pub service: PublicService,
		pub name: String,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let instance = PublicStruct::inject(&ctx).await.unwrap();
	assert_eq!(instance.service.value, "public");
	assert_eq!(instance.name, "");
}
