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
pub use mock::{
	CallRecord, DummyCache, MockFunction, MockRedisClusterCache, MockSchemaEditor, SimpleHandler,
	Spy,
};
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
	MySqlContainer, PostgresContainer, RedisContainer, TestDatabase, with_mysql, with_postgres,
	with_redis,
};

#[cfg(feature = "static")]
pub use static_files::*;

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
	pub use super::mock::{MockFunction, MockRedisClusterCache, SimpleHandler, Spy};
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
