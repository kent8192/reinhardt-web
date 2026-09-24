//! OpenID Connect request validation, token issuance, and host integration.

#[cfg(feature = "database")]
use super::PostgresOidcStore;
use super::signer::OidcSigner;
use super::store::{OidcCodeContext, OidcPending, OidcStateStore};
use crate::oauth2_server::{
	AuthorizationDecision, AuthorizationRequest, ClientKind, CodeExchangeRequest, OAuthError,
	OAuthServer,
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

/// OIDC protocol or host-integration error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OidcError {
	/// A request parameter is malformed or unsupported.
	InvalidRequest,
	/// Client authentication failed.
	InvalidClient,
	/// The client is not registered for this flow.
	UnauthorizedClient,
	/// The code is expired, reused, or bound to other parameters.
	InvalidGrant,
	/// Only the registered openid scope is supported.
	InvalidScope,
	/// The response type is not the supported code flow.
	UnsupportedResponseType,
	/// The user or host denied authorization.
	AccessDenied,
	/// Silent authentication needs an interactive login.
	LoginRequired,
	/// The host cannot obtain requested consent silently.
	ConsentRequired,
	/// The host cannot perform requested account selection.
	AccountSelectionRequired,
	/// Another interaction is required to satisfy the request.
	InteractionRequired,
	/// A dependency failed; no token should be issued.
	ServerError,
}

impl OidcError {
	/// Standard wire error code.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::InvalidRequest => "invalid_request",
			Self::InvalidClient => "invalid_client",
			Self::UnauthorizedClient => "unauthorized_client",
			Self::InvalidGrant => "invalid_grant",
			Self::InvalidScope => "invalid_scope",
			Self::UnsupportedResponseType => "unsupported_response_type",
			Self::AccessDenied => "access_denied",
			Self::LoginRequired => "login_required",
			Self::ConsentRequired => "consent_required",
			Self::AccountSelectionRequired => "account_selection_required",
			Self::InteractionRequired => "interaction_required",
			Self::ServerError => "server_error",
		}
	}
}

impl From<OAuthError> for OidcError {
	fn from(error: OAuthError) -> Self {
		match error {
			OAuthError::InvalidRequest | OAuthError::InvalidTarget => Self::InvalidRequest,
			OAuthError::InvalidClient => Self::InvalidClient,
			OAuthError::UnauthorizedClient => Self::UnauthorizedClient,
			OAuthError::InvalidGrant => Self::InvalidGrant,
			OAuthError::InvalidScope => Self::InvalidScope,
			OAuthError::AccessDenied => Self::AccessDenied,
			OAuthError::UnsupportedGrantType => Self::InvalidRequest,
			OAuthError::UnsupportedResponseType => Self::UnsupportedResponseType,
			OAuthError::ServerError => Self::ServerError,
		}
	}
}

/// URLs and bounded lifetimes for a single issuer origin.
#[derive(Clone, Debug)]
pub struct OidcConfig {
	/// HTTPS issuer origin without a path or trailing slash.
	pub issuer: String,
	/// Mounted authorization endpoint URL.
	pub authorization_endpoint: String,
	/// Mounted token endpoint URL.
	pub token_endpoint: String,
	/// Mounted UserInfo endpoint URL and sole access-token audience.
	pub userinfo_endpoint: String,
	/// Mounted JWKS endpoint URL.
	pub jwks_uri: String,
	/// ID Token lifetime; default five minutes, maximum fifteen.
	pub id_token_ttl: Duration,
	/// UserInfo access-token lifetime; default ten minutes, maximum one hour.
	pub access_token_ttl: Duration,
	/// Scheduled key-rotation interval; default thirty days.
	pub key_rotation_interval: Duration,
	/// Maximum clock skew used to retain retired public keys.
	pub clock_skew: Duration,
	allow_loopback_http: bool,
}

