//! Authorization, issuance, and inspection rules.

use super::store::{
	ClientKind, ClientRegistration, CodeRedemption, OAuthServerStore, PendingRecord,
	ResourceRegistration, StoredCode, StoredToken,
};
use crate::repository::UserRepository;
use argon2::Argon2;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use password_hash::{
	PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString, rand_core::OsRng,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
	collections::HashSet,
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub(crate) struct CodeExchangeRequest<'a> {
	pub(crate) code: &'a str,
	pub(crate) client_id: &'a str,
	pub(crate) client_secret: Option<&'a str>,
	pub(crate) redirect_uri: &'a str,
	pub(crate) verifier: &'a str,
	pub(crate) resource: Option<&'a str>,
	pub(crate) ttl: Duration,
}

/// OAuth error returned by the protocol core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OAuthError {
	/// A parameter or request was malformed.
	InvalidRequest,
	/// Client authentication failed.
	InvalidClient,
	/// The client is not allowed to use the requested grant.
	UnauthorizedClient,
	/// A code or grant is invalid or expired.
	InvalidGrant,
	/// A scope is invalid or outside the client's allowlist.
	InvalidScope,
	/// A resource audience is invalid.
	InvalidTarget,
	/// The requested grant is not implemented.
	UnsupportedGrantType,
	/// The requested authorization response type is not implemented.
	UnsupportedResponseType,
	/// The authenticated user denied or cannot approve access.
	AccessDenied,
	/// Persistent storage or another internal operation failed.
	ServerError,
}
impl OAuthError {
	/// OAuth wire error code.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::InvalidRequest => "invalid_request",
			Self::InvalidClient => "invalid_client",
			Self::UnauthorizedClient => "unauthorized_client",
			Self::InvalidGrant => "invalid_grant",
			Self::InvalidScope => "invalid_scope",
			Self::InvalidTarget => "invalid_target",
			Self::UnsupportedGrantType => "unsupported_grant_type",
			Self::UnsupportedResponseType => "unsupported_response_type",
			Self::AccessDenied => "access_denied",
			Self::ServerError => "server_error",
		}
	}
}
impl std::fmt::Display for OAuthError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
impl std::error::Error for OAuthError {}

/// User or client on whose behalf a token was issued.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum TokenPrincipal {
	/// A delegated user principal.
	User(String),
	/// A client acting on its own behalf.
	Client(String),
}

