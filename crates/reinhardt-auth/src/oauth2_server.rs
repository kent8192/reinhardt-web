//! OAuth 2.0 authorization server primitives.
//!
//! The host owns login and consent. This module validates protocol requests,
//! persists the pending decision and issues audience-bound opaque tokens.

#[cfg(feature = "database")]
mod postgres;
mod protocol;
mod store;

#[cfg(feature = "database")]
pub use postgres::PostgresOAuthStore;
pub use protocol::{
	AuthorizationDecision, AuthorizationRequest, IssuedToken, OAuthError, OAuthRateLimiter,
	OAuthServer, OAuthServerConfig, PendingAuthorization, SharedOAuthRateLimiter, TokenInfo,
	TokenPrincipal,
};
pub use store::{
	ClientKind, ClientRegistration, CodeRedemption, MemoryOAuthStore, OAuthServerStore,
	PendingRecord, ResourceRegistration, StoredCode, StoredToken,
};
