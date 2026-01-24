//! Social Authentication Module
//!
//! Provides OAuth2/OIDC-based social login support for third-party identity providers.
//!
//! # Supported Providers
//!
//! - **Google OIDC**: OpenID Connect authentication with Google
//! - **GitHub OAuth2**: OAuth 2.0 authentication with GitHub
//! - **Apple OIDC**: OpenID Connect authentication with Apple (with JWT-based client_secret)
//! - **Microsoft OIDC**: OpenID Connect authentication with Microsoft/Azure AD
//!
//! # Security Features
//!
//! - **PKCE**: Proof Key for Code Exchange (RFC 7636) for all flows
//! - **CSRF Protection**: State parameter validation
//! - **ID Token Validation**: Signature verification with JWKS
//! - **Nonce Validation**: Replay attack prevention for OIDC flows
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_auth::social::{SocialAuthBackend, providers::GoogleProvider};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut backend = SocialAuthBackend::new();
//!
//!     // Register Google provider
//!     let google = GoogleProvider::new(config).await.unwrap();
//!     backend.register_provider(Arc::new(google));
//!
//!     // Start authorization flow
//!     let auth_url = backend.start_authorization("google").await.unwrap();
//!
//!     // Handle callback
//!     let user = backend.handle_callback("google", &code, &state).await.unwrap();
//! }
//! ```

pub mod core;
pub mod flow;
pub mod oidc;
pub mod providers;
pub mod backend;
pub mod user_mapping;
pub mod storage;

// Re-export core types
pub use core::{
	OAuthProvider, OAuth2Client, SocialAuthError,
	OAuthToken, TokenResponse, IdToken, StandardClaims,
	ProviderConfig, OIDCConfig, OAuth2Config,
};

// Re-export flow types
pub use flow::{
	AuthorizationFlow, TokenExchangeFlow, RefreshFlow,
	PkceFlow, StateStore, StateData,
};

// Re-export OIDC types
pub use oidc::{
	DiscoveryClient, OIDCDiscovery,
	IdTokenValidator,
	JwksCache, Jwk, JwkSet,
	UserInfoClient,
};

// Re-export providers
pub use providers::{
	GoogleProvider,
	GitHubProvider,
	AppleProvider,
	MicrosoftProvider,
};

// Re-export backend
pub use backend::SocialAuthBackend;

// Re-export user mapping
pub use user_mapping::{UserMapper, DefaultUserMapper};

// Re-export storage
pub use storage::{SocialAccount, SocialAccountStorage};
