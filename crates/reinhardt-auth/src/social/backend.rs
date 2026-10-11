//! Social authentication backend
//!
//! Orchestrates OAuth2/OIDC flows and integrates with reinhardt-auth.

use std::collections::HashMap;
use std::sync::Arc;

use crate::social::core::{OAuthProvider, SocialAuthError, StandardClaims, TokenResponse};
use crate::social::flow::{ContextualStateData, InMemoryStateStore, StateData, StateStore};

/// Result of beginning an authorization flow
pub struct AuthorizationResult {
	/// The URL to redirect the user to
	pub authorization_url: String,
	/// The state parameter for CSRF verification
	pub state: String,
	/// The nonce parameter for replay attack prevention (OIDC only)
	pub nonce: Option<String>,
	/// The PKCE code verifier (if PKCE is used)
	pub code_verifier: Option<String>,
}

/// Result of handling an authorization callback
///
/// A callback only succeeds once the provider has identified the user, so
/// `claims` is always present. A failed ID token validation or UserInfo
/// request fails the whole callback instead of yielding a token response
/// without an identity.
pub struct CallbackResult {
	/// The token response from the provider
	pub token_response: TokenResponse,
	/// The user's claims, from the validated ID token or the UserInfo endpoint
	pub claims: StandardClaims,
}

/// Result of handling a contextual authorization callback.
pub struct ContextualCallbackResult {
	/// The callback result from the provider.
	pub callback: CallbackResult,
	/// Opaque application context stored when authorization began.
	pub context: Vec<u8>,
}

/// Social authentication backend
pub struct SocialAuthBackend {
	providers: HashMap<String, Arc<dyn OAuthProvider>>,
	state_store: Arc<dyn StateStore>,
}

impl SocialAuthBackend {
	/// Create a new social authentication backend with in-memory state store
	pub fn new() -> Self {
		Self {
			providers: HashMap::new(),
			state_store: Arc::new(InMemoryStateStore::new()),
		}
	}

	/// Create a new social authentication backend with custom state store
	pub fn with_state_store(state_store: Arc<dyn StateStore>) -> Self {
		Self {
			providers: HashMap::new(),
			state_store,
		}
	}

	/// Register a provider
	pub fn register_provider(&mut self, provider: Arc<dyn OAuthProvider>) {
		self.providers.insert(provider.name().to_string(), provider);
	}

	/// Get a registered provider by name
	pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn OAuthProvider>> {
		self.providers.get(name)
	}

	/// List registered provider names
	pub fn provider_names(&self) -> Vec<&str> {
		self.providers.keys().map(|s| s.as_str()).collect()
	}

	/// Begin an authorization flow for a provider
	pub async fn begin_auth(
		&self,
		provider_name: &str,
		code_challenge: Option<&str>,
		code_verifier: Option<String>,
	) -> Result<AuthorizationResult, SocialAuthError> {
		let (authorization, state_data) = self
			.prepare_authorization(provider_name, code_challenge, code_verifier)
			.await?;
		self.state_store.store(state_data).await?;
		Ok(authorization)
	}

	/// Begin an authorization flow with browser or session binding and opaque context.
	pub async fn begin_auth_with_context(
		&self,
		provider_name: &str,
		code_challenge: Option<&str>,
		code_verifier: Option<String>,
		binding: &[u8],
		context: Vec<u8>,
	) -> Result<AuthorizationResult, SocialAuthError> {
		if binding.is_empty() {
			return Err(SocialAuthError::StateValidation(
				"OAuth state binding must not be empty".to_string(),
			));
		}

		let (authorization, state_data) = self
			.prepare_authorization(provider_name, code_challenge, code_verifier)
			.await?;
		let contextual =
			ContextualStateData::new(state_data, provider_name.to_string(), binding, context)?;
		self.state_store.store_contextual(contextual).await?;
		Ok(authorization)
	}

