//! Core abstractions for social authentication

pub mod error;
pub mod provider;
pub mod client;
pub mod token;
pub mod claims;
pub mod config;

pub use error::SocialAuthError;
pub use provider::OAuthProvider;
pub use client::OAuth2Client;
pub use token::{OAuthToken, TokenResponse};
pub use claims::{IdToken, StandardClaims};
pub use config::{ProviderConfig, OIDCConfig, OAuth2Config};
