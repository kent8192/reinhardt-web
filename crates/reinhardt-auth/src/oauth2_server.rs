//! OAuth 2.0 authorization server primitives.
//!
//! The host owns login and consent. This module validates protocol requests,
//! persists the pending decision and issues audience-bound opaque tokens.
//!
//! ```
//! use reinhardt_auth::oauth2_server::OAuthServerConfig;
//!
//! let config = OAuthServerConfig::new(
//!     "https://auth.example.com",
//!     "https://auth.example.com/oauth/authorize",
//!     "https://auth.example.com/oauth/token",
//!     "https://auth.example.com/oauth/revoke",
//!     "https://auth.example.com/oauth/introspect",
//! ).unwrap();
//! assert_eq!(
//!     config.metadata_url().unwrap(),
//!     "https://auth.example.com/.well-known/oauth-authorization-server",
//! );
//! ```

mod http;
#[cfg(feature = "database")]
mod postgres;
mod protocol;
mod store;

pub(crate) use protocol::CodeExchangeRequest;
#[cfg(feature = "oidc-op")]
pub(crate) use protocol::PreparedAuthorization;

pub use http::{OAuthBrowserSession, OAuthConsentPresenter, OAuthEndpoint, OAuthHandler};
#[cfg(feature = "database")]
pub use postgres::PostgresOAuthStore;
pub use protocol::{
	AuthorizationDecision, AuthorizationRequest, IssuedToken, OAuthError, OAuthRateLimiter,
	OAuthServer, OAuthServerConfig, PendingAuthorization, SharedOAuthRateLimiter, TokenInfo,
	TokenPrincipal,
};
pub use store::{
	AuthorizationCommit, ClientKind, ClientRegistration, CodeInspection, CodeRedemption,
	CodeRedemptionRequest, MemoryOAuthStore, OAuthServerStore, PendingRecord, ResourceRegistration,
	StoredCode, StoredToken,
};

#[cfg(test)]
mod tests;