/// Protocol lifetimes and published endpoint URLs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OAuthServerConfig {
	/// Absolute HTTPS issuer, without a trailing slash.
	pub issuer: String,
	/// Absolute authorization endpoint URL.
	pub authorization_endpoint: String,
	/// Absolute token endpoint URL.
	pub token_endpoint: String,
	/// Absolute revocation endpoint URL.
	pub revocation_endpoint: String,
	/// Absolute introspection endpoint URL.
	pub introspection_endpoint: String,
	/// Lifetime of an authorization code (at most five minutes).
	pub code_ttl: Duration,
	/// Lifetime of a pending authorization (at most ten minutes).
	pub pending_ttl: Duration,
	/// Lifetime of an access token (at most one hour).
	pub token_ttl: Duration,
}
impl OAuthServerConfig {
	/// Construct defaults from an HTTPS issuer and explicit mounted URLs.
	pub fn new(
		issuer: &str,
		authorization_endpoint: &str,
		token_endpoint: &str,
		revocation_endpoint: &str,
		introspection_endpoint: &str,
	) -> Result<Self, OAuthError> {
		let config = Self {
			issuer: issuer.trim_end_matches('/').to_owned(),
			authorization_endpoint: authorization_endpoint.to_owned(),
			token_endpoint: token_endpoint.to_owned(),
			revocation_endpoint: revocation_endpoint.to_owned(),
			introspection_endpoint: introspection_endpoint.to_owned(),
			code_ttl: Duration::from_secs(300),
			pending_ttl: Duration::from_secs(600),
			token_ttl: Duration::from_secs(3600),
		};
		if config.issuer.starts_with("http://") {
			return Err(OAuthError::InvalidRequest);
		}
		config.validate()?;
		Ok(config)
	}
	/// Construct an explicit HTTP loopback issuer for development and tests.
	pub fn for_loopback_development(
		issuer: &str,
		authorization_endpoint: &str,
		token_endpoint: &str,
		revocation_endpoint: &str,
		introspection_endpoint: &str,
	) -> Result<Self, OAuthError> {
		let config = Self {
			issuer: issuer.trim_end_matches('/').to_owned(),
			authorization_endpoint: authorization_endpoint.to_owned(),
			token_endpoint: token_endpoint.to_owned(),
			revocation_endpoint: revocation_endpoint.to_owned(),
			introspection_endpoint: introspection_endpoint.to_owned(),
			code_ttl: Duration::from_secs(300),
			pending_ttl: Duration::from_secs(600),
			token_ttl: Duration::from_secs(3600),
		};
		if !config.issuer.starts_with("http://") {
			return Err(OAuthError::InvalidRequest);
		}
		config.validate()?;
		Ok(config)
	}
	/// RFC 8414 well-known metadata URL derived from the issuer.
	pub fn metadata_url(&self) -> Result<String, OAuthError> {
		self.validate()?;
		let issuer = Url::parse(&self.issuer).map_err(|_| OAuthError::InvalidRequest)?;
		let suffix = issuer.path().trim_start_matches('/');
		if suffix.is_empty() {
			Ok(format!(
				"{}/.well-known/oauth-authorization-server",
				issuer.origin().ascii_serialization()
			))
		} else {
			Ok(format!(
				"{}/.well-known/oauth-authorization-server/{suffix}",
				issuer.origin().ascii_serialization()
			))
		}
	}
	fn validate(&self) -> Result<(), OAuthError> {
		let issuer = Url::parse(&self.issuer).map_err(|_| OAuthError::InvalidRequest)?;
		if !(issuer.scheme() == "https"
			|| (issuer.scheme() == "http"
				&& matches!(
					issuer.host_str(),
					Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
				))) || issuer.query().is_some()
			|| issuer.fragment().is_some()
			|| issuer.username() != ""
			|| issuer.password().is_some()
		{
			return Err(OAuthError::InvalidRequest);
		}
		for endpoint in [
			&self.authorization_endpoint,
			&self.token_endpoint,
			&self.revocation_endpoint,
			&self.introspection_endpoint,
		] {
			let url = Url::parse(endpoint).map_err(|_| OAuthError::InvalidRequest)?;
			if url.scheme() != issuer.scheme()
				|| url.origin() != issuer.origin()
				|| !url.username().is_empty()
				|| url.password().is_some()
				|| url.query().is_some()
				|| url.fragment().is_some()
			{
				return Err(OAuthError::InvalidRequest);
			}
		}
		if self.code_ttl.as_secs() == 0
			|| self.code_ttl > Duration::from_secs(300)
			|| self.pending_ttl.as_secs() == 0
			|| self.pending_ttl > Duration::from_secs(600)
			|| self.token_ttl.as_secs() == 0
			|| self.token_ttl > Duration::from_secs(3600)
		{
			return Err(OAuthError::InvalidRequest);
		}
		Ok(())
	}
}

/// Validated request shown to the host's login and consent UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingAuthorization {
	/// Opaque, one-time pending-request identifier.
	pub id: String,
	/// Registered client identifier.
	pub client_id: String,
	/// Registered redirect URI.
	pub redirect_uri: String,
	/// Requested scopes.
	pub scopes: Vec<String>,
	/// Resource audience.
	pub audience: String,
	/// PKCE S256 challenge.
	pub code_challenge: String,
	/// Client state, passed through unchanged.
	pub state: Option<String>,
}

/// Authorization query supplied by the browser.
#[derive(Clone, Debug)]
pub struct AuthorizationRequest {
	/// Client identifier.
	pub client_id: String,
	/// Registered redirect URI.
	pub redirect_uri: String,
	/// Response type; only `code` is supported.
	pub response_type: String,
	/// S256 challenge.
	pub code_challenge: String,
	/// Challenge method; only `S256` is supported.
	pub code_challenge_method: String,
	/// Space-separated scopes.
	pub scope: Option<String>,
	/// RFC 8707 resource indicator.
	pub resource: Option<String>,
	/// Opaque client state.
	pub state: Option<String>,
}

/// Host decision bound to the pending request and browser session.
#[derive(Clone, Debug)]
pub enum AuthorizationDecision {
	/// Approve for an authenticated active user and a subset of requested scopes.
	Approve {
		/// Authenticated user ID from the host.
		user_id: String,
		/// Scopes explicitly approved by the host.
		scopes: Vec<String>,
	},
	/// Deny access.
	Deny,
}

/// Access token data returned only at issuance.
#[derive(Clone, Debug, Serialize)]
pub struct IssuedToken {
	/// Opaque bearer token.
	pub access_token: String,
	/// Always `Bearer`.
	pub token_type: &'static str,
	/// Lifetime in seconds.
	pub expires_in: u64,
	/// Space-separated granted scopes.
	pub scope: String,
}