impl OidcConfig {
	/// Construct an HTTPS issuer profile.
	pub fn new(
		issuer: &str,
		authorization_endpoint: &str,
		token_endpoint: &str,
		userinfo_endpoint: &str,
		jwks_uri: &str,
	) -> Result<Self, OidcError> {
		let config = Self {
			issuer: issuer.trim_end_matches('/').to_owned(),
			authorization_endpoint: authorization_endpoint.to_owned(),
			token_endpoint: token_endpoint.to_owned(),
			userinfo_endpoint: userinfo_endpoint.to_owned(),
			jwks_uri: jwks_uri.to_owned(),
			id_token_ttl: Duration::from_secs(300),
			access_token_ttl: Duration::from_secs(600),
			key_rotation_interval: Duration::from_secs(30 * 24 * 3600),
			clock_skew: Duration::from_secs(60),
			allow_loopback_http: false,
		};
		config.validate()?;
		Ok(config)
	}

	/// Construct an explicit HTTP loopback profile for development and tests.
	pub fn for_loopback_development(
		issuer: &str,
		authorization_endpoint: &str,
		token_endpoint: &str,
		userinfo_endpoint: &str,
		jwks_uri: &str,
	) -> Result<Self, OidcError> {
		let mut config = Self::new_unvalidated(
			issuer,
			authorization_endpoint,
			token_endpoint,
			userinfo_endpoint,
			jwks_uri,
		);
		config.allow_loopback_http = true;
		if !config.issuer.starts_with("http://") {
			return Err(OidcError::InvalidRequest);
		}
		config.validate()?;
		Ok(config)
	}

	fn new_unvalidated(
		issuer: &str,
		authorization_endpoint: &str,
		token_endpoint: &str,
		userinfo_endpoint: &str,
		jwks_uri: &str,
	) -> Self {
		Self {
			issuer: issuer.trim_end_matches('/').to_owned(),
			authorization_endpoint: authorization_endpoint.to_owned(),
			token_endpoint: token_endpoint.to_owned(),
			userinfo_endpoint: userinfo_endpoint.to_owned(),
			jwks_uri: jwks_uri.to_owned(),
			id_token_ttl: Duration::from_secs(300),
			access_token_ttl: Duration::from_secs(600),
			key_rotation_interval: Duration::from_secs(30 * 24 * 3600),
			clock_skew: Duration::from_secs(60),
			allow_loopback_http: false,
		}
	}

	/// Discovery URL for the root-path issuer.
	pub fn discovery_url(&self) -> String {
		format!("{}/.well-known/openid-configuration", self.issuer)
	}

	fn validate(&self) -> Result<(), OidcError> {
		let issuer = Url::parse(&self.issuer).map_err(|_| OidcError::InvalidRequest)?;
		let secure = issuer.scheme() == "https";
		let loopback = issuer.scheme() == "http"
			&& self.allow_loopback_http
			&& matches!(
				issuer.host_str(),
				Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
			);
		if (!secure && !loopback)
			|| issuer.path() != "/"
			|| issuer.query().is_some()
			|| issuer.fragment().is_some()
			|| !issuer.username().is_empty()
			|| issuer.password().is_some()
			|| issuer.host_str().is_none()
		{
			return Err(OidcError::InvalidRequest);
		}
		for endpoint in [
			&self.authorization_endpoint,
			&self.token_endpoint,
			&self.userinfo_endpoint,
			&self.jwks_uri,
		] {
			let url = Url::parse(endpoint).map_err(|_| OidcError::InvalidRequest)?;
			if url.origin() != issuer.origin()
				|| url.scheme() != issuer.scheme()
				|| url.query().is_some()
				|| url.fragment().is_some()
				|| !url.username().is_empty()
				|| url.password().is_some()
			{
				return Err(OidcError::InvalidRequest);
			}
		}
		if self.id_token_ttl.is_zero()
			|| self.id_token_ttl > Duration::from_secs(900)
			|| self.access_token_ttl.is_zero()
			|| self.access_token_ttl > Duration::from_secs(3600)
			|| self.key_rotation_interval.is_zero()
			|| self.clock_skew > Duration::from_secs(300)
		{
			return Err(OidcError::InvalidRequest);
		}
		Ok(())
	}
}

/// Host status check that must explicitly consult the authoritative account.
#[async_trait]
pub trait OidcAccountStatus: Send + Sync {
	/// Return false for a disabled or deleted user and fail on lookup errors.
	async fn is_active(&self, user_id: &str) -> Result<bool, String>;
}

