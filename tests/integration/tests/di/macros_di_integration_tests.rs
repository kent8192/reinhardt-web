//! Dependency injection macros integration tests
//!
//! Integration tests for reinhardt-macros' dependency injection macros
//! (#[endpoint] and #[use_injection]) working with reinhardt-di's
//! injection context and scope management.
//!
//! These tests verify that the procedural macros correctly integrate with
//! the DI system to inject dependencies into handler functions.

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt_macros::{endpoint, use_injection};
use std::sync::Arc;

// ========== Test Data Structures ==========

#[derive(Clone, Default, Debug, PartialEq)]
struct Database {
	connection_string: String,
}

#[derive(Clone, Default, Debug)]
struct Config {
	api_key: String,
	max_connections: usize,
}

#[derive(Clone, Default, Debug)]
struct Logger {
	level: String,
}

#[derive(Clone)]
struct CustomService {
	value: i32,
}

#[async_trait::async_trait]
impl Injectable for CustomService {
	async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(CustomService { value: 42 })
	}
}

// ========== Endpoint Macro Tests ==========

#[endpoint]
async fn handler_with_inject(
	#[inject] db: Database,
	regular_param: String,
) -> Result<String, reinhardt_di::DiError> {
	Ok(format!("db: {:?}, param: {}", db, regular_param))
}

#[endpoint]
async fn handler_multiple_inject(
	#[inject] db: Database,
	#[inject] config: Config,
	regular_param: i32,
) -> Result<String, reinhardt_di::DiError> {
	Ok(format!(
		"db: {:?}, config key: {}, param: {}",
		db, config.api_key, regular_param
	))
}

#[endpoint]
async fn handler_with_cache_control(
	#[inject] db: Database,
	#[inject(cache = false)] service: CustomService,
) -> Result<i32, reinhardt_di::DiError> {
	Ok(db.connection_string.len() as i32 + service.value)
}

#[endpoint]
async fn handler_only_inject(#[inject] db: Database) -> Result<Database, reinhardt_di::DiError> {
	// db is a Depends<Database>, deref to get &Database then clone
	Ok((*db).clone())
}

#[endpoint]
async fn handler_no_inject(
	regular_param1: String,
	regular_param2: i32,
) -> Result<String, reinhardt_di::DiError> {
	Ok(format!("{}-{}", regular_param1, regular_param2))
}

#[tokio::test]
async fn test_endpoint_with_single_inject() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// #[inject] parameters are removed by the macro, only pass regular params and &ctx at the end
	let result = handler_with_inject("test_value".to_string(), &ctx)
		.await
		.unwrap();

	// Check that it includes db info and regular param
	assert!(result.contains("db:"));
	assert!(result.contains("param: test_value"));
}

#[tokio::test]
async fn test_endpoint_with_multiple_inject() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Both #[inject] params (db, config) are removed, only pass regular_param and &ctx
	let result = handler_multiple_inject(100, &ctx).await.unwrap();

	// Check that it includes db, config, and regular param info
	assert!(result.contains("db:"));
	assert!(result.contains("config key:"));
	assert!(result.contains("param: 100"));
}

#[tokio::test]
async fn test_endpoint_with_cache_control() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Both #[inject] params (db, service) are removed, only pass &ctx
	let result = handler_with_cache_control(&ctx).await.unwrap();

	assert_eq!(result, 42); // service.value
}

#[tokio::test]
async fn test_endpoint_only_inject_params() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Only #[inject] param (db) is removed, only pass &ctx
	let result = handler_only_inject(&ctx).await.unwrap();

	assert_eq!(result.connection_string, "");
}

#[tokio::test]
async fn test_endpoint_no_inject_params() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// No #[inject] params, so pass all regular params and &ctx at the end
	let result = handler_no_inject("hello".to_string(), 42, &ctx)
		.await
		.unwrap();

	assert_eq!(result, "hello-42");
}

// ========== use_injection Macro Tests ==========

#[use_injection]
async fn process_data(#[inject] db: Database, data: String) -> DiResult<String> {
	Ok(format!(
		"Processing {} with db: {:?}",
		data, db.connection_string
	))
}

#[use_injection]
async fn complex_handler(
	#[inject] db: Database,
	#[inject] config: Config,
	#[inject] logger: Logger,
	input: i32,
) -> DiResult<String> {
	Ok(format!(
		"Input: {}, DB: {:?}, Config: {}, Logger: {:?}",
		input, db.connection_string, config.max_connections, logger.level
	))
}

#[use_injection]
async fn cache_control(
	#[inject] cached_db: Database,
	#[inject(cache = false)] fresh_logger: Logger,
) -> DiResult<String> {
	Ok(format!(
		"DB: {:?}, Logger: {:?}",
		cached_db.connection_string, fresh_logger.level
	))
}

#[use_injection]
async fn validate_user(#[inject] _db: Database, user_id: u64) -> DiResult<bool> {
	// For testing purposes, just check user_id validity
	// (db is injected to test DI functionality, not used in validation logic)
	Ok(user_id > 0)
}

#[use_injection]
async fn calculate_report(
	#[inject] db: Database,
	#[inject] config: Config,
	year: i32,
	month: i32,
) -> DiResult<String> {
	Ok(format!(
		"Report for {}/{} using {} connections from {:?}",
		year, month, config.max_connections, db.connection_string
	))
}

#[tokio::test]
async fn test_simple_injection() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// #[inject] db param is removed, only pass data and &ctx
	let result = process_data("test_data".to_string(), &ctx).await.unwrap();
	assert!(result.contains("Processing test_data"));
}

#[tokio::test]
async fn test_multiple_injections() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// All #[inject] params (db, config, logger) are removed, only pass input and &ctx
	let result = complex_handler(42, &ctx).await.unwrap();
	assert!(result.contains("Input: 42"));
	assert!(result.contains("DB:"));
	assert!(result.contains("Config:"));
	assert!(result.contains("Logger:"));
}

#[tokio::test]
async fn test_use_injection_cache_control() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Both #[inject] params (cached_db, fresh_logger) are removed, only pass &ctx
	let result = cache_control(&ctx).await.unwrap();
	assert!(result.contains("DB:"));
	assert!(result.contains("Logger:"));
}

#[tokio::test]
async fn test_helper_function() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// #[inject] db param is removed, only pass user_id and &ctx
	let valid = validate_user(12345, &ctx).await.unwrap();
	assert!(valid);

	let invalid = validate_user(0, &ctx).await.unwrap();
	assert!(!invalid);
}

#[tokio::test]
async fn test_business_logic() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// #[inject] params (db, config) are removed, only pass year, month, and &ctx
	let result = calculate_report(2025, 10, &ctx).await.unwrap();
	assert!(result.contains("Report for 2025/10"));
	assert!(result.contains("connections"));
}

#[tokio::test]
async fn test_injection_caching() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// #[inject] db param is removed, only pass data and &ctx
	// First call
	let result1 = process_data("data1".to_string(), &ctx).await.unwrap();
	// Second call - should use cached context
	let result2 = process_data("data2".to_string(), &ctx).await.unwrap();

	assert!(result1.contains("data1"));
	assert!(result2.contains("data2"));
}