/// Active token metadata; never includes the raw token.
#[derive(Clone, Debug, Serialize)]
pub struct TokenInfo {
	/// Owning client.
	pub client_id: String,
	/// User or client principal.
	pub principal: TokenPrincipal,
	/// Granted scopes.
	pub scopes: Vec<String>,
	/// Single audience.
	pub audience: String,
	/// UNIX issuance time.
	pub issued_at: i64,
	/// UNIX expiry time.
	pub expires_at: i64,
}
impl TokenInfo {
	/// Check whether the token includes the requested scope.
	pub fn has_scope(&self, scope: &str) -> bool {
		self.scopes.iter().any(|s| s == scope)
	}
}

/// Distributed rate-limiter contract for protocol endpoints.
#[async_trait]
pub trait OAuthRateLimiter: Send + Sync {
	/// Allow or reject an operation for a client/session key.
	async fn allow(&self, key: &str) -> bool;
}
/// Marker for a limiter whose counters are shared across server instances.
pub trait SharedOAuthRateLimiter: OAuthRateLimiter {}

/// Authorization server using an expiry- and principal-aware store.
pub struct OAuthServer {
	config: OAuthServerConfig,
	store: Arc<dyn OAuthServerStore>,
	users: Arc<dyn UserRepository>,
	limiter: Arc<dyn OAuthRateLimiter>,
	production: bool,
}
impl OAuthServer {
	/// Validated configuration used by this server.
	pub fn config(&self) -> &OAuthServerConfig {
		&self.config
	}
	/// Development/test constructor; the store and limiter may be in-memory.
	pub fn for_development(
		config: OAuthServerConfig,
		store: Arc<dyn OAuthServerStore>,
		users: Arc<dyn UserRepository>,
		limiter: Arc<dyn OAuthRateLimiter>,
	) -> Result<Self, OAuthError> {
		config.validate()?;
		Ok(Self {
			config,
			store,
			users,
			limiter,
			production: false,
		})
	}
	/// Production constructor requiring PostgreSQL and a host-provided shared limiter.
	#[cfg(feature = "database")]
	pub fn for_production<L: SharedOAuthRateLimiter + 'static>(
		config: OAuthServerConfig,
		store: super::postgres::PostgresOAuthStore,
		users: Arc<dyn UserRepository>,
		limiter: Arc<L>,
	) -> Result<Self, OAuthError> {
		config.validate()?;
		if config.issuer.starts_with("http://") {
			return Err(OAuthError::InvalidRequest);
		}
		Ok(Self {
			config,
			store: Arc::new(store),
			users,
			limiter,
			production: true,
		})
	}
	/// Whether this server requires transport security for incoming requests.
	pub fn is_production(&self) -> bool {
		self.production
	}
	/// Rate-limit a public operation; key design is supplied by the host.
	pub async fn allow(&self, key: &str) -> bool {
		self.limiter.allow(key).await
	}
	/// Get a client for CORS and authorization checks.
	pub async fn client(&self, id: &str) -> Result<Option<ClientRegistration>, OAuthError> {
		self.store
			.client(id)
			.await
			.map_err(|_| OAuthError::ServerError)
	}
	/// Determine whether a browser origin is registered to an enabled public client.
	pub async fn allows_origin(&self, origin: &str) -> Result<bool, OAuthError> {
		Ok(self
			.store
			.public_client_for_origin(origin)
			.await
			.map_err(|_| OAuthError::ServerError)?
			.is_some())
	}
	/// Construct an OAuth error redirect only for a registered client and redirect URI.
	pub async fn authorization_error_redirect(
		&self,
		client_id: &str,
		redirect_uri: &str,
		state: Option<&str>,
		error: OAuthError,
	) -> Option<String> {
		let client = self.client(client_id).await.ok().flatten()?;
		if !client.enabled
			|| !client.authorization_code
			|| !client
				.redirect_uris
				.iter()
				.any(|uri| redirect_matches(uri, redirect_uri))
		{
			return None;
		}
		let mut url = Url::parse(redirect_uri).ok()?;
		url.query_pairs_mut().append_pair("error", error.as_str());
		if let Some(state) = state {
			url.query_pairs_mut().append_pair("state", state);
		}
		url.query_pairs_mut()
			.append_pair("iss", &self.config.issuer);
		Some(url.into())
	}
	/// Register a validated client; returns a newly generated secret for confidential clients.
	pub async fn register_client(
		&self,
		mut client: ClientRegistration,
	) -> Result<Option<String>, OAuthError> {
		validate_client_registration(&client)?;
		for audience in &client.audiences {
			let resource = self
				.store
				.resource_for_audience(audience)
				.await
				.map_err(|_| OAuthError::ServerError)?;
			if !resource.is_some_and(|r| r.enabled) {
				return Err(OAuthError::InvalidTarget);
			}
		}
		let secret = if client.kind == ClientKind::Confidential {
			let raw = random_secret();
			client.secret_hash = Some(hash_password(&raw).await?);
			client.previous_secret_hash = None;
			client.previous_secret_expires_at = None;
			Some(raw)
		} else {
			client.secret_hash = None;
			client.previous_secret_hash = None;
			client.previous_secret_expires_at = None;
			None
		};
		self.store
			.put_client(client)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(secret)
	}
	/// Rotate a confidential client's secret; old credentials become invalid immediately.
	pub async fn rotate_client_secret(&self, id: &str) -> Result<String, OAuthError> {
		self.rotate_client_secret_with_overlap(id, Duration::ZERO)
			.await
	}
	/// Rotate a client secret with at most twenty-four hours of old-secret overlap.
	pub async fn rotate_client_secret_with_overlap(
		&self,
		id: &str,
		overlap: Duration,
	) -> Result<String, OAuthError> {
		if overlap > Duration::from_secs(24 * 3600) {
			return Err(OAuthError::InvalidRequest);
		}
		let mut client = self.active_client(id).await?;
		if client.kind != ClientKind::Confidential {
			return Err(OAuthError::InvalidClient);
		}
		let raw = random_secret();
		client.previous_secret_hash = if overlap.is_zero() {
			None
		} else {
			client.secret_hash.take()
		};
		client.previous_secret_expires_at = if overlap.is_zero() {
			None
		} else {
			Some(now() + overlap.as_secs() as i64)
		};
		client.secret_hash = Some(hash_password(&raw).await?);
		self.store
			.put_client(client)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(raw)
	}
	/// Immediately invalidate a previous client credential during rotation.
	pub async fn revoke_previous_client_secret(&self, id: &str) -> Result<(), OAuthError> {
		let mut client = self.active_client(id).await?;
		client.previous_secret_hash = None;
		client.previous_secret_expires_at = None;
		self.store
			.put_client(client)
			.await
			.map_err(|_| OAuthError::ServerError)
	}
	/// Disable a registration before revoking all of its issued access tokens.
	pub async fn disable_client(&self, id: &str) -> Result<u64, OAuthError> {
		let mut client = self.active_client(id).await?;
		client.enabled = false;
		self.store
			.put_client(client)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		self.revoke_client(id).await
	}
	/// Register a resource server and return its new introspection secret.
	pub async fn register_resource(
		&self,
		resource_id: &str,
		audience: &str,
	) -> Result<String, OAuthError> {
		if resource_id.is_empty()
			|| audience.is_empty()
			|| !valid_resource_uri(audience)
			|| (audience.starts_with("http://") && !self.config.issuer.starts_with("http://"))
		{
			return Err(OAuthError::InvalidRequest);
		}
		let raw = random_secret();
		self.store
			.put_resource(ResourceRegistration {
				resource_id: resource_id.to_owned(),
				audience: audience.to_owned(),
				secret_hash: hash_password(&raw).await?,
				enabled: true,
			})
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(raw)
	}
	/// Rotate a resource server's introspection secret.
	pub async fn rotate_resource_secret(&self, id: &str) -> Result<String, OAuthError> {
		let mut resource = self
			.store
			.resource(id)
			.await
			.map_err(|_| OAuthError::ServerError)?
			.ok_or(OAuthError::InvalidClient)?;
		let raw = random_secret();
		resource.secret_hash = hash_password(&raw).await?;
		self.store
			.put_resource(resource)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(raw)
	}
	/// Validate a browser authorization request and persist a pending decision.
	pub async fn begin_authorization(
		&self,
		request: AuthorizationRequest,
		browser_session: &str,
	) -> Result<PendingAuthorization, OAuthError> {
		self.begin_authorization_inner(request, browser_session, false)
			.await
	}
	/// Begin an OIDC request while binding openid to the OIDC code endpoint.
	pub(crate) async fn begin_oidc_authorization(
		&self,
		request: AuthorizationRequest,
		browser_session: &str,
	) -> Result<PendingAuthorization, OAuthError> {
		self.begin_authorization_inner(request, browser_session, true)
			.await
	}
	async fn begin_authorization_inner(
		&self,
		request: AuthorizationRequest,
		browser_session: &str,
		oidc: bool,
	) -> Result<PendingAuthorization, OAuthError> {
		if request.response_type != "code" {
			return Err(OAuthError::UnsupportedResponseType);
		}
		if browser_session.is_empty()
			|| request.code_challenge_method != "S256"
			|| !valid_challenge(&request.code_challenge)
		{
			return Err(OAuthError::InvalidRequest);
		}
		let client = self.active_client(&request.client_id).await?;
		if !client.authorization_code {
			return Err(OAuthError::UnauthorizedClient);
		}
		if !client
			.redirect_uris
			.iter()
			.any(|uri| redirect_matches(uri, &request.redirect_uri))
		{
			return Err(OAuthError::InvalidRequest);
		}
		let scopes = choose_scopes(&client, request.scope.as_deref())?;
		if (oidc && (!client.oidc_enabled || scopes.len() != 1 || scopes[0] != "openid"))
			|| (!oidc && scopes.iter().any(|scope| scope == "openid"))
		{
			return Err(OAuthError::InvalidScope);
		}
		let audience = choose_audience(&client, request.resource.as_deref())?;
		self.active_audience(&audience).await?;
		let pending = PendingAuthorization {
			id: random_secret(),
			client_id: client.client_id,
			redirect_uri: request.redirect_uri,
			scopes,
			audience,
			code_challenge: request.code_challenge,
			state: request.state,
		};
		let record = PendingRecord {
			request: pending.clone(),
			oidc,
			session_digest: digest(browser_session),
			expires_at: now() + self.config.pending_ttl.as_secs() as i64,
		};
		self.store
			.put_pending(&pending.id, record)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(pending)
	}
	/// Complete exactly one pending request. Returns the validated redirect URL.
	pub async fn complete_authorization(
		&self,
		pending_id: &str,
		browser_session: &str,
		decision: AuthorizationDecision,
	) -> Result<String, OAuthError> {
		self.complete_authorization_inner(pending_id, browser_session, decision, false)
			.await
	}
	/// Complete an OIDC authorization; the resulting code can only be redeemed by OIDC.
	pub(crate) async fn complete_oidc_authorization(
		&self,
		pending_id: &str,
		browser_session: &str,
		decision: AuthorizationDecision,
	) -> Result<String, OAuthError> {
		self.complete_authorization_inner(pending_id, browser_session, decision, true)
			.await
	}
	async fn complete_authorization_inner(
		&self,
		pending_id: &str,
		browser_session: &str,
		decision: AuthorizationDecision,
		oidc: bool,
	) -> Result<String, OAuthError> {
		if browser_session.is_empty() {
			return Err(OAuthError::InvalidRequest);
		}
		let pending = self
			.store
			.take_pending(pending_id)
			.await
			.map_err(|_| OAuthError::ServerError)?
			.ok_or(OAuthError::InvalidGrant)?;
		if pending.expires_at <= now()
			|| pending.session_digest != digest(browser_session)
			|| pending.oidc != oidc
		{
			return Err(OAuthError::InvalidGrant);
		}
		let request = pending.request;
		let client = self.active_client(&request.client_id).await?;
		if !client.authorization_code
			|| (oidc && !client.oidc_enabled)
			|| !client
				.redirect_uris
				.iter()
				.any(|uri| redirect_matches(uri, &request.redirect_uri))
		{
			return Err(OAuthError::InvalidGrant);
		}
		let mut redirect =
			Url::parse(&request.redirect_uri).map_err(|_| OAuthError::InvalidRequest)?;
		match decision {
			AuthorizationDecision::Deny => {
				redirect
					.query_pairs_mut()
					.append_pair("error", "access_denied");
			}
			AuthorizationDecision::Approve { user_id, scopes } => {
				let user = self
					.users
					.get_user_by_id(&user_id)
					.await
					.map_err(|_| OAuthError::ServerError)?;
				if !user.is_some_and(|user| user.is_account_active() && user.is_authenticated()) {
					redirect
						.query_pairs_mut()
						.append_pair("error", "access_denied");
				} else {
					if !subset(&scopes, &request.scopes) {
						return Err(OAuthError::InvalidScope);
					}
					let raw = random_secret();
					self.store
						.put_code(StoredCode {
							digest: digest(&raw),
							client_id: request.client_id,
							redirect_uri: request.redirect_uri,
							challenge: request.code_challenge,
							user_id,
							scopes,
							audience: request.audience,
							oidc,
							expires_at: now() + self.config.code_ttl.as_secs() as i64,
							redeemed: false,
							replayed: false,
						})
						.await
						.map_err(|_| OAuthError::ServerError)?;
					redirect.query_pairs_mut().append_pair("code", &raw);
				}
			}
		}
		if let Some(state) = request.state {
			redirect.query_pairs_mut().append_pair("state", &state);
		}
		redirect
			.query_pairs_mut()
			.append_pair("iss", &self.config.issuer);
		Ok(redirect.into())
	}
	/// Exchange a one-time code for an opaque access token.
	pub async fn exchange_code(
		&self,
		code: &str,
		client_id: &str,
		client_secret: Option<&str>,
		redirect_uri: &str,
		verifier: &str,
		resource: Option<&str>,
	) -> Result<IssuedToken, OAuthError> {
		self.exchange_code_inner(
			CodeExchangeRequest {
				code,
				client_id,
				client_secret,
				redirect_uri,
				verifier,
				resource,
				ttl: self.config.token_ttl,
			},
			false,
		)
		.await
		.map(|(token, _)| token)
	}
	/// Redeem an OIDC code with an OIDC-specific access-token lifetime.
	pub(crate) async fn exchange_oidc_code(
		&self,
		request: CodeExchangeRequest<'_>,
	) -> Result<(IssuedToken, String), OAuthError> {
		if request.ttl.is_zero() || request.ttl > Duration::from_secs(3600) {
			return Err(OAuthError::InvalidRequest);
		}
		self.exchange_code_inner(request, true).await
	}
	async fn exchange_code_inner(
		&self,
		request: CodeExchangeRequest<'_>,
		expect_oidc: bool,
	) -> Result<(IssuedToken, String), OAuthError> {
		let CodeExchangeRequest {
			code,
			client_id,
			client_secret,
			redirect_uri,
			verifier,
			resource,
			ttl,
		} = request;
		let client = self.authenticate_client(client_id, client_secret).await?;
		if !client.authorization_code || (expect_oidc && !client.oidc_enabled) {
			return Err(OAuthError::UnauthorizedClient);
		}
		if !valid_verifier(verifier) {
			return Err(OAuthError::InvalidGrant);
		}
		let redeemed = self
			.store
			.redeem_code(
				&digest(code),
				client_id,
				redirect_uri,
				&challenge(verifier),
				resource,
				now(),
			)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		let CodeRedemption::Valid(record) = redeemed else {
			return Err(OAuthError::InvalidGrant);
		};
		if record.oidc != expect_oidc {
			return Err(OAuthError::InvalidGrant);
		}
		let user_id = record.user_id.clone();
		let token = self
			.issue(
				&client,
				TokenPrincipal::User(record.user_id),
				record.scopes,
				record.audience,
				Some(record.digest),
				ttl,
			)
			.await?;
		Ok((token, user_id))
	}
	/// Issue a client-credentials token for a confidential client.
	pub async fn client_credentials(
		&self,
		client_id: &str,
		secret: &str,
		scope: Option<&str>,
		resource: Option<&str>,
	) -> Result<IssuedToken, OAuthError> {
		let client = self.authenticate_client(client_id, Some(secret)).await?;
		if client.kind != ClientKind::Confidential || !client.client_credentials {
			return Err(OAuthError::UnauthorizedClient);
		}
		let scopes = choose_scopes(&client, scope)?;
		if scopes.iter().any(|scope| scope == "openid") {
			return Err(OAuthError::InvalidScope);
		}
		let audience = choose_audience(&client, resource)?;
		self.active_audience(&audience).await?;
		self.issue(
			&client,
			TokenPrincipal::Client(client_id.to_owned()),
			scopes,
			audience,
			None,
			self.config.token_ttl,
		)
		.await
	}
	async fn issue(
		&self,
		client: &ClientRegistration,
		principal: TokenPrincipal,
		scopes: Vec<String>,
		audience: String,
		code_digest: Option<String>,
		ttl: Duration,
	) -> Result<IssuedToken, OAuthError> {
		let raw = random_secret();
		let issued_at = now();
		self.store
			.put_token(StoredToken {
				digest: digest(&raw),
				client_id: client.client_id.clone(),
				principal,
				scopes: scopes.clone(),
				audience,
				issued_at,
				expires_at: issued_at + ttl.as_secs() as i64,
				revoked: false,
				code_digest,
			})
			.await
			.map_err(|_| OAuthError::ServerError)?;
		Ok(IssuedToken {
			access_token: raw,
			token_type: "Bearer",
			expires_in: ttl.as_secs(),
			scope: scopes.join(" "),
		})
	}
	/// Look up active token metadata. User tokens require an active account.
	pub async fn token_info(&self, token: &str) -> Result<Option<TokenInfo>, OAuthError> {
		let Some(record) = self
			.store
			.token(&digest(token))
			.await
			.map_err(|_| OAuthError::ServerError)?
		else {
			return Ok(None);
		};
		if record.revoked || record.expires_at <= now() {
			return Ok(None);
		}
		let resource = self
			.store
			.resource_for_audience(&record.audience)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		if !self
			.client(&record.client_id)
			.await?
			.is_some_and(|c| c.enabled)
			|| !resource.is_some_and(|r| r.enabled)
		{
			return Ok(None);
		}
		if let TokenPrincipal::User(ref id) = record.principal {
			let user = self
				.users
				.get_user_by_id(id)
				.await
				.map_err(|_| OAuthError::ServerError)?;
			if !user.is_some_and(|u| u.is_account_active() && u.is_authenticated()) {
				return Ok(None);
			}
		}
		Ok(Some(TokenInfo {
			client_id: record.client_id,
			principal: record.principal,
			scopes: record.scopes,
			audience: record.audience,
			issued_at: record.issued_at,
			expires_at: record.expires_at,
		}))
	}
	/// Revoke an access token, limited to its owning client.
	pub async fn revoke(
		&self,
		token: &str,
		client_id: &str,
		secret: Option<&str>,
	) -> Result<(), OAuthError> {
		self.authenticate_client(client_id, secret).await?;
		self.store
			.revoke_token(&digest(token), client_id)
			.await
			.map_err(|_| OAuthError::ServerError)
	}
	/// Inspect a token for an authenticated resource server and its audience.
	pub async fn introspect(
		&self,
		token: &str,
		resource_id: &str,
		secret: &str,
	) -> Result<Option<TokenInfo>, OAuthError> {
		let resource = self
			.store
			.resource(resource_id)
			.await
			.map_err(|_| OAuthError::ServerError)?
			.ok_or(OAuthError::InvalidClient)?;
		if !resource.enabled || !verify_password(secret, &resource.secret_hash).await? {
			return Err(OAuthError::InvalidClient);
		}
		Ok(self
			.token_info(token)
			.await?
			.filter(|info| info.audience == resource.audience))
	}
	/// Revoke all tokens issued to a user after a host account security event.
	pub async fn revoke_user(&self, user_id: &str) -> Result<u64, OAuthError> {
		self.store
			.revoke_user(user_id)
			.await
			.map_err(|_| OAuthError::ServerError)
	}
	/// Revoke all tokens issued to a client.
	pub async fn revoke_client(&self, client_id: &str) -> Result<u64, OAuthError> {
		self.store
			.revoke_client(client_id)
			.await
			.map_err(|_| OAuthError::ServerError)
	}
	async fn active_client(&self, id: &str) -> Result<ClientRegistration, OAuthError> {
		self.store
			.client(id)
			.await
			.map_err(|_| OAuthError::ServerError)?
			.filter(|c| c.enabled)
			.ok_or(OAuthError::InvalidClient)
	}
	async fn active_audience(&self, audience: &str) -> Result<(), OAuthError> {
		let resource = self
			.store
			.resource_for_audience(audience)
			.await
			.map_err(|_| OAuthError::ServerError)?;
		if resource.is_some_and(|r| r.enabled) {
			Ok(())
		} else {
			Err(OAuthError::InvalidTarget)
		}
	}
	async fn authenticate_client(
		&self,
		id: &str,
		secret: Option<&str>,
	) -> Result<ClientRegistration, OAuthError> {
		let client = self.active_client(id).await?;
		match client.kind {
			ClientKind::Public if secret.is_none() => Ok(client),
			ClientKind::Confidential => {
				let (Some(raw), Some(hash)) = (secret, client.secret_hash.as_deref()) else {
					return Err(OAuthError::InvalidClient);
				};
				let current_matches = verify_password(raw, hash).await?;
				let previous_matches = if client
					.previous_secret_expires_at
					.is_some_and(|end| end > now())
				{
					match client.previous_secret_hash.as_deref() {
						Some(previous) => verify_password(raw, previous).await?,
						None => false,
					}
				} else {
					false
				};
				if current_matches || previous_matches {
					Ok(client)
				} else {
					Err(OAuthError::InvalidClient)
				}
			}
			_ => Err(OAuthError::InvalidClient),
		}
	}
}

