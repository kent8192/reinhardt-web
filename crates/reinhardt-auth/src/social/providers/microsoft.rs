//! Microsoft OIDC provider

use crate::social::core::{
	OAuthProvider, ProviderConfig, SocialAuthError, StandardClaims, TokenResponse,
};
use async_trait::async_trait;

/// Microsoft OIDC provider
pub struct MicrosoftProvider {
	// Implementation pending
}

impl MicrosoftProvider {
	/// Create a new Microsoft provider
	pub async fn new(_config: ProviderConfig) -> Result<Self, SocialAuthError> {
		todo!("TASK-018: Implement MicrosoftProvider")
	}
}

#[async_trait]
impl OAuthProvider for MicrosoftProvider {
	fn name(&self) -> &str {
		"microsoft"
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
