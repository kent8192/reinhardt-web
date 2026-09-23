//! OAuth 2.0 authorization server primitives.
//!
//! The host owns login and consent. This module validates protocol requests,
//! persists the pending decision and issues audience-bound opaque tokens.

mod protocol;
mod store;

pub use protocol::{
	AuthorizationDecision, AuthorizationRequest, IssuedToken, OAuthError, OAuthRateLimiter,
	OAuthServer, OAuthServerConfig, PendingAuthorization, SharedOAuthRateLimiter, TokenInfo,
	TokenPrincipal,
};
pub use store::{
	ClientKind, ClientRegistration, CodeRedemption, MemoryOAuthStore, OAuthServerStore,
	PendingRecord, ResourceRegistration, StoredCode, StoredToken,
};
