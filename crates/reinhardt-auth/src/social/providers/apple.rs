//! Apple OIDC provider

use crate::social::core::{
	OAuthProvider, ProviderConfig, SocialAuthError, StandardClaims, TokenResponse,
};
use async_trait::async_trait;

/// Apple OIDC provider
pub struct AppleProvider {
	// Implementation pending
}

impl AppleProvider {
	/// Create a new Apple provider
	pub async fn new(_config: ProviderConfig) -> Result<Self, SocialAuthError> {
		todo!("TASK-017: Implement AppleProvider")
	}
}

#[async_trait]
impl OAuthProvider for AppleProvider {
	fn name(&self) -> &str {
		"apple"
	}

	fn is_oidc(&self) -> bool {
		true
	}

	async fn authorization_url(
		&self,
		_state: &str,
		_nonce: Option<&str>,
		_code_challenge: Option<&str>,
	) -> Result<String, SocialAuthError> {
		todo!()
	}

	async fn exchange_code(
		&self,
		_code: &str,
		_code_verifier: Option<&str>,
	) -> Result<TokenResponse, SocialAuthError> {
		todo!()
	}

	async fn refresh_token(&self, _refresh_token: &str) -> Result<TokenResponse, SocialAuthError> {
		todo!()
	}

	async fn get_user_info(&self, _access_token: &str) -> Result<StandardClaims, SocialAuthError> {
		todo!()
	}
}
