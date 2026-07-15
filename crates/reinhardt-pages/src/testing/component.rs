//! Native component testing harness.
//!
//! The in-memory renderer resolves reactive views, active suspense branches,
//! and deferred content branches before exposing queryable text and roles. Each
//! screen establishes an isolated keyed-query cache scope for its test work.

mod error;
mod events;
mod fixture;
mod pretty;
mod query;
mod role;
mod scheduler;
mod screen;
#[cfg(feature = "msw")]
pub(crate) mod server_fn_mock;
mod text_match;
mod tree;

#[cfg(test)]
mod tests;

pub use error::{EventError, QueryError};
pub use events::ElementHandle;
pub use fixture::{EventFixture, EventFixtureError};
pub use role::Role;
pub use scheduler::SettleError;
pub use screen::{Screen, TestRender, render};
#[cfg(feature = "msw")]
pub use server_fn_mock::{RecordedServerFnCall, ServerFnCallQuery};
pub use text_match::TextMatch;

#[cfg(feature = "msw")]
pub(crate) fn active_query_scope_id() -> Option<u64> {
	server_fn_mock::active_scope_id()
}

#[cfg(not(feature = "msw"))]
pub(crate) fn active_query_scope_id() -> Option<u64> {
	scheduler::active_scope_id()
}
