//! Basic auto injection tests
//!
//! Tests for #[injectable] and #[injectable_factory] macros

use reinhardt_di::{
	injectable, injectable_factory, global_registry, DependencyScope, InjectionContext,
	SingletonScope,
};
use std::sync::Arc;

/// Simple config struct with #[injectable] macro
#[injectable(scope = "singleton")]
#[derive(Clone, Debug)]
struct AppConfig {
	database_url: String,
	api_key: String,
}

impl Default for AppConfig {
	fn default() -> Self {
		Self {
			database_url: "postgres://localhost:5432/test".to_string(),
			api_key: "test-key-12345".to_string(),
		}
	}
}

/// Database connection using #[injectable_factory] macro
#[derive(Clone, Debug)]
struct DatabaseConnection {
	url: String,
	connected: bool,
}

#[injectable_factory(scope = "singleton")]
async fn create_database(#[inject] config: Arc<AppConfig>) -> DatabaseConnection {
	DatabaseConnection { url: config.database_url.clone(), connected: true }
}

#[tokio::test]
async fn test_injectable_macro_registration() {
	// Initialize registry
	let _registry = global_registry();

	// Verify AppConfig is registered
	assert!(
		_registry.is_registered::<AppConfig>(),
		"AppConfig should be registered via #[injectable] macro"
	);

	// Verify scope is Singleton
	assert_eq!(
		_registry.get_scope::<AppConfig>(),
		Some(DependencyScope::Singleton),
		"AppConfig should have Singleton scope"
	);
}

#[tokio::test]
async fn test_injectable_factory_registration() {
	let _registry = global_registry();

	// Verify DatabaseConnection is registered
	assert!(
		_registry.is_registered::<DatabaseConnection>(),
		"DatabaseConnection should be registered via #[injectable_factory] macro"
	);

	// Verify scope is Singleton
	assert_eq!(
		_registry.get_scope::<DatabaseConnection>(),
		Some(DependencyScope::Singleton),
		"DatabaseConnection should have Singleton scope"
	);
}

#[tokio::test]
async fn test_resolve_injectable_struct() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Resolve AppConfig
	let config = ctx.resolve::<AppConfig>().await;

	assert!(
		config.is_ok(),
		"Should successfully resolve AppConfig: {:?}",
		config.err()
	);

	let config = config.unwrap();
	assert_eq!(config.database_url, "postgres://localhost:5432/test");
	assert_eq!(config.api_key, "test-key-12345");
}

#[tokio::test]
async fn test_resolve_injectable_factory_with_dependency() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// Resolve DatabaseConnection which depends on AppConfig
	let db = ctx.resolve::<DatabaseConnection>().await;

	assert!(
		db.is_ok(),
		"Should successfully resolve DatabaseConnection: {:?}",
		db.err()
	);

	let db = db.unwrap();
	assert_eq!(db.url, "postgres://localhost:5432/test");
	assert_eq!(db.connected, true);
}

#[tokio::test]
async fn test_singleton_caching() {
	let singleton_scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton_scope).build();

	// First resolution
	let config1 = ctx.resolve::<AppConfig>().await.unwrap();

	// Second resolution should use cached instance
	let config2 = ctx.resolve::<AppConfig>().await.unwrap();

	// Verify same instance (Arc pointer equality)
	assert!(
		Arc::ptr_eq(&config1, &config2),
		"Singleton dependencies should return the same Arc instance"
	);
}