fn validate_client_registration(client: &ClientRegistration) -> Result<(), OAuthError> {
	if client.client_id.is_empty()
		|| !(client.authorization_code || client.client_credentials)
		|| (client.kind == ClientKind::Public && client.client_credentials)
		|| (client.kind != ClientKind::Public && !client.browser_origins.is_empty())
		|| (client.oidc_enabled
			&& (client.kind != ClientKind::Confidential
				|| !client.authorization_code
				|| !client.scopes.iter().any(|scope| scope == "openid")))
		|| !subset(&client.default_scopes, &client.scopes)
		|| client.scopes.iter().any(|s| !valid_scope(s))
		|| client.scopes.iter().collect::<HashSet<_>>().len() != client.scopes.len()
		|| client
			.default_audience
			.as_ref()
			.is_some_and(|a| !client.audiences.contains(a))
		|| client.audiences.is_empty()
		|| client.audiences.iter().any(|a| !valid_resource_uri(a))
	{
		return Err(OAuthError::InvalidRequest);
	}
	if client.authorization_code && client.redirect_uris.is_empty() {
		return Err(OAuthError::InvalidRequest);
	}
	for redirect in &client.redirect_uris {
		validate_redirect(redirect)?;
	}
	for origin in &client.browser_origins {
		let url = Url::parse(origin).map_err(|_| OAuthError::InvalidRequest)?;
		if url.scheme() != "https"
			|| url.origin().ascii_serialization() != *origin
			|| url.username() != ""
			|| url.password().is_some()
		{
			return Err(OAuthError::InvalidRequest);
		}
	}
	Ok(())
}
fn valid_resource_uri(value: &str) -> bool {
	Url::parse(value).is_ok_and(|url| {
		(url.scheme() == "https"
			|| (url.scheme() == "http"
				&& matches!(
					url.host_str(),
					Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
				))) && url.host_str().is_some()
			&& url.fragment().is_none()
			&& url.username().is_empty()
			&& url.password().is_none()
	})
}
fn validate_redirect(value: &str) -> Result<(), OAuthError> {
	let url = Url::parse(value).map_err(|_| OAuthError::InvalidRequest)?;
	if url.fragment().is_some() || url.username() != "" || url.password().is_some() {
		return Err(OAuthError::InvalidRequest);
	}
	match url.scheme() {
		"https" if url.host_str().is_some() => Ok(()),
		"http" if matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1")) => Ok(()),
		"http" => Err(OAuthError::InvalidRequest),
		scheme
			if scheme.contains('.')
				&& url.host_str().is_none()
				&& value.starts_with(&format!("{scheme}:/"))
				&& !value.starts_with(&format!("{scheme}://")) =>
		{
			Ok(())
		}
		_ => Err(OAuthError::InvalidRequest),
	}
}
fn redirect_matches(registered: &str, supplied: &str) -> bool {
	if registered == supplied {
		return true;
	}
	let (Ok(a), Ok(b)) = (Url::parse(registered), Url::parse(supplied)) else {
		return false;
	};
	if a.scheme() != "http"
		|| b.scheme() != "http"
		|| !matches!(a.host_str(), Some("127.0.0.1" | "[::1]" | "::1"))
		|| a.host_str() != b.host_str()
		|| b.port().is_none()
	{
		return false;
	}
	let mut canonical = b;
	canonical.set_port(a.port()).is_ok() && canonical.as_str() == registered
}
fn choose_scopes(
	client: &ClientRegistration,
	raw: Option<&str>,
) -> Result<Vec<String>, OAuthError> {
	let scopes = match raw {
		Some(s) => {
			if s.is_empty() {
				return Err(OAuthError::InvalidScope);
			}
			s.split(' ').map(str::to_owned).collect()
		}
		None => client.default_scopes.clone(),
	};
	if !subset(&scopes, &client.scopes)
		|| scopes.iter().any(|s| !valid_scope(s))
		|| scopes.iter().collect::<HashSet<_>>().len() != scopes.len()
	{
		return Err(OAuthError::InvalidScope);
	}
	Ok(scopes)
}
fn choose_audience(client: &ClientRegistration, raw: Option<&str>) -> Result<String, OAuthError> {
	let selected = raw
		.or(client.default_audience.as_deref())
		.ok_or(OAuthError::InvalidTarget)?;
	if !client.audiences.iter().any(|a| a == selected) {
		return Err(OAuthError::InvalidTarget);
	}
	Ok(selected.to_owned())
}
fn subset(values: &[String], allowed: &[String]) -> bool {
	values.iter().all(|v| allowed.contains(v))
}
fn valid_scope(scope: &str) -> bool {
	!scope.is_empty()
		&& scope
			.bytes()
			.all(|c| (0x21..=0x7e).contains(&c) && c != b'"' && c != b'\\')
}
fn valid_challenge(value: &str) -> bool {
	value.len() == 43
		&& value
			.bytes()
			.all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
fn valid_verifier(value: &str) -> bool {
	(43..=128).contains(&value.len())
		&& value
			.bytes()
			.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~'))
}
pub(super) fn challenge(verifier: &str) -> String {
	URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn digest(value: &str) -> String {
	hex::encode(Sha256::digest(value.as_bytes()))
}
fn random_secret() -> String {
	let mut bytes = [0u8; 32];
	rand::rng().fill_bytes(&mut bytes);
	URL_SAFE_NO_PAD.encode(bytes)
}
fn now() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs() as i64
}
async fn hash_password(value: &str) -> Result<String, OAuthError> {
	let value = value.to_owned();
	tokio::task::spawn_blocking(move || {
		Argon2::default()
			.hash_password(value.as_bytes(), &SaltString::generate(&mut OsRng))
			.map(|hash| hash.to_string())
			.map_err(|_| OAuthError::ServerError)
	})
	.await
	.map_err(|_| OAuthError::ServerError)?
}
async fn verify_password(value: &str, hash: &str) -> Result<bool, OAuthError> {
	let value = value.to_owned();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || {
		PasswordHash::new(&hash).is_ok_and(|parsed| {
			Argon2::default()
				.verify_password(value.as_bytes(), &parsed)
				.is_ok()
		})
	})
	.await
	.map_err(|_| OAuthError::ServerError)
}