	/// Prepare the provider authorization URL and state record shared by both flows.
	async fn prepare_authorization(
		&self,
		provider_name: &str,
		code_challenge: Option<&str>,
		code_verifier: Option<String>,
	) -> Result<(AuthorizationResult, StateData), SocialAuthError> {
		let provider = self.providers.get(provider_name).ok_or_else(|| {
			SocialAuthError::Provider(format!("Provider not registered: {}", provider_name))
		})?;

		// Generate state for CSRF protection
		let state = generate_random_string(32);

		// Generate nonce for OIDC providers
		let nonce = if provider.is_oidc() {
			Some(generate_random_string(32))
		} else {
			None
		};

		// Build authorization URL
		let authorization_url = provider
			.authorization_url(&state, nonce.as_deref(), code_challenge)
			.await?;

		// Store state data for callback verification
		let state_data = StateData::new(state.clone(), nonce.clone(), code_verifier.clone());
		Ok((
			AuthorizationResult {
				authorization_url,
				state,
				nonce,
				code_verifier,
			},
			state_data,
		))
	}

	/// Handle an authorization callback
	///
	/// # Errors
	///
	/// Returns an error when the state is invalid, the code exchange fails,
	/// ID token validation fails, or the user's claims cannot be retrieved.
	/// For OAuth2-only providers and OIDC providers that return no ID token,
	/// a failed UserInfo request is returned as the provider's error
	/// (typically [`SocialAuthError::UserInfoError`]).
	pub async fn handle_callback(
		&self,
		provider_name: &str,
		code: &str,
		state: &str,
	) -> Result<CallbackResult, SocialAuthError> {
		let provider = self.providers.get(provider_name).ok_or_else(|| {
			SocialAuthError::Provider(format!("Provider not registered: {}", provider_name))
		})?;

		let state_data = self.state_store.consume(state).await?;
		self.complete_callback(provider.as_ref(), provider_name, code, &state_data)
			.await
	}

	/// Handle a contextual authorization callback with binding verification.
	///
	/// # Errors
	///
	/// Returns the same errors as [`Self::handle_callback`], plus
	/// [`SocialAuthError::InvalidState`] when the state is expired, belongs to
	/// another provider, or does not match `binding`.
	pub async fn handle_callback_with_context(
		&self,
		provider_name: &str,
		code: &str,
		state: &str,
		binding: &[u8],
	) -> Result<ContextualCallbackResult, SocialAuthError> {
		if binding.is_empty() {
			return Err(SocialAuthError::StateValidation(
				"OAuth state binding must not be empty".to_string(),
			));
		}

		let provider = self.providers.get(provider_name).ok_or_else(|| {
			SocialAuthError::Provider(format!("Provider not registered: {}", provider_name))
		})?;
		let contextual = self.state_store.consume_contextual(state).await?;
		if contextual.state_data().is_expired()
			|| contextual.provider_name() != provider_name
			|| !contextual.binding_matches(binding)
		{
			return Err(SocialAuthError::InvalidState);
		}

		let (state_data, context) = contextual.into_parts();
		let callback = self
			.complete_callback(provider.as_ref(), provider_name, code, &state_data)
			.await?;
		Ok(ContextualCallbackResult { callback, context })
	}

	/// Complete provider token exchange and claims retrieval for a validated state.
	async fn complete_callback(
		&self,
		provider: &dyn OAuthProvider,
		provider_name: &str,
		code: &str,
		state_data: &StateData,
	) -> Result<CallbackResult, SocialAuthError> {
		// Exchange code for tokens
		let token_response = provider
			.exchange_code(code, state_data.code_verifier.as_deref())
			.await?;

		// Resolve the user's identity. A callback without claims is not a
		// completed sign-in, so every retrieval failure is propagated.
		let claims = match &token_response.id_token {
			Some(id_token_str) if provider.is_oidc() => {
				let id_token = provider
					.validate_id_token(id_token_str, state_data.nonce.as_deref())
					.await?;
				StandardClaims::from(id_token)
			}
			// OAuth2-only providers, and OIDC providers that omitted the ID
			// token, identify the user through the UserInfo endpoint.
			_ => provider
				.get_user_info(&token_response.access_token)
				.await
				.inspect_err(|e| {
					tracing::warn!(
						provider = %provider_name,
						error = %e,
						"Failed to fetch user info; rejecting social auth callback",
					)
				})?,
		};

		Ok(CallbackResult {
			token_response,
			claims,
		})
	}
}

impl Default for SocialAuthBackend {
	fn default() -> Self {
		Self::new()
	}
}