/// Browser Authorization Code request after HTTP parameter parsing.
#[derive(Clone, Debug)]
pub struct OidcAuthorizationRequest {
	/// Registered confidential client identifier.
	pub client_id: String,
	/// Exact registered callback URL.
	pub redirect_uri: String,
	/// Must be code.
	pub response_type: String,
	/// Must be openid for the initial profile.
	pub scope: String,
	/// S256 PKCE challenge.
	pub code_challenge: String,
	/// Must be S256.
	pub code_challenge_method: String,
	/// Opaque RP callback state.
	pub state: Option<String>,
	/// Optional RP nonce.
	pub nonce: Option<String>,
	/// Optional space-separated OIDC prompt values.
	pub prompt: Option<String>,
	/// Optional maximum age of active authentication.
	pub max_age: Option<u64>,
}

/// Host response after authenticating and authorizing a pending request.
#[derive(Clone, Debug)]
pub enum OidcAuthorizationDecision {
	/// Approve with explicitly observed authentication and interaction facts.
	Approve {
		/// Stable host local user identifier.
		user_id: String,
		/// UNIX time of the latest active authentication.
		auth_time: i64,
		/// Whether the host obtained or already holds valid consent.
		consented: bool,
		/// Whether an explicit consent prompt was completed in this flow.
		consent_prompted: bool,
		/// Whether requested account selection occurred.
		account_selected: bool,
		/// Whether requested reauthentication occurred in this flow.
		reauthenticated: bool,
	},
	/// Deny using an OIDC authorization error.
	Deny(OidcError),
}

/// Successful token response for Authorization Code Flow.
#[derive(Clone, Debug, Serialize)]
pub struct OidcTokenResponse {
	/// Opaque UserInfo-only access token.
	pub access_token: String,
	/// Always Bearer.
	pub token_type: &'static str,
	/// Access-token lifetime in seconds.
	pub expires_in: u64,
	/// Always openid.
	pub scope: String,
	/// Signed RS256 ID Token.
	pub id_token: String,
}

/// Issuer-side OIDC protocol assembled on a shared OAuth server.
pub struct OidcProvider {
	config: OidcConfig,
	oauth: Arc<OAuthServer>,
	state: Arc<dyn OidcStateStore>,
	accounts: Arc<dyn OidcAccountStatus>,
	signer: Arc<dyn OidcSigner>,
	production: bool,
}

impl OidcProvider {
	/// Build a development provider using explicit host adapters.
	pub fn for_development(
		config: OidcConfig,
		oauth: Arc<OAuthServer>,
		state: Arc<dyn OidcStateStore>,
		accounts: Arc<dyn OidcAccountStatus>,
		signer: Arc<dyn OidcSigner>,
	) -> Result<Self, OidcError> {
		config.validate()?;
		if config.issuer != oauth.config().issuer || oauth.is_production() {
			return Err(OidcError::InvalidRequest);
		}
		Ok(Self {
			config,
			oauth,
			state,
			accounts,
			signer,
			production: false,
		})
	}

	/// Build a production provider only when shared SQL state and signing are ready.
	#[cfg(feature = "database")]
	pub async fn for_production(
		config: OidcConfig,
		oauth: Arc<OAuthServer>,
		state: PostgresOidcStore,
		accounts: Arc<dyn OidcAccountStatus>,
		signer: Arc<dyn OidcSigner>,
	) -> Result<Self, OidcError> {
		config.validate()?;
		if config.allow_loopback_http
			|| config.issuer != oauth.config().issuer
			|| !oauth.is_production()
		{
			return Err(OidcError::InvalidRequest);
		}
		let state: Arc<dyn OidcStateStore> = Arc::new(state);
		let key = state
			.active_key()
			.await
			.map_err(|_| OidcError::ServerError)?;
		let key = key.ok_or(OidcError::ServerError)?;
		if signer
			.public_key(&key.public.kid)
			.await
			.map_err(|_| OidcError::ServerError)?
			!= Some(key.public)
		{
			return Err(OidcError::ServerError);
		}
		Ok(Self {
			config,
			oauth,
			state,
			accounts,
			signer,
			production: true,
		})
	}

	/// Validated issuer configuration.
	pub fn config(&self) -> &OidcConfig {
		&self.config
	}

	/// Whether this provider requires HTTPS incoming requests.
	pub fn is_production(&self) -> bool {
		self.production
	}

	/// Shared rate limiter supplied by the OAuth server.
	pub async fn allow(&self, key: &str) -> bool {
		self.oauth.allow(key).await
	}

