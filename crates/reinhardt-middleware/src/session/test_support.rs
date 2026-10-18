//! Test-only fixtures shared between the in-crate unit tests and external
//! integration tests for the typed session-value extractors.
//!
//! Available when the `test-support` cargo feature is enabled, or when the
//! crate is compiled under `#[cfg(test)]` from within itself. The module is
//! `#[doc(hidden)]` to keep it out of the public API surface — it is *not*
//! a stability guarantee for downstream crates.
//!
//! See Issue #4462 for the deduplication rationale: previously this
//! `TenantIdKey` fixture was declared verbatim in both
//! `src/session/value.rs` (`#[cfg(test)] mod tests`) and
//! `tests/session_value_from_request.rs`, which invited silent drift
//! whenever the key string changed.

#![allow(missing_docs)] // fixtures are internal test utilities

use async_trait::async_trait;
use reinhardt_di::{DiRegistrationList, Injectable, InjectionContext, SingletonScope};
use reinhardt_http::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

use super::data::SessionData;
use super::middleware::SessionMiddleware;
use super::store::SessionStore;
use super::value::SessionKey;

/// Tenant-id session key fixture used by the typed-extractor test suites.
///
/// Mirrors the shape every external `SessionKey` implementor would write
/// — a zero-sized marker type whose `KEY` constant points at the session
/// store entry. Keeping a single definition keeps the unit and integration
/// suites in lockstep.
pub struct TenantIdKey;

impl SessionKey for TenantIdKey {
	const KEY: &'static str = "tenant_id";
}

/// Singleton scope holding the DI registrations of `middleware`, as the
/// router applies them for handlers behind it.
pub fn session_di_scope(middleware: &SessionMiddleware) -> Arc<SingletonScope> {
	let scope = SingletonScope::new();
	let mut registrations = DiRegistrationList::new();
	for (type_id, value) in middleware.di_registrations() {
		registrations.register_arc_any(type_id, value);
	}
	registrations.apply_to(&scope);
	Arc::new(scope)
}

/// Handler that resolves the request's `SessionData` through its
/// `Injectable` implementation and saves it to the store, as a handler with
/// `#[inject] session: SessionData` that writes to its session does.
///
/// `SessionMiddleware` stores a new session only once a handler saves it and
/// sends `Set-Cookie` only for stored sessions, so tests that need a session
/// cookie for a request without one use this handler.
pub struct SessionSavingHandler {
	scope: Arc<SingletonScope>,
}

impl SessionSavingHandler {
	/// Wire the handler to the store `middleware` registers for DI.
	pub fn new(middleware: &SessionMiddleware) -> Self {
		Self {
			scope: session_di_scope(middleware),
		}
	}
}

#[async_trait]
impl Handler for SessionSavingHandler {
	async fn handle(&self, request: Request) -> Result<Response> {
		let ctx = InjectionContext::builder(Arc::clone(&self.scope)).build();
		ctx.set_request(request);
		let session = SessionData::inject(&ctx)
			.await
			.expect("SessionMiddleware must provide a session for every request");
		let store = ctx
			.get_singleton::<SessionStore>()
			.expect("SessionMiddleware must register its store for DI");
		store.save(session);
		Ok(Response::ok())
	}
}
