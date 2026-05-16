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