	/// Public Discovery metadata for the implemented Code Flow only.
	pub fn discovery(&self) -> Value {
		json!({
			"issuer": self.config.issuer,
			"authorization_endpoint": self.config.authorization_endpoint,
			"token_endpoint": self.config.token_endpoint,
			"userinfo_endpoint": self.config.userinfo_endpoint,
			"jwks_uri": self.config.jwks_uri,
			"response_types_supported": ["code"],
			"response_modes_supported": ["query"],
			"grant_types_supported": ["authorization_code"],
			"subject_types_supported": ["public"],
			"id_token_signing_alg_values_supported": ["RS256"],
			"scopes_supported": ["openid"],
			"claims_supported": ["sub"],
			"token_endpoint_auth_methods_supported": ["client_secret_basic"],
			"code_challenge_methods_supported": ["S256"],
			"request_parameter_supported": false,
			"request_uri_parameter_supported": false,
			"claims_parameter_supported": false
		})
	}

	/// Public JSON Web Key Set, excluding compromised and expired retired keys.
	pub async fn jwks(&self) -> Result<Value, OidcError> {
		let keys = self
			.state
			.public_keys(now())
			.await
			.map_err(|_| OidcError::ServerError)?;
		Ok(json!({"keys": keys.into_iter().map(|key| json!({
			"kty":"RSA", "use":"sig", "alg":"RS256", "kid":key.public.kid,
			"n":key.public.n, "e":key.public.e
		})).collect::<Vec<_>>()}))
	}

	/// Activate a provisioned signing key across all instances.
	pub async fn rotate_signing_key(&self, kid: &str) -> Result<(), OidcError> {
		let public = self
			.signer
			.public_key(kid)
			.await
			.map_err(|_| OidcError::ServerError)?
			.ok_or(OidcError::InvalidRequest)?;
		let retention = 900 + self.config.clock_skew.as_secs();
		self.state
			.rotate_key(public, now(), retention as i64)
			.await
			.map_err(|_| OidcError::ServerError)?;
		tracing::info!(event = "oidc_signing_key_rotated", kid);
		Ok(())
	}

	/// Report whether the active signing key has reached the configured rotation age.
	pub async fn signing_key_rotation_due(&self) -> Result<bool, OidcError> {
		let key = self
			.state
			.active_key()
			.await
			.map_err(|_| OidcError::ServerError)?;
		Ok(key.is_none_or(|key| {
			now() - key.activated_at >= self.config.key_rotation_interval.as_secs() as i64
		}))
	}

	/// Stop signing with a compromised key and remove it from JWKS.
	pub async fn compromise_signing_key(&self, kid: &str) -> Result<(), OidcError> {
		self.state
			.compromise_key(kid)
			.await
			.map_err(|_| OidcError::ServerError)?;
		tracing::warn!(event = "oidc_signing_key_compromised", kid);
		Ok(())
	}

