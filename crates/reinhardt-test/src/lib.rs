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
pub mod messages;
pub mod mock;
pub mod response;
pub mod testcase;

#[cfg(feature = "testcontainers")]
pub mod containers;

pub use assertions::*;
pub use client::{APIClient, ClientError};
pub use debug::{DebugEntry, DebugPanel, DebugToolbar, SqlQuery, TimingInfo};
pub use factory::{APIRequestFactory, RequestBuilder};
pub use fixtures::{Factory, FactoryBuilder, FixtureError, FixtureLoader, FixtureResult};
pub use messages::{
    MessagesTestMixin, assert_message_count, assert_message_exists, assert_message_level,
    assert_message_tags, assert_messages,
};
pub use mock::{CallRecord, MockFunction, Spy};
pub use response::{ResponseExt, TestResponse};
pub use testcase::APITestCase;

#[cfg(feature = "testcontainers")]
pub use containers::{
    MySqlContainer, PostgresContainer, RedisContainer, TestDatabase, with_mysql, with_postgres,
    with_redis,
};

/// Re-export commonly used testing types
pub mod prelude {
    pub use super::assertions::*;
    pub use super::client::APIClient;
    pub use super::debug::DebugToolbar;
    pub use super::factory::APIRequestFactory;
    pub use super::fixtures::{Factory, FactoryBuilder, FixtureLoader};
    pub use super::messages::{
        MessagesTestMixin, assert_message_count, assert_message_exists, assert_messages,
    };
    pub use super::mock::{MockFunction, Spy};
    pub use super::response::TestResponse;
    pub use super::testcase::APITestCase;

    #[cfg(feature = "testcontainers")]
    pub use super::containers::{
        MySqlContainer, PostgresContainer, RedisContainer, TestDatabase, with_mysql, with_postgres,
        with_redis,
    };
}
