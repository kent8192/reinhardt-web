//! Issuer-side OpenID Connect Authorization Code provider.
//!
//! This feature builds on the OAuth authorization server while keeping
//! subject mappings, login continuations, and signing-key state separate.

mod http;
#[cfg(feature = "database")]
mod postgres;
mod protocol;
mod signer;
mod store;

pub use http::{OidcEndpoint, OidcHandler, OidcInteraction};
#[cfg(feature = "database")]
pub use postgres::PostgresOidcStore;
pub use protocol::{
	OidcAccountStatus, OidcAuthorizationDecision, OidcAuthorizationRequest, OidcConfig, OidcError,
	OidcPendingHandle, OidcProvider, OidcTokenResponse,
};
pub use signer::{OidcSigner, RsaPemKeyRing};
pub use store::{
	MemoryOidcStore, OidcCodeContext, OidcKey, OidcPending, OidcStateStore, PublicRsaJwk,
};

#[cfg(test)]
mod tests;