	/// Validate and store a browser authorization request.
	pub async fn begin_authorization(
		&self,
		request: OidcAuthorizationRequest,
		browser_session: &str,
	) -> Result<OidcPendingHandle, OidcError> {
		if request.response_type != "code" {
			return Err(OidcError::UnsupportedResponseType);
		}
		if request.scope != "openid" {
			return Err(OidcError::InvalidScope);
		}
		if request.max_age.is_some_and(|value| value > i64::MAX as u64)
			|| request.state.as_ref().is_some_and(|v| v.len() > 512)
			|| request
				.nonce
				.as_ref()
				.is_some_and(|v| v.is_empty() || v.len() > 512)
		{
			return Err(OidcError::InvalidRequest);
		}
		let prompts = parse_prompts(request.prompt.as_deref())?;
		let client = self
			.oauth
			.client(&request.client_id)
			.await?
			.filter(|client| client.enabled)
			.ok_or(OidcError::InvalidClient)?;
		if client.kind != ClientKind::Confidential
			|| !client.authorization_code
			|| !client.oidc_enabled
			|| !client.scopes.iter().any(|scope| scope == "openid")
			|| !client
				.audiences
				.iter()
				.any(|aud| aud == &self.config.userinfo_endpoint)
		{
			return Err(OidcError::UnauthorizedClient);
		}
		let redirect = Url::parse(&request.redirect_uri).map_err(|_| OidcError::InvalidRequest)?;
		let secure_redirect = redirect.scheme() == "https";
		let loopback_redirect = self.config.allow_loopback_http
			&& redirect.scheme() == "http"
			&& matches!(
				redirect.host_str(),
				Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
			);
		if !client
			.redirect_uris
			.iter()
			.any(|uri| uri == &request.redirect_uri)
			|| (!secure_redirect && !loopback_redirect)
		{
			return Err(OidcError::InvalidRequest);
		}
		let pending = self
			.oauth
			.begin_oidc_authorization(
				AuthorizationRequest {
					client_id: request.client_id.clone(),
					redirect_uri: request.redirect_uri.clone(),
					response_type: request.response_type,
					code_challenge: request.code_challenge,
					code_challenge_method: request.code_challenge_method,
					scope: Some(request.scope),
					resource: Some(self.config.userinfo_endpoint.clone()),
					state: request.state.clone(),
				},
				browser_session,
			)
			.await?;
		let oidc = OidcPending {
			session_digest: digest(browser_session),
			requested_at: now(),
			client_id: request.client_id,
			redirect_uri: request.redirect_uri,
			state: request.state,
			nonce: request.nonce,
			prompts,
			max_age: request.max_age,
			expires_at: now() + self.oauth.config().pending_ttl.as_secs() as i64,
		};
		self.state
			.put_pending(&pending.id, oidc.clone())
			.await
			.map_err(|_| OidcError::ServerError)?;
		Ok(OidcPendingHandle {
			id: pending.id,
			request: oidc,
		})
	}

	/// Complete a one-time login continuation and return the RP redirect.
	pub async fn complete_authorization(
		&self,
		pending_id: &str,
		browser_session: &str,
		decision: OidcAuthorizationDecision,
	) -> Result<String, OidcError> {
		let pending = self
			.state
			.take_pending(pending_id, &digest(browser_session), now())
			.await
			.map_err(|_| OidcError::ServerError)?
			.ok_or(OidcError::InvalidGrant)?;
		let denial = match &decision {
			OidcAuthorizationDecision::Deny(error) => Some(*error),
			OidcAuthorizationDecision::Approve {
				user_id,
				auth_time,
				consented,
				consent_prompted,
				account_selected,
				reauthenticated,
			} => {
				let valid_identity = !user_id.is_empty()
					&& *auth_time > 0
					&& *auth_time <= now() + self.config.clock_skew.as_secs() as i64;
				let active = if valid_identity {
					self.accounts
						.is_active(user_id)
						.await
						.map_err(|_| OidcError::ServerError)?
				} else {
					false
				};
				let login_required = pending.prompts.iter().any(|prompt| prompt == "login")
					&& (!reauthenticated
						|| *auth_time + (self.config.clock_skew.as_secs() as i64)
							< pending.requested_at);
				let max_age_exceeded = pending.max_age.is_some_and(|age| {
					(age == 0 && (!reauthenticated || *auth_time < pending.requested_at))
						|| now() - *auth_time
							> (age as i64).saturating_add(self.config.clock_skew.as_secs() as i64)
				});
				if !active {
					Some(OidcError::AccessDenied)
				} else if login_required || max_age_exceeded {
					Some(OidcError::LoginRequired)
				} else if !consented {
					Some(if pending.prompts.iter().any(|prompt| prompt == "none") {
						OidcError::ConsentRequired
					} else {
						OidcError::AccessDenied
					})
				} else if pending.prompts.iter().any(|prompt| prompt == "consent")
					&& !consent_prompted
				{
					Some(OidcError::ConsentRequired)
				} else if pending
					.prompts
					.iter()
					.any(|prompt| prompt == "select_account")
					&& !account_selected
				{
					Some(OidcError::AccountSelectionRequired)
				} else {
					None
				}
			}
		};
		if let Some(error) = denial {
			self.oauth
				.complete_oidc_authorization(
					pending_id,
					browser_session,
					AuthorizationDecision::Deny,
				)
				.await?;
			tracing::info!(event = "oidc_authorization_denied", client_id = %pending.client_id, reason = error.as_str());
			return self.redirect_error(&pending.redirect_uri, pending.state.as_deref(), error);
		}
		let OidcAuthorizationDecision::Approve {
			user_id, auth_time, ..
		} = decision
		else {
			return Err(OidcError::ServerError);
		};
		let subject = random_subject();
		self.state
			.subject_or_insert(&user_id, &subject)
			.await
			.map_err(|_| OidcError::ServerError)?;
		let redirect = self
			.oauth
			.complete_oidc_authorization(
				pending_id,
				browser_session,
				AuthorizationDecision::Approve {
					user_id: user_id.clone(),
					scopes: vec!["openid".to_owned()],
				},
			)
			.await?;
		let url = Url::parse(&redirect).map_err(|_| OidcError::ServerError)?;
		let Some(code) = url
			.query_pairs()
			.find(|(key, _)| key == "code")
			.map(|(_, value)| value.into_owned())
		else {
			return Ok(redirect);
		};
		self.state
			.put_code(OidcCodeContext {
				digest: digest(&code),
				user_id,
				client_id: pending.client_id.clone(),
				nonce: pending.nonce,
				auth_time,
				expires_at: now() + self.oauth.config().code_ttl.as_secs() as i64,
			})
			.await
			.map_err(|_| OidcError::ServerError)?;
		tracing::info!(event = "oidc_authorization_code_issued", client_id = %pending.client_id);
		Ok(redirect)
	}

