//! Testing utilities for Reinhardt framework
//!
//! This crate provides reusable testing utilities and helpers for the Reinhardt framework.
//! Actual test cases for Reinhardt functionality should be placed in the `reinhardt-qa` crate.
//!
//! This crate provides testing tools similar to Django REST Framework's test utilities:
//! - APIClient: Test client for making API requests
//! - APIRequestFactory: Factory for creating test requests
//! - APITestCase: Base test case with common assertions
//! - Response assertions and helpers
//! - TestContainers integration for database testing (optional, requires `testcontainers` feature)

pub mod assertions;
pub mod client;
pub mod debug;
pub mod factory;
pub mod fixtures;
pub mod http;
pub mod logging;
pub mod messages;
pub mod mock;
pub mod resource;
pub mod response;
pub mod server;
pub mod testcase;
pub mod views;
pub mod viewsets;

#[cfg(feature = "testcontainers")]
pub mod containers;

pub mod websocket;

// Re-export testcontainers crates for convenient access via reinhardt::test::testcontainers
#[cfg(feature = "testcontainers")]
pub use testcontainers;

#[cfg(feature = "testcontainers")]
pub use testcontainers_modules;

#[cfg(feature = "static")]
pub mod static_files;

pub use assertions::*;
pub use client::{APIClient, ClientError};
pub use debug::{DebugEntry, DebugPanel, DebugToolbar, SqlQuery, TimingInfo};
pub use factory::{APIRequestFactory, RequestBuilder};
pub use fixtures::{
	Factory, FactoryBuilder, FixtureError, FixtureLoader, FixtureResult, random_test_key,
	test_config_value,
};

#[cfg(feature = "testcontainers")]
pub use fixtures::{postgres_container, redis_container};
pub use http::{
	assert_has_header, assert_header_contains, assert_header_equals, assert_no_header,
	assert_status, create_insecure_request, create_request, create_response_with_headers,
	create_response_with_status, create_secure_request, create_test_request, create_test_response,
	extract_json, get_header, has_header, header_contains, header_equals,
};
pub use logging::init_test_logging;
pub use messages::{
	MessagesTestMixin, assert_message_count, assert_message_exists, assert_message_level,
	assert_message_tags, assert_messages,
};
pub use mock::{CallRecord, MockFunction, MockSchemaEditor, SimpleHandler, Spy};
pub use resource::{
	AsyncTeardownGuard, AsyncTestResource, SuiteGuard, SuiteResource, TeardownGuard, TestResource,
	acquire_suite,
};
pub use response::{ResponseExt, TestResponse};
pub use server::{
	BodyEchoHandler, DelayedHandler, EchoPathHandler, LargeResponseHandler, MethodEchoHandler,
	RouterHandler, StatusCodeHandler, shutdown_test_server, spawn_test_server,
};
pub use testcase::APITestCase;
pub use views::{
	ApiTestModel, ErrorKind, ErrorTestView, SimpleTestView, TestModel, create_api_test_objects,
	create_json_request, create_large_test_objects, create_request as create_view_request,
	create_request_with_headers, create_request_with_path_params, create_test_objects,
};
pub use viewsets::{SimpleViewSet, TestViewSet};

#[cfg(feature = "testcontainers")]
pub use containers::{
	MailHogContainer, MySqlContainer, PostgresContainer, RabbitMQContainer, RedisContainer,
	TestDatabase, with_mailhog, with_mysql, with_postgres, with_rabbitmq, with_redis,
};

#[cfg(feature = "static")]
pub use static_files::*;

pub use websocket::WebSocketTestClient;

/// Re-export commonly used testing types
pub mod prelude {
	pub use super::assertions::*;
	pub use super::client::APIClient;
	pub use super::debug::DebugToolbar;
	pub use super::factory::APIRequestFactory;
	pub use super::fixtures::{
		Factory, FactoryBuilder, FixtureLoader, random_test_key, test_config_value,
	};

	#[cfg(feature = "testcontainers")]
	pub use super::fixtures::{postgres_container, redis_container};
	pub use super::http::{
		assert_has_header, assert_header_contains, assert_header_equals, assert_no_header,
		assert_status, create_insecure_request, create_request, create_response_with_headers,
		create_response_with_status, create_secure_request, create_test_request,
		create_test_response, extract_json, get_header, has_header, header_contains, header_equals,
	};
	pub use super::logging::init_test_logging;
	pub use super::messages::{
		MessagesTestMixin, assert_message_count, assert_message_exists, assert_messages,
	};
	pub use super::mock::{MockFunction, SimpleHandler, Spy};
	pub use super::poll_until;
	pub use super::resource::{
		AsyncTeardownGuard, AsyncTestResource, SuiteGuard, SuiteResource, TeardownGuard,
		TestResource, acquire_suite,
	};
	pub use super::response::TestResponse;
	pub use super::server::{
		BodyEchoHandler, DelayedHandler, EchoPathHandler, LargeResponseHandler, MethodEchoHandler,
		RouterHandler, StatusCodeHandler, shutdown_test_server, spawn_test_server,
	};
	pub use super::testcase::APITestCase;
	pub use super::views::{
		ApiTestModel, ErrorTestView, SimpleTestView, TestModel, create_api_test_objects,
		create_test_objects,
	};
	pub use super::viewsets::{SimpleViewSet, TestViewSet};

	#[cfg(feature = "testcontainers")]
	pub use super::containers::{
		MySqlContainer, PostgresContainer, RedisContainer, TestDatabase, with_mysql, with_postgres,
		with_redis,
	};

	#[cfg(feature = "static")]
	pub use super::static_files::*;
}

/// Poll a condition until it becomes true or timeout is reached.
///
/// This is useful for testing asynchronous operations that may take some time to complete,
/// such as cache expiration, rate limit window resets, or background task completion.
///
/// # Arguments
///
/// * `timeout` - Maximum duration to wait for the condition to become true
/// * `interval` - Duration to wait between each poll attempt
/// * `condition` - Async closure that returns `true` when the desired state is reached
///
/// # Returns
///
/// * `Ok(())` if the condition becomes true within the timeout
/// * `Err(String)` if the timeout is reached before the condition becomes true
///
/// # Examples
///
/// ```no_run
/// use reinhardt_test::poll_until;
/// use std::time::Duration;
///
/// # async fn example() {
/// // Poll until a cache entry expires
/// poll_until(
///     Duration::from_millis(200),
///     Duration::from_millis(10),
///     || async {
///         // Check if cache entry has expired
///         // cache.get("key").await.is_none()
///         true
///     }
/// ).await.expect("Condition should be met");
/// # }
/// ```
pub async fn poll_until<F, Fut>(
	timeout: std::time::Duration,
	interval: std::time::Duration,
	mut condition: F,
) -> Result<(), String>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = bool>,
{
	let start = std::time::Instant::now();
	while start.elapsed() < timeout {
		if condition().await {
			return Ok(());
		}
		tokio::time::sleep(interval).await;
	}
	Err(format!("Timeout after {:?} waiting for condition", timeout))
}
