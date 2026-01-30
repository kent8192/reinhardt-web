//! GitHub OAuth2 provider

use crate::social::core::{
	OAuthProvider, ProviderConfig, SocialAuthError, StandardClaims, TokenResponse,
};
use async_trait::async_trait;

/// GitHub OAuth2 provider
pub struct GitHubProvider {
	// Implementation pending
}

impl GitHubProvider {
	/// Create a new GitHub provider
	pub async fn new(_config: ProviderConfig) -> Result<Self, SocialAuthError> {
		todo!("TASK-016: Implement GitHubProvider")
	}
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
	fn name(&self) -> &str {
		"github"
	}

	fn is_oidc(&self) -> bool {
		false
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