	/// Redirect an authorization error only to an enabled client's exact URI.
	pub async fn authorization_error_redirect(
		&self,
		client_id: &str,
		redirect_uri: &str,
		state: Option<&str>,
		error: OidcError,
	) -> Option<String> {
		let client = self.oauth.client(client_id).await.ok().flatten()?;
		if !client.enabled
			|| client.kind != ClientKind::Confidential
			|| !client.authorization_code
			|| !client.oidc_enabled
			|| !client.redirect_uris.iter().any(|uri| uri == redirect_uri)
		{
			return None;
		}
		let redirect = Url::parse(redirect_uri).ok()?;
		if redirect.scheme() != "https"
			&& !(self.config.allow_loopback_http
				&& redirect.scheme() == "http"
				&& matches!(
					redirect.host_str(),
					Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
				)) {
			return None;
		}
		self.redirect_error(redirect_uri, state, error).ok()
	}

	fn redirect_error(
		&self,
		redirect_uri: &str,
		state: Option<&str>,
		error: OidcError,
	) -> Result<String, OidcError> {
		let mut url = Url::parse(redirect_uri).map_err(|_| OidcError::InvalidRequest)?;
		url.query_pairs_mut().append_pair("error", error.as_str());
		if let Some(state) = state {
			url.query_pairs_mut().append_pair("state", state);
		}
		url.query_pairs_mut()
			.append_pair("iss", &self.config.issuer);
		Ok(url.into())
	}

	/// Redeem an OIDC code and issue a signed ID Token plus UserInfo token.
	pub async fn exchange_code(
		&self,
		code: &str,
		client_id: &str,
		client_secret: &str,
		redirect_uri: &str,
		verifier: &str,
	) -> Result<OidcTokenResponse, OidcError> {
		let context = self
			.state
			.code(&digest(code))
			.await
			.map_err(|_| OidcError::ServerError)?
			.filter(|context| context.expires_at > now() && context.client_id == client_id)
			.ok_or(OidcError::InvalidGrant)?;
		if !self
			.accounts
			.is_active(&context.user_id)
			.await
			.map_err(|_| OidcError::ServerError)?
		{
			return Err(OidcError::InvalidGrant);
		}
		let subject = self
			.state
			.subject(&context.user_id)
			.await
			.map_err(|_| OidcError::ServerError)?
			.ok_or(OidcError::InvalidGrant)?;
		let id_token = self.sign_id_token(&context, &subject).await?;
		let (issued, redeemed_user) = self
			.oauth
			.exchange_oidc_code(CodeExchangeRequest {
				code,
				client_id,
				client_secret: Some(client_secret),
				redirect_uri,
				verifier,
				resource: Some(&self.config.userinfo_endpoint),
				ttl: self.config.access_token_ttl,
			})
			.await?;
		if redeemed_user != context.user_id || issued.scope != "openid" {
			let _ = self
				.oauth
				.revoke(&issued.access_token, client_id, Some(client_secret))
				.await;
			return Err(OidcError::ServerError);
		}
		tracing::info!(event = "oidc_token_issued", client_id);
		Ok(OidcTokenResponse {
			access_token: issued.access_token,
			token_type: issued.token_type,
			expires_in: issued.expires_in,
			scope: issued.scope,
			id_token,
		})
	}