/// Generates a random alphanumeric string of the specified length
fn generate_random_string(length: usize) -> String {
	use rand::Rng;
	rand::rng()
		.sample_iter(&rand::distr::Alphanumeric)
		.take(length)
		.map(char::from)
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_trait::async_trait;
	use rstest::rstest;

	/// Provider whose token exchange succeeds without an ID token and whose
	/// UserInfo request returns `userinfo`.
	struct StubProvider {
		oidc: bool,
		userinfo: Result<StandardClaims, SocialAuthError>,
	}

	#[async_trait]
	impl OAuthProvider for StubProvider {
		fn name(&self) -> &str {
			"stub"
		}

		fn is_oidc(&self) -> bool {
			self.oidc
		}

		async fn authorization_url(
			&self,
			state: &str,
			_nonce: Option<&str>,
			_code_challenge: Option<&str>,
		) -> Result<String, SocialAuthError> {
			Ok(format!("https://provider.example/authorize?state={state}"))
		}

		async fn exchange_code(
			&self,
			_code: &str,
			_code_verifier: Option<&str>,
		) -> Result<TokenResponse, SocialAuthError> {
			Ok(TokenResponse {
				access_token: "access-token".to_string(),
				token_type: "Bearer".to_string(),
				expires_in: Some(3600),
				refresh_token: None,
				scope: None,
				id_token: None,
			})
		}

		async fn refresh_token(
			&self,
			_refresh_token: &str,
		) -> Result<TokenResponse, SocialAuthError> {
			Err(SocialAuthError::NotSupported("refresh".to_string()))
		}

		async fn get_user_info(
			&self,
			_access_token: &str,
		) -> Result<StandardClaims, SocialAuthError> {
			self.userinfo.clone()
		}
	}

	fn claims_for(sub: &str) -> StandardClaims {
		StandardClaims {
			sub: sub.to_string(),
			email: None,
			email_verified: None,
			name: None,
			given_name: None,
			family_name: None,
			picture: None,
			locale: None,
			additional_claims: HashMap::new(),
		}
	}

	async fn backend_with(provider: StubProvider) -> (SocialAuthBackend, String) {
		let mut backend = SocialAuthBackend::new();
		backend.register_provider(Arc::new(provider));
		let authorization = backend.begin_auth("stub", None, None).await.unwrap();
		(backend, authorization.state)
	}

	#[rstest]
	#[case::oauth2_provider(false)]
	#[case::oidc_provider_without_id_token(true)]
	#[tokio::test]
	async fn callback_returns_userinfo_claims(#[case] oidc: bool) {
		// Arrange
		let (backend, state) = backend_with(StubProvider {
			oidc,
			userinfo: Ok(claims_for("user-42")),
		})
		.await;

		// Act
		let callback = backend
			.handle_callback("stub", "code", &state)
			.await
			.unwrap();

		// Assert
		assert_eq!(callback.claims.sub, "user-42");
		assert_eq!(callback.token_response.access_token, "access-token");
	}

	#[rstest]
	#[case::oauth2_provider(false)]
	#[case::oidc_provider_without_id_token(true)]
	#[tokio::test]
	async fn callback_fails_when_userinfo_fails(#[case] oidc: bool) {
		// Arrange
		let userinfo_error =
			SocialAuthError::UserInfoError("UserInfo request failed (401 Unauthorized)".into());
		let (backend, state) = backend_with(StubProvider {
			oidc,
			userinfo: Err(userinfo_error.clone()),
		})
		.await;

		// Act
		let result = backend.handle_callback("stub", "code", &state).await;

		// Assert
		assert_eq!(result.err(), Some(userinfo_error));
	}

	#[test]
	fn test_backend_creation() {
		// Arrange & Act
		let backend = SocialAuthBackend::new();

		// Assert
		assert!(backend.provider_names().is_empty());
	}

	#[test]
	fn test_backend_default() {
		// Arrange & Act
		let backend = SocialAuthBackend::default();

		// Assert
		assert!(backend.provider_names().is_empty());
	}

	#[test]
	fn test_get_nonexistent_provider() {
		// Arrange
		let backend = SocialAuthBackend::new();

		// Act
		let provider = backend.get_provider("nonexistent");

		// Assert
		assert!(provider.is_none());
	}
}
