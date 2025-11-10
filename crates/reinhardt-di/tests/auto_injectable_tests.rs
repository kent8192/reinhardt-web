//! Tests for automatic Injectable implementation

use reinhardt_di::{Depends, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Default, Clone, Debug, PartialEq)]
struct SimpleConfig {
	host: String,
	port: u16,
}

#[derive(Default, Clone)]
struct AnotherConfig {
	api_key: String,
}

#[tokio::test]
async fn test_auto_injectable_simple() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton_scope);

	// SimpleConfig should be automatically injectable
	let config = SimpleConfig::inject(&ctx).await.unwrap();
	assert_eq!(config.host, "");
	assert_eq!(config.port, 0);
}

#[tokio::test]
async fn test_auto_injectable_with_depends() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton_scope);

	// Should work with Depends wrapper
	let depends_config = Depends::<SimpleConfig>::builder()
		.resolve(&ctx)
		.await
		.unwrap();
	assert_eq!(depends_config.host, "");
	assert_eq!(depends_config.port, 0);
}

#[tokio::test]
async fn test_auto_injectable_caching() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton_scope);

	// First injection - creates new instance
	let config1 = SimpleConfig::inject(&ctx).await.unwrap();

	// Second injection - should get cached instance
	let config2 = SimpleConfig::inject(&ctx).await.unwrap();

	// They should be equal (same default values)
	assert_eq!(config1, config2);
}

#[tokio::test]
async fn test_multiple_auto_injectable_types() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton_scope);

	// Multiple different types should work
	let config1 = SimpleConfig::inject(&ctx).await.unwrap();
	let config2 = AnotherConfig::inject(&ctx).await.unwrap();

	assert_eq!(config1.host, "");
	assert_eq!(config2.api_key, "");
}

// Custom implementation should still work
struct CustomInjectable {
	value: i32,
}

#[async_trait::async_trait]
impl Injectable for CustomInjectable {
	async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(CustomInjectable { value: 42 })
	}
}

#[tokio::test]
async fn test_custom_injectable_override() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton_scope);

	// Custom implementation should be used
	let custom = CustomInjectable::inject(&ctx).await.unwrap();
	assert_eq!(custom.value, 42);
}