	async fn sign_id_token(
		&self,
		context: &OidcCodeContext,
		subject: &str,
	) -> Result<String, OidcError> {
		let key = self
			.state
			.active_key()
			.await
			.map_err(|_| OidcError::ServerError)?
			.ok_or(OidcError::ServerError)?;
		if self
			.signer
			.public_key(&key.public.kid)
			.await
			.map_err(|_| OidcError::ServerError)?
			!= Some(key.public.clone())
		{
			return Err(OidcError::ServerError);
		}
		let header = json!({"alg":"RS256","typ":"JWT","kid":key.public.kid});
		let issued_at = now();
		let mut claims = json!({
			"iss": self.config.issuer,
			"sub": subject,
			"aud": context.client_id,
			"iat": issued_at,
			"exp": issued_at + self.config.id_token_ttl.as_secs() as i64,
			"auth_time": context.auth_time
		});
		if let Some(nonce) = &context.nonce {
			claims["nonce"] = json!(nonce);
		}
		let header = serde_json::to_vec(&header).map_err(|_| OidcError::ServerError)?;
		let claims = serde_json::to_vec(&claims).map_err(|_| OidcError::ServerError)?;
		let input = format!(
			"{}.{}",
			URL_SAFE_NO_PAD.encode(header),
			URL_SAFE_NO_PAD.encode(claims)
		);
		let signature = self
			.signer
			.sign(&key.public.kid, input.as_bytes())
			.await
			.map_err(|_| OidcError::ServerError)?;
		Ok(format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature)))
	}

	/// Return only the published subject for an active UserInfo credential.
	pub async fn userinfo(&self, token: &str) -> Result<Option<Value>, OidcError> {
		let Some(info) = self.oauth.token_info(token).await? else {
			return Ok(None);
		};
		if info.audience != self.config.userinfo_endpoint
			|| info.scopes.len() != 1
			|| info.scopes[0] != "openid"
		{
			return Ok(None);
		}
		let crate::oauth2_server::TokenPrincipal::User(user_id) = info.principal else {
			return Ok(None);
		};
		if !self
			.accounts
			.is_active(&user_id)
			.await
			.map_err(|_| OidcError::ServerError)?
		{
			return Ok(None);
		}
		let subject = self
			.state
			.subject(&user_id)
			.await
			.map_err(|_| OidcError::ServerError)?;
		Ok(subject.map(|sub| json!({"sub":sub})))
	}

	/// Invalidate codes and tokens and retire the published subject after deletion.
	pub async fn retire_user(&self, user_id: &str) -> Result<(), OidcError> {
		if !self.production {
			self.oauth.retire_user(user_id).await?;
		}
		self.state
			.retire_user(user_id)
			.await
			.map_err(|_| OidcError::ServerError)?;
		tracing::info!(event = "oidc_subject_retired");
		Ok(())
	}
}

/// Host-facing handle for a stored authorization request.
#[derive(Clone, Debug)]
pub struct OidcPendingHandle {
	/// Opaque single-use continuation identifier.
	pub id: String,
	/// Validated OIDC fields needed by the host interaction.
	pub request: OidcPending,
}

fn parse_prompts(value: Option<&str>) -> Result<Vec<String>, OidcError> {
	let Some(value) = value else {
		return Ok(Vec::new());
	};
	let values: Vec<_> = value.split(' ').map(str::to_owned).collect();
	if values.is_empty()
		|| values
			.iter()
			.any(|v| !matches!(v.as_str(), "none" | "login" | "consent" | "select_account"))
		|| values
			.iter()
			.collect::<std::collections::HashSet<_>>()
			.len() != values.len()
		|| (values.iter().any(|v| v == "none") && values.len() != 1)
	{
		return Err(OidcError::InvalidRequest);
	}
	Ok(values)
}

fn random_subject() -> String {
	let mut bytes = [0u8; 32];
	rand::rng().fill_bytes(&mut bytes);
	URL_SAFE_NO_PAD.encode(bytes)
}

fn digest(value: &str) -> String {
	hex::encode(Sha256::digest(value.as_bytes()))
}

fn now() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs() as i64
}
