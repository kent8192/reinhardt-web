//! Registration and state storage for the authorization server.

use super::protocol::{PendingAuthorization, TokenPrincipal};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Seconds added to a Device Code's polling interval for every `slow_down`.
const SLOW_DOWN_STEP: u64 = 5;

/// Whether a client has a server-side credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClientKind {
	/// Client without a server-side secret.
	Public,
	/// Client authenticated with a server-side secret.
	Confidential,
}

/// Administrative registration of an OAuth client.
///
/// Construct with [`ClientRegistration::new`] and assign the public fields; new
/// capabilities are added as fields without breaking that pattern.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientRegistration {
	/// Unique identifier.
	pub client_id: String,
	/// Public or confidential.
	pub kind: ClientKind,
	/// Password hash for a confidential client's secret.
	pub secret_hash: Option<String>,
	/// Previous secret during a bounded administrative rotation overlap.
	#[serde(default)]
	pub previous_secret_hash: Option<String>,
	/// UNIX time when the previous secret stops authenticating.
	#[serde(default)]
	pub previous_secret_expires_at: Option<i64>,
	/// Whether this registration may use the OpenID Provider Code Flow.
	#[serde(default)]
	pub oidc_enabled: bool,
	/// Whether the code grant is enabled.
	pub authorization_code: bool,
	/// Whether the code grant also issues refresh tokens. Requires `authorization_code`,
	/// and the server must enable `OAuthServerConfig::refresh_tokens`.
	#[serde(default)]
	pub refresh_token: bool,
	/// Whether the client credentials grant is enabled.
	pub client_credentials: bool,
	/// Whether the RFC 8628 Device Authorization Grant is enabled.
	#[serde(default)]
	pub device_code: bool,
	/// Exact redirect URI allowlist.
	pub redirect_uris: Vec<String>,
	/// Scope allowlist.
	pub scopes: Vec<String>,
	/// Scopes used when scope is omitted.
	pub default_scopes: Vec<String>,
	/// Resource audience allowlist.
	pub audiences: Vec<String>,
	/// Explicit default resource audience, if any.
	pub default_audience: Option<String>,
	/// Browser origins permitted for CORS.
	pub browser_origins: Vec<String>,
	/// Disabled registrations cannot issue tokens.
	pub enabled: bool,
}

impl ClientRegistration {
	/// Create a registration with every grant disabled and every allowlist empty.
	///
	/// The registration is enabled, not OIDC-enabled, and has no secret hashes.
	/// Assign the public fields to enable grants and fill allowlists before
	/// passing it to `OAuthServer::register_client`.
	///
	/// ```
	/// use reinhardt_auth::oauth2_server::{ClientKind, ClientRegistration};
	///
	/// let mut client = ClientRegistration::new("tv-app", ClientKind::Public);
	/// client.device_code = true;
	/// client.audiences = vec!["https://api.example".into()];
	/// assert!(client.enabled && !client.authorization_code);
	/// ```
	pub fn new(client_id: impl Into<String>, kind: ClientKind) -> Self {
		Self {
			client_id: client_id.into(),
			kind,
			secret_hash: None,
			previous_secret_hash: None,
			previous_secret_expires_at: None,
			oidc_enabled: false,
			authorization_code: false,
			refresh_token: false,
			client_credentials: false,
			device_code: false,
			redirect_uris: Vec::new(),
			scopes: Vec::new(),
			default_scopes: Vec::new(),
			audiences: Vec::new(),
			default_audience: None,
			browser_origins: Vec::new(),
			enabled: true,
		}
	}
}

/// Administrative registration of a resource server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceRegistration {
	/// Unique resource identifier used as the audience.
	pub audience: String,
	/// Identifier used for introspection authentication.
	pub resource_id: String,
	/// Password hash for introspection authentication.
	pub secret_hash: String,
	/// Whether the resource server may introspect tokens.
	pub enabled: bool,
}

/// Persisted pending authorization, bound to a browser session digest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingRecord {
	/// Request data to present to the host.
	pub request: PendingAuthorization,
	/// OIDC requests may only be completed by the OIDC provider.
	#[serde(default)]
	pub oidc: bool,
	/// SHA-256 digest of the host's browser-session binding.
	pub session_digest: String,
	/// UNIX expiry time.
	pub expires_at: i64,
}

/// Persisted authorization code, stored only under a digest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredCode {
	/// SHA-256 code digest.
	pub digest: String,
	/// Owning client.
	pub client_id: String,
	/// Original redirect URI.
	pub redirect_uri: String,
	/// PKCE S256 challenge.
	pub challenge: String,
	/// Authenticated user ID.
	pub user_id: String,
	/// Approved scopes.
	pub scopes: Vec<String>,
	/// Approved audience.
	pub audience: String,
	/// Whether this code belongs to the OpenID Connect token endpoint.
	#[serde(default)]
	pub oidc: bool,
	/// UNIX expiry time.
	pub expires_at: i64,
	/// Whether a successful redemption has occurred.
	pub redeemed: bool,
	/// Whether the redeemed code was replayed.
	pub replayed: bool,
}

/// Validated authorization state to commit atomically with an optional code.
#[derive(Clone, Copy, Debug)]
pub struct AuthorizationCommit<'a> {
	/// Exact pending snapshot used for validation; rechecked under the store lock.
	pub pending: &'a PendingRecord,
	/// Approved code, or none when authorization was denied.
	pub code: Option<&'a StoredCode>,
	/// Current UNIX time for the final expiry check.
	pub now: i64,
}
impl AuthorizationCommit<'_> {
	pub(crate) fn is_valid(&self) -> bool {
		let pending = self.pending;
		let request = &pending.request;
		pending.expires_at > self.now
			&& self.code.is_none_or(|code| {
				code.client_id == request.client_id
					&& code.redirect_uri == request.redirect_uri
					&& code.challenge == request.code_challenge
					&& code.audience == request.audience
					&& code.oidc == pending.oidc
					&& !code.user_id.is_empty()
					&& !code.digest.is_empty()
					&& code
						.scopes
						.iter()
						.all(|scope| request.scopes.contains(scope))
					&& code.expires_at > self.now
					&& !code.redeemed
					&& !code.replayed
			})
	}
}

/// Result of atomic code redemption.
#[derive(Clone, Debug)]
pub enum CodeRedemption {
	/// First and only valid redemption.
	Valid(StoredCode),
	/// Code was previously redeemed; linked tokens are revoked.
	Replay,
	/// Code is absent or expired.
	Invalid,
}

/// Authenticated bindings for inspecting a code before fallible host checks.
#[derive(Clone, Copy, Debug)]
pub struct CodeInspection<'a> {
	/// SHA-256 digest of the presented authorization code.
	pub digest: &'a str,
	/// Authenticated client identifier.
	pub client_id: &'a str,
	/// Exact original redirect URI.
	pub redirect_uri: &'a str,
	/// PKCE S256 challenge computed from the supplied verifier.
	pub challenge: &'a str,
	/// Optional requested resource audience.
	pub resource: Option<&'a str>,
	/// Whether the request belongs to the OIDC token endpoint.
	pub expect_oidc: bool,
	/// Current UNIX time for rejecting an unused expired code.
	pub now: i64,
}
impl CodeInspection<'_> {
	pub(crate) fn matches(&self, code: &StoredCode) -> bool {
		code.client_id == self.client_id
			&& code.redirect_uri == self.redirect_uri
			&& code.challenge == self.challenge
			&& code.oidc == self.expect_oidc
			&& self
				.resource
				.is_none_or(|resource| resource == code.audience)
	}
}

/// Validated code bindings and the token to store in one atomic redemption.
#[derive(Debug)]
pub struct CodeRedemptionRequest<'a> {
	/// Digest of the code presented by the client.
	pub digest: &'a str,
	/// Authenticated client identifier.
	pub client_id: &'a str,
	/// Exact redirect URI used for the authorization.
	pub redirect_uri: &'a str,
	/// PKCE S256 challenge derived from the verifier.
	pub challenge: &'a str,
	/// Optional resource parameter to match against the code.
	pub resource: Option<&'a str>,
	/// Whether this is an OIDC code exchange.
	pub expect_oidc: bool,
	/// Current UNIX time used to reject expired codes.
	pub now: i64,
	/// Access token to insert only after the code is validated.
	pub token: StoredToken,
}

/// Lifecycle state of a Device Authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DeviceAuthorizationStatus {
	/// Waiting for the user to approve or deny on the Verification Page.
	Pending,
	/// Approved by a user; the Device Code has not been redeemed yet.
	Approved,
	/// Denied by the user or because the approving account was unusable.
	Denied,
	/// The Device Code was exchanged for a token.
	Redeemed,
	/// Invalidated by client disablement or a user security event.
	Invalidated,
}

/// Persisted Device Authorization, stored only under digests of its secrets.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoredDeviceAuthorization {
	/// Opaque random identifier handed to the Verification Page.
	pub id: String,
	/// SHA-256 digest of the Device Code.
	pub device_code_digest: String,
	/// SHA-256 digest of the normalized User Code.
	pub user_code_digest: String,
	/// Client that requested the authorization.
	pub client_id: String,
	/// Requested scopes.
	pub scopes: Vec<String>,
	/// Resource audience.
	pub audience: String,
	/// SHA-256 digest of the browser session that first looked up the User Code.
	pub session_digest: Option<String>,
	/// Lifecycle state.
	pub status: DeviceAuthorizationStatus,
	/// User who approved, once approved.
	pub user_id: Option<String>,
	/// Scopes approved by the user, once approved.
	pub approved_scopes: Vec<String>,
	/// Digest of the access token issued from this authorization.
	pub token_digest: Option<String>,
	/// Minimum seconds between polls; grows by five on every `slow_down`.
	pub interval: u64,
	/// UNIX time of the latest poll, or none before the first poll.
	pub last_polled_at: Option<i64>,
	/// UNIX expiry time.
	pub expires_at: i64,
}
impl StoredDeviceAuthorization {
	/// Bind an unbound pending authorization to a browser session, or confirm the binding.
	pub(crate) fn bind_session(&mut self, session_digest: &str, now: i64) -> bool {
		if self.status != DeviceAuthorizationStatus::Pending || self.expires_at <= now {
			return false;
		}
		match &self.session_digest {
			Some(bound) => bound == session_digest,
			None => {
				self.session_digest = Some(session_digest.to_owned());
				true
			}
		}
	}
	/// Apply a user decision when the record is still pending, unexpired and bound to the session.
	pub(crate) fn decide(&mut self, commit: &DeviceDecisionCommit<'_>) -> bool {
		if self.status != DeviceAuthorizationStatus::Pending
			|| self.expires_at <= commit.now
			|| self.session_digest.as_deref() != Some(commit.session_digest)
		{
			return false;
		}
		match commit.approval {
			Some(approval) => {
				if approval.user_id.is_empty()
					|| !approval
						.scopes
						.iter()
						.all(|scope| self.scopes.contains(scope))
				{
					return false;
				}
				self.status = DeviceAuthorizationStatus::Approved;
				self.user_id = Some(approval.user_id.to_owned());
				self.approved_scopes = approval.scopes.to_vec();
			}
			None => self.status = DeviceAuthorizationStatus::Denied,
		}
		true
	}
	/// Evaluate one poll. A redeemed record reports a replay before throttling.
	/// Otherwise expiry precedes `slow_down`, which precedes the decision state: an
	/// expired session must conclude with `expired_token` (RFC 8628 section 3.5),
	/// so expired polls neither throttle nor update the polling state.
	pub(crate) fn poll(&mut self, client_id: &str, now: i64) -> DevicePoll {
		if self.client_id != client_id {
			return DevicePoll::Invalid;
		}
		if self.status == DeviceAuthorizationStatus::Redeemed {
			return DevicePoll::Replay;
		}
		if self.expires_at <= now {
			return DevicePoll::Expired;
		}
		let too_fast = self
			.last_polled_at
			.is_some_and(|last| now.saturating_sub(last) < self.interval as i64);
		self.last_polled_at = Some(now);
		if too_fast {
			self.interval = self.interval.saturating_add(SLOW_DOWN_STEP);
			return DevicePoll::SlowDown;
		}
		match self.status {
			DeviceAuthorizationStatus::Pending => DevicePoll::Pending,
			DeviceAuthorizationStatus::Denied => DevicePoll::Denied,
			DeviceAuthorizationStatus::Approved => DevicePoll::Approved(Box::new(self.clone())),
			DeviceAuthorizationStatus::Redeemed | DeviceAuthorizationStatus::Invalidated => {
				DevicePoll::Invalid
			}
		}
	}
	/// Redeem an approved authorization with its token, or report why that is impossible.
	pub(crate) fn redeem(&mut self, request: &DeviceRedemptionRequest<'_>) -> DeviceRedemption {
		if self.client_id != request.client_id {
			return DeviceRedemption::Invalid;
		}
		if self.status == DeviceAuthorizationStatus::Redeemed {
			return DeviceRedemption::Replay;
		}
		if self.status != DeviceAuthorizationStatus::Approved
			|| self.expires_at <= request.now
			|| !token_matches_device_authorization(&request.token, self)
		{
			return DeviceRedemption::Invalid;
		}
		self.status = DeviceAuthorizationStatus::Redeemed;
		self.token_digest = Some(request.token.digest.clone());
		DeviceRedemption::Valid(Box::new(self.clone()))
	}
	/// Invalidate a record that has not been redeemed; returns whether it changed.
	pub(crate) fn invalidate(&mut self) -> bool {
		if matches!(
			self.status,
			DeviceAuthorizationStatus::Redeemed | DeviceAuthorizationStatus::Invalidated
		) {
			return false;
		}
		self.status = DeviceAuthorizationStatus::Invalidated;
		true
	}
}

/// Approval attached to a [`DeviceDecisionCommit`].
#[derive(Clone, Copy, Debug)]
pub struct DeviceApproval<'a> {
	/// Authenticated user who approved.
	pub user_id: &'a str,
	/// Approved scopes; must be a subset of the requested scopes.
	pub scopes: &'a [String],
}

/// User decision to commit atomically to a Device Authorization.
#[derive(Clone, Copy, Debug)]
pub struct DeviceDecisionCommit<'a> {
	/// Identifier of the Device Authorization.
	pub id: &'a str,
	/// Digest of the browser session that must be bound to the record.
	pub session_digest: &'a str,
	/// Current UNIX time for the expiry check.
	pub now: i64,
	/// Approval details, or none to deny.
	pub approval: Option<DeviceApproval<'a>>,
}

/// Outcome of one atomically evaluated poll of a Device Code.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DevicePoll {
	/// Polled before the current interval elapsed; the interval grew by five seconds.
	SlowDown,
	/// The Device Authorization expired.
	Expired,
	/// The user has not decided yet.
	Pending,
	/// The user denied the request.
	Denied,
	/// Approved and unredeemed; carries a snapshot for host checks before redemption.
	Approved(Box<StoredDeviceAuthorization>),
	/// The Device Code was already redeemed; the token issued from it is revoked.
	Replay,
	/// Unknown, invalidated, or belonging to another client.
	Invalid,
}

/// Validated Device Code bindings and the token to store in one atomic redemption.
#[derive(Debug)]
pub struct DeviceRedemptionRequest<'a> {
	/// Digest of the Device Code presented by the client.
	pub device_code_digest: &'a str,
	/// Authenticated client identifier.
	pub client_id: &'a str,
	/// Current UNIX time used to reject an expired authorization.
	pub now: i64,
	/// Access token to insert only after the authorization is validated.
	pub token: StoredToken,
}

/// Result of atomic Device Authorization redemption.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DeviceRedemption {
	/// First and only valid redemption.
	Valid(Box<StoredDeviceAuthorization>),
	/// Already redeemed; the previously issued token is revoked.
	Replay,
	/// Absent, expired, no longer approved, or mismatched.
	Invalid,
}

/// Persisted opaque token metadata, stored only under a digest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredToken {
	/// SHA-256 token digest.
	pub digest: String,
	/// Owning client.
	pub client_id: String,
	/// User or client principal.
	pub principal: TokenPrincipal,
	/// Granted scopes.
	pub scopes: Vec<String>,
	/// Single resource audience.
	pub audience: String,
	/// UNIX issuance time.
	pub issued_at: i64,
	/// UNIX expiry time.
	pub expires_at: i64,
	/// Revocation state.
	pub revoked: bool,
	/// Authorization-code digest if delegated.
	pub code_digest: Option<String>,
	/// Token family that issued this access token, if it came from a refresh-enabled grant.
	#[serde(default)]
	pub family_id: Option<String>,
}

/// One token family: the refresh-token chain started by a single code redemption
/// and every access token issued from it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoredTokenFamily {
	/// Random family identifier.
	pub family_id: String,
	/// Client the family was issued to.
	pub client_id: String,
	/// Delegating user.
	pub user_id: String,
	/// Scopes of the original grant; refreshes may narrow but never widen them.
	pub scopes: Vec<String>,
	/// Single resource audience of the original grant.
	pub audience: String,
	/// Digest of the authorization code that started the family.
	pub code_digest: String,
	/// UNIX creation time.
	pub created_at: i64,
	/// UNIX absolute expiry; fixed at creation and never extended by rotation.
	pub absolute_expires_at: i64,
	/// Whether the whole family has been revoked.
	pub revoked: bool,
}

/// Persisted refresh token, stored only under a digest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoredRefreshToken {
	/// SHA-256 refresh-token digest.
	pub digest: String,
	/// Owning token family.
	pub family_id: String,
	/// Digest of the refresh token this one replaced, if any.
	pub parent_digest: Option<String>,
	/// UNIX issuance time.
	pub issued_at: i64,
	/// UNIX idle expiry; never later than the family's absolute expiry.
	pub idle_expires_at: i64,
	/// Whether this token has already been exchanged; presenting it again is reuse.
	pub rotated: bool,
}

/// Atomic code redemption that also starts a token family.
#[derive(Debug)]
pub struct CodeRedemptionWithRefresh<'a> {
	/// Code bindings and access token; `token.family_id` names `family`.
	pub redemption: CodeRedemptionRequest<'a>,
	/// New family, keyed to the redeemed code digest.
	pub family: StoredTokenFamily,
	/// First refresh token of the family.
	pub refresh_token: StoredRefreshToken,
}

/// Result of inspecting a presented refresh token.
// Allowed: the value is returned once per refresh and destructured immediately, so
// boxing the active records would only add a heap allocation to every refresh.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum RefreshInspection {
	/// Unrotated, unexpired token of a live family owned by the client.
	Active(StoredRefreshToken, StoredTokenFamily),
	/// An already-rotated token of a live family owned by the client was presented;
	/// the store revoked the family and its access tokens atomically.
	Reused {
		/// Revoked family identifier, for audit logging.
		family_id: String,
	},
	/// Missing, expired, revoked, or owned by another client.
	Invalid,
}

/// Atomic rotation of an inspected refresh token.
#[derive(Debug)]
pub struct RefreshRotationRequest<'a> {
	/// Digest of the presented refresh token.
	pub presented_digest: &'a str,
	/// Authenticated client identifier.
	pub client_id: &'a str,
	/// Current UNIX time, rechecked against idle and absolute expiry under lock.
	pub now: i64,
	/// Replacement refresh token; its family and parent must match the presented token.
	pub replacement: StoredRefreshToken,
	/// Access token issued by this refresh; `family_id` names the same family.
	pub token: StoredToken,
}

/// Result of atomic refresh-token rotation.
#[derive(Clone, Debug)]
pub enum RefreshRotation {
	/// Presented token marked rotated; replacement and access token stored.
	Rotated,
	/// Presented token was already rotated (e.g. a concurrent refresh won);
	/// the family was revoked atomically.
	Reused {
		/// Revoked family identifier, for audit logging.
		family_id: String,
	},
	/// Presented token became invalid; nothing was written.
	Invalid,
}

/// Atomic storage contract for a multi-instance OAuth server.
#[async_trait]
pub trait OAuthServerStore: Send + Sync {
	/// Save a validated registration.
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String>;
	/// Insert a registration only if its identifier is absent, including across instances.
	/// Stores used for administrative registration must implement this atomic operation.
	async fn insert_client_if_absent(&self, _client: ClientRegistration) -> Result<bool, String> {
		Err("store does not support atomic client registration".to_owned())
	}
	/// Replace a registration only if its complete current value matches the snapshot.
	/// A concurrent administrative change returns false without writing.
	async fn compare_and_swap_client(
		&self,
		expected: &ClientRegistration,
		replacement: ClientRegistration,
	) -> Result<bool, String>;
	/// Atomically disable a client and invalidate its pending requests, codes, Device
	/// Authorizations, tokens, and token families. Returns the number of newly revoked
	/// access tokens, or none for a missing registration.
	async fn disable_client(&self, client_id: &str) -> Result<Option<u64>, String>;
	/// Get a client registration.
	async fn client(&self, client_id: &str) -> Result<Option<ClientRegistration>, String>;
	/// Find an enabled public client that registered a browser origin.
	async fn public_client_for_origin(
		&self,
		origin: &str,
	) -> Result<Option<ClientRegistration>, String>;
	/// Save a resource server registration.
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String>;
	/// Insert a resource only if its identifier and audience are unclaimed.
	/// Stores used for administrative registration must implement this atomic operation.
	async fn insert_resource_if_absent(
		&self,
		_resource: ResourceRegistration,
	) -> Result<bool, String> {
		Err("store does not support atomic resource registration".to_owned())
	}
	/// Get a resource server registration.
	async fn resource(&self, resource_id: &str) -> Result<Option<ResourceRegistration>, String>;
	/// Compare the complete resource snapshot and atomically replace its registration.
	/// The resource identifier and audience must remain unchanged. Returns false on conflict.
	async fn compare_and_swap_resource(
		&self,
		expected: &ResourceRegistration,
		replacement: ResourceRegistration,
	) -> Result<bool, String>;

	/// Find a registered resource by its unique audience.
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String>;
	/// Save a pending request with a random identifier.
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String>;
	/// Read a pending snapshot without consuming it before fallible host validation.
	async fn pending(&self, id: &str) -> Result<Option<PendingRecord>, String>;
	/// Atomically consume the matching pending snapshot and insert its approved code.
	/// Returns false for a stale or expired snapshot; errors leave both unchanged.
	async fn complete_pending(&self, request: AuthorizationCommit<'_>) -> Result<bool, String>;
	/// In-memory coordination hook. Invoke the infallible callback synchronously
	/// under the commit lock, with no intervening await after either state changes.
	/// Stores without this capability must fail without invoking the callback.
	#[doc(hidden)]
	async fn complete_pending_in_memory(
		&self,
		_request: AuthorizationCommit<'_>,
		_on_commit: &mut (dyn FnMut() + Send),
	) -> Result<bool, String> {
		Err("store does not support in-memory authorization coordination".to_owned())
	}
	/// PostgreSQL coordination hook. All writes belong to the supplied transaction;
	/// the caller owns commit or rollback. P0: unavailable without database support.
	#[doc(hidden)]
	#[cfg(feature = "database")]
	async fn complete_pending_in_transaction(
		&self,
		_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
		_request: AuthorizationCommit<'_>,
	) -> Result<bool, String> {
		Err("store does not support PostgreSQL authorization coordination".to_owned())
	}
	/// Atomically consume an unexpired pending request for its browser session and flow.
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		oidc: bool,
		now: i64,
	) -> Result<Option<PendingRecord>, String>;
	/// Save an authorization code.
	async fn put_code(&self, code: StoredCode) -> Result<(), String>;
	/// Read code metadata before constructing a token; redemption rechecks it under lock.
	async fn code(&self, digest: &str) -> Result<Option<StoredCode>, String>;
	/// Inspect bindings under the code lock without consuming an unused code.
	/// A correctly bound replay atomically revokes linked tokens and the token family
	/// started by the code, even if expired.
	/// Returns none for missing, mismatched, expired, or replayed codes.
	async fn inspect_code_for_exchange(
		&self,
		request: CodeInspection<'_>,
	) -> Result<Option<StoredCode>, String>;
	/// Atomically redeem a bound code and save its token, rolling both back on failure.
	/// A replay also revokes the token family started by the code.
	async fn redeem_code_and_store_token(
		&self,
		request: CodeRedemptionRequest<'_>,
	) -> Result<CodeRedemption, String>;
	/// Save a client-credentials token. Code-linked tokens use atomic redemption.
	async fn put_token(&self, token: StoredToken) -> Result<(), String>;
	/// Look up opaque token metadata.
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String>;
	/// Revoke a token belonging to the specified client.
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String>;
	/// Invalidate outstanding user codes and approved Device Authorizations and revoke
	/// tokens atomically, including the user's token families. Returns the newly revoked
	/// access-token count.
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String>;
	/// Invalidate a retired user's codes, approved Device Authorizations, tokens, and token
	/// families together.
	async fn retire_user(&self, user_id: &str) -> Result<(), String>;
	/// Revoke all client tokens and token families (administrative operation).
	/// Returns the newly revoked access-token count.
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String>;
	/// Atomically redeem a bound code, store its access token, and start a token family.
	/// Replay and invalid outcomes match `redeem_code_and_store_token` and write nothing new.
	/// Stores without refresh-token support must keep this default error.
	async fn redeem_code_and_store_tokens(
		&self,
		_request: CodeRedemptionWithRefresh<'_>,
	) -> Result<CodeRedemption, String> {
		Err("store does not support refresh tokens".to_owned())
	}
	/// Inspect a presented refresh token for the authenticated client without rotating it.
	/// Presenting an already-rotated token of a live family owned by the client revokes
	/// the family and its access tokens atomically and returns `Reused`.
	async fn inspect_refresh_token(
		&self,
		_digest: &str,
		_client_id: &str,
		_now: i64,
	) -> Result<RefreshInspection, String> {
		Err("store does not support refresh tokens".to_owned())
	}
	/// Atomically mark the presented token rotated and store its replacement and access token.
	/// Exactly one concurrent rotation of the same token returns `Rotated`; the others
	/// observe the rotated token, revoke the family, and return `Reused`.
	async fn rotate_refresh_token(
		&self,
		_request: RefreshRotationRequest<'_>,
	) -> Result<RefreshRotation, String> {
		Err("store does not support refresh tokens".to_owned())
	}
	/// Revoke the family of a refresh token (rotated or not) owned by the client,
	/// together with its access tokens. Returns whether a matching family was found.
	/// Stores that cannot create families hold none, so the default finds nothing.
	async fn revoke_refresh_family(&self, _digest: &str, _client_id: &str) -> Result<bool, String> {
		Ok(false)
	}
	/// Insert a Device Authorization unless its User Code digest collides with a live
	/// record (or its identifiers are already taken). Returns false on collision so the
	/// caller can generate a fresh User Code. Expired records do not block reuse.
	async fn insert_device_authorization(
		&self,
		_record: StoredDeviceAuthorization,
		_now: i64,
	) -> Result<bool, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Read a Device Authorization by Device Code digest without changing it.
	async fn device_authorization(
		&self,
		_device_code_digest: &str,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Read a Device Authorization by its opaque identifier without changing it.
	async fn device_authorization_by_id(
		&self,
		_id: &str,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Atomically bind a live pending record to a browser session by User Code digest.
	/// Returns the record when it is pending, unexpired, and unbound or bound to the same
	/// session; otherwise none and the record is untouched.
	async fn bind_device_user_code(
		&self,
		_user_code_digest: &str,
		_session_digest: &str,
		_now: i64,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Atomically commit a user decision to a pending, unexpired record bound to the
	/// session. Returns false for a stale, unbound, or mismatched record.
	async fn decide_device_authorization(
		&self,
		_commit: DeviceDecisionCommit<'_>,
	) -> Result<bool, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Evaluate one poll atomically: throttle, expiry, decision state, and replay
	/// detection. A replay also revokes the token issued from the Device Code.
	async fn poll_device_authorization(
		&self,
		_device_code_digest: &str,
		_client_id: &str,
		_now: i64,
	) -> Result<DevicePoll, String> {
		Err("store does not support device authorization".to_owned())
	}
	/// Atomically redeem an approved Device Authorization and save its token, rolling
	/// both back on failure. A replay revokes the previously issued token.
	async fn redeem_device_authorization(
		&self,
		_request: DeviceRedemptionRequest<'_>,
	) -> Result<DeviceRedemption, String> {
		Err("store does not support device authorization".to_owned())
	}
}

/// Development and test store. Production requires PostgreSQL.
#[derive(Default)]
pub struct MemoryOAuthStore {
	state: Mutex<MemoryState>,
}
#[derive(Default)]
struct MemoryState {
	clients: HashMap<String, ClientRegistration>,
	resources: HashMap<String, ResourceRegistration>,
	pending: HashMap<String, PendingRecord>,
	codes: HashMap<String, StoredCode>,
	tokens: HashMap<String, StoredToken>,
	families: HashMap<String, StoredTokenFamily>,
	refresh_tokens: HashMap<String, StoredRefreshToken>,
	device_authorizations: HashMap<String, StoredDeviceAuthorization>,
}
impl MemoryState {
	/// Revoke a family and every access token issued from it.
	fn revoke_family(&mut self, family_id: &str) {
		if let Some(family) = self.families.get_mut(family_id) {
			family.revoked = true;
		}
		for token in self
			.tokens
			.values_mut()
			.filter(|token| token.family_id.as_deref() == Some(family_id))
		{
			token.revoked = true;
		}
	}
	/// Revoke every family started by the code, together with its access tokens.
	fn revoke_families_of_code(&mut self, code_digest: &str) {
		let ids: Vec<String> = self
			.families
			.values()
			.filter(|family| family.code_digest == code_digest)
			.map(|family| family.family_id.clone())
			.collect();
		for id in ids {
			self.revoke_family(&id);
		}
	}
	/// Mark matching families revoked without touching access tokens, which the caller
	/// revokes (and counts) through its own principal or client filter.
	fn mark_families_revoked(&mut self, matches: impl Fn(&StoredTokenFamily) -> bool) {
		for family in self.families.values_mut().filter(|family| matches(family)) {
			family.revoked = true;
		}
	}
	/// Resolve a presented refresh token to its family and apply the shared state rules.
	/// Reuse of a rotated token revokes the family before reporting.
	fn check_refresh_token(
		&mut self,
		digest: &str,
		client_id: &str,
		now: i64,
	) -> Result<RefreshInspection, String> {
		let Some(token) = self.refresh_tokens.get(digest).cloned() else {
			return Ok(RefreshInspection::Invalid);
		};
		let Some(family) = self.families.get(&token.family_id).cloned() else {
			return Ok(RefreshInspection::Invalid);
		};
		if family.revoked || family.absolute_expires_at <= now || family.client_id != client_id {
			return Ok(RefreshInspection::Invalid);
		}
		if token.rotated {
			self.revoke_family(&family.family_id);
			return Ok(RefreshInspection::Reused {
				family_id: family.family_id,
			});
		}
		if token.idle_expires_at <= now {
			return Ok(RefreshInspection::Invalid);
		}
		Ok(RefreshInspection::Active(token, family))
	}
	fn redeem(
		&mut self,
		request: CodeRedemptionRequest<'_>,
		family: Option<(StoredTokenFamily, StoredRefreshToken)>,
	) -> Result<CodeRedemption, String> {
		let CodeRedemptionRequest {
			digest,
			client_id,
			redirect_uri,
			challenge,
			resource,
			expect_oidc,
			now,
			token,
		} = request;
		let token_collision = self.tokens.contains_key(&token.digest);
		let Some(code) = self.codes.get_mut(digest) else {
			return Ok(CodeRedemption::Invalid);
		};
		if code.client_id != client_id
			|| code.redirect_uri != redirect_uri
			|| code.challenge != challenge
			|| resource.is_some_and(|r| r != code.audience)
			|| code.oidc != expect_oidc
		{
			return Ok(CodeRedemption::Invalid);
		}
		if code.redeemed {
			code.replayed = true;
			for linked in self
				.tokens
				.values_mut()
				.filter(|t| t.code_digest.as_deref() == Some(digest))
			{
				linked.revoked = true;
			}
			self.revoke_families_of_code(digest);
			return Ok(CodeRedemption::Replay);
		}
		if code.expires_at <= now || !token_matches_code(&token, code) {
			return Ok(CodeRedemption::Invalid);
		}
		if token_collision {
			return Err("token digest collision".to_owned());
		}
		if let Some((family, refresh)) = &family {
			if !family_matches_code(family, refresh, &token, code) {
				return Err("token family does not match the redeemed code".to_owned());
			}
			if self.families.contains_key(&family.family_id)
				|| self.refresh_tokens.contains_key(&refresh.digest)
			{
				return Err("token family digest collision".to_owned());
			}
		}
		code.redeemed = true;
		let redeemed = code.clone();
		if let Some((family, refresh)) = family {
			self.families.insert(family.family_id.clone(), family);
			self.refresh_tokens.insert(refresh.digest.clone(), refresh);
		}
		self.tokens.insert(token.digest.clone(), token);
		Ok(CodeRedemption::Valid(redeemed))
	}
}
impl MemoryOAuthStore {
	/// Create an empty in-memory store.
	pub fn new() -> Self {
		Self::default()
	}
}
#[async_trait]
impl OAuthServerStore for MemoryOAuthStore {
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String> {
		self.state
			.lock()
			.await
			.clients
			.insert(client.client_id.clone(), client);
		Ok(())
	}
	async fn insert_client_if_absent(&self, client: ClientRegistration) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		if state.clients.contains_key(&client.client_id) {
			return Ok(false);
		}
		state.clients.insert(client.client_id.clone(), client);
		Ok(true)
	}
	async fn compare_and_swap_client(
		&self,
		expected: &ClientRegistration,
		replacement: ClientRegistration,
	) -> Result<bool, String> {
		if replacement.client_id != expected.client_id {
			return Err("client identity cannot change".to_owned());
		}
		let mut state = self.state.lock().await;
		if state.clients.get(&expected.client_id) != Some(expected) {
			return Ok(false);
		}
		state
			.clients
			.insert(replacement.client_id.clone(), replacement);
		Ok(true)
	}
	async fn disable_client(&self, id: &str) -> Result<Option<u64>, String> {
		let mut state = self.state.lock().await;
		let Some(client) = state.clients.get_mut(id) else {
			return Ok(None);
		};
		client.enabled = false;
		state
			.pending
			.retain(|_, pending| pending.request.client_id != id);
		for code in state.codes.values_mut().filter(|code| code.client_id == id) {
			code.redeemed = true;
			code.replayed = true;
		}
		for record in state
			.device_authorizations
			.values_mut()
			.filter(|record| record.client_id == id)
		{
			record.invalidate();
		}
		let mut count = 0;
		for token in state
			.tokens
			.values_mut()
			.filter(|token| token.client_id == id && !token.revoked)
		{
			token.revoked = true;
			count += 1;
		}
		state.mark_families_revoked(|family| family.client_id == id);
		Ok(Some(count))
	}
	async fn client(&self, id: &str) -> Result<Option<ClientRegistration>, String> {
		Ok(self.state.lock().await.clients.get(id).cloned())
	}
	async fn public_client_for_origin(
		&self,
		origin: &str,
	) -> Result<Option<ClientRegistration>, String> {
		Ok(self
			.state
			.lock()
			.await
			.clients
			.values()
			.find(|c| {
				c.enabled
					&& c.kind == ClientKind::Public
					&& c.browser_origins.iter().any(|o| o == origin)
			})
			.cloned())
	}
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String> {
		let mut state = self.state.lock().await;
		if state.resources.values().any(|existing| {
			existing.audience == resource.audience && existing.resource_id != resource.resource_id
		}) {
			return Err("audience is already registered".to_owned());
		}
		state
			.resources
			.insert(resource.resource_id.clone(), resource);
		Ok(())
	}
	async fn insert_resource_if_absent(
		&self,
		resource: ResourceRegistration,
	) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		if state.resources.contains_key(&resource.resource_id)
			|| state
				.resources
				.values()
				.any(|r| r.audience == resource.audience)
		{
			return Ok(false);
		}
		state
			.resources
			.insert(resource.resource_id.clone(), resource);
		Ok(true)
	}
	async fn compare_and_swap_resource(
		&self,
		expected: &ResourceRegistration,
		replacement: ResourceRegistration,
	) -> Result<bool, String> {
		if replacement.resource_id != expected.resource_id
			|| replacement.audience != expected.audience
		{
			return Err("resource identity cannot change".to_owned());
		}
		let mut state = self.state.lock().await;
		if state.resources.get(&expected.resource_id) != Some(expected) {
			return Ok(false);
		}
		state
			.resources
			.insert(replacement.resource_id.clone(), replacement);
		Ok(true)
	}
	async fn resource(&self, id: &str) -> Result<Option<ResourceRegistration>, String> {
		Ok(self.state.lock().await.resources.get(id).cloned())
	}
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String> {
		Ok(self
			.state
			.lock()
			.await
			.resources
			.values()
			.find(|r| r.audience == audience)
			.cloned())
	}
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String> {
		self.state
			.lock()
			.await
			.pending
			.insert(id.to_owned(), pending);
		Ok(())
	}
	async fn pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		Ok(self.state.lock().await.pending.get(id).cloned())
	}
	async fn complete_pending(&self, request: AuthorizationCommit<'_>) -> Result<bool, String> {
		self.complete_pending_in_memory(request, &mut || {}).await
	}
	async fn complete_pending_in_memory(
		&self,
		request: AuthorizationCommit<'_>,
		on_commit: &mut (dyn FnMut() + Send),
	) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		let id = &request.pending.request.id;
		if state.pending.get(id) != Some(request.pending) || !request.is_valid() {
			return Ok(false);
		}
		if let Some(code) = request.code {
			if state.codes.contains_key(&code.digest) {
				return Err("code digest collision".to_owned());
			}
			state.codes.insert(code.digest.clone(), code.clone());
		}
		state.pending.remove(id);
		// No await or fallible operation between the two stores' mutations.
		on_commit();
		Ok(true)
	}
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		oidc: bool,
		now: i64,
	) -> Result<Option<PendingRecord>, String> {
		let mut state = self.state.lock().await;
		if state.pending.get(id).is_none_or(|pending| {
			pending.session_digest != session_digest
				|| pending.oidc != oidc
				|| pending.expires_at <= now
		}) {
			return Ok(None);
		}
		Ok(state.pending.remove(id))
	}
	async fn put_code(&self, code: StoredCode) -> Result<(), String> {
		self.state
			.lock()
			.await
			.codes
			.insert(code.digest.clone(), code);
		Ok(())
	}
	async fn code(&self, digest: &str) -> Result<Option<StoredCode>, String> {
		Ok(self.state.lock().await.codes.get(digest).cloned())
	}
	async fn inspect_code_for_exchange(
		&self,
		request: CodeInspection<'_>,
	) -> Result<Option<StoredCode>, String> {
		let mut state = self.state.lock().await;
		let Some(code) = state.codes.get_mut(request.digest) else {
			return Ok(None);
		};
		if !request.matches(code) {
			return Ok(None);
		}
		if code.redeemed {
			code.replayed = true;
			for token in state
				.tokens
				.values_mut()
				.filter(|token| token.code_digest.as_deref() == Some(request.digest))
			{
				token.revoked = true;
			}
			state.revoke_families_of_code(request.digest);
			return Ok(None);
		}
		Ok((code.expires_at > request.now).then(|| code.clone()))
	}
	async fn redeem_code_and_store_token(
		&self,
		request: CodeRedemptionRequest<'_>,
	) -> Result<CodeRedemption, String> {
		self.state.lock().await.redeem(request, None)
	}
	async fn redeem_code_and_store_tokens(
		&self,
		request: CodeRedemptionWithRefresh<'_>,
	) -> Result<CodeRedemption, String> {
		let CodeRedemptionWithRefresh {
			redemption,
			family,
			refresh_token,
		} = request;
		self.state
			.lock()
			.await
			.redeem(redemption, Some((family, refresh_token)))
	}
	async fn put_token(&self, token: StoredToken) -> Result<(), String> {
		if token.code_digest.is_some() {
			return Err("code-linked tokens require atomic redemption".to_owned());
		}
		let mut state = self.state.lock().await;
		state.tokens.insert(token.digest.clone(), token);
		Ok(())
	}
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String> {
		Ok(self.state.lock().await.tokens.get(digest).cloned())
	}
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String> {
		if let Some(token) = self.state.lock().await.tokens.get_mut(digest)
			&& token.client_id == client_id
		{
			token.revoked = true;
		}
		Ok(())
	}
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String> {
		let mut state = self.state.lock().await;
		invalidate_user_device_authorizations(&mut state, user_id);
		for code in state
			.codes
			.values_mut()
			.filter(|code| code.user_id == user_id)
		{
			code.redeemed = true;
			code.replayed = true;
		}
		let mut count = 0;
		for token in state.tokens.values_mut() {
			if token.principal == TokenPrincipal::User(user_id.to_owned()) && !token.revoked {
				token.revoked = true;
				count += 1;
			}
		}
		state.mark_families_revoked(|family| family.user_id == user_id);
		Ok(count)
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		let mut state = self.state.lock().await;
		invalidate_user_device_authorizations(&mut state, user_id);
		for code in state
			.codes
			.values_mut()
			.filter(|code| code.user_id == user_id)
		{
			code.redeemed = true;
			code.replayed = true;
		}
		for token in state
			.tokens
			.values_mut()
			.filter(|token| token.principal == TokenPrincipal::User(user_id.to_owned()))
		{
			token.revoked = true;
		}
		state.mark_families_revoked(|family| family.user_id == user_id);
		Ok(())
	}
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String> {
		let mut state = self.state.lock().await;
		let mut count = 0;
		for token in state.tokens.values_mut() {
			if token.client_id == client_id && !token.revoked {
				token.revoked = true;
				count += 1;
			}
		}
		state.mark_families_revoked(|family| family.client_id == client_id);
		Ok(count)
	}
	async fn inspect_refresh_token(
		&self,
		digest: &str,
		client_id: &str,
		now: i64,
	) -> Result<RefreshInspection, String> {
		self.state
			.lock()
			.await
			.check_refresh_token(digest, client_id, now)
	}
	async fn rotate_refresh_token(
		&self,
		request: RefreshRotationRequest<'_>,
	) -> Result<RefreshRotation, String> {
		let RefreshRotationRequest {
			presented_digest,
			client_id,
			now,
			replacement,
			token,
		} = request;
		let mut state = self.state.lock().await;
		let Some(presented) = state.refresh_tokens.get(presented_digest) else {
			return Ok(RefreshRotation::Invalid);
		};
		if replacement.family_id != presented.family_id
			|| token.family_id.as_deref() != Some(presented.family_id.as_str())
			|| replacement.parent_digest.as_deref() != Some(presented_digest)
			|| token.client_id != client_id
			|| token.revoked
			|| replacement.rotated
		{
			return Err("replacement tokens do not match the presented refresh token".to_owned());
		}
		match state.check_refresh_token(presented_digest, client_id, now)? {
			RefreshInspection::Active(..) => {}
			RefreshInspection::Reused { family_id } => {
				return Ok(RefreshRotation::Reused { family_id });
			}
			RefreshInspection::Invalid => return Ok(RefreshRotation::Invalid),
		}
		if state.refresh_tokens.contains_key(&replacement.digest)
			|| state.tokens.contains_key(&token.digest)
		{
			return Err("token digest collision".to_owned());
		}
		if let Some(presented) = state.refresh_tokens.get_mut(presented_digest) {
			presented.rotated = true;
		}
		state
			.refresh_tokens
			.insert(replacement.digest.clone(), replacement);
		state.tokens.insert(token.digest.clone(), token);
		Ok(RefreshRotation::Rotated)
	}
	async fn revoke_refresh_family(&self, digest: &str, client_id: &str) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		let Some(family_id) = state
			.refresh_tokens
			.get(digest)
			.map(|token| token.family_id.clone())
		else {
			return Ok(false);
		};
		if state
			.families
			.get(&family_id)
			.is_none_or(|family| family.client_id != client_id)
		{
			return Ok(false);
		}
		state.revoke_family(&family_id);
		Ok(true)
	}
	async fn insert_device_authorization(
		&self,
		record: StoredDeviceAuthorization,
		now: i64,
	) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		state
			.device_authorizations
			.retain(|_, existing| existing.expires_at > now);
		if state.device_authorizations.values().any(|existing| {
			existing.user_code_digest == record.user_code_digest || existing.id == record.id
		}) || state
			.device_authorizations
			.contains_key(&record.device_code_digest)
		{
			return Ok(false);
		}
		state
			.device_authorizations
			.insert(record.device_code_digest.clone(), record);
		Ok(true)
	}
	async fn device_authorization(
		&self,
		device_code_digest: &str,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		Ok(self
			.state
			.lock()
			.await
			.device_authorizations
			.get(device_code_digest)
			.cloned())
	}
	async fn device_authorization_by_id(
		&self,
		id: &str,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		Ok(self
			.state
			.lock()
			.await
			.device_authorizations
			.values()
			.find(|record| record.id == id)
			.cloned())
	}
	async fn bind_device_user_code(
		&self,
		user_code_digest: &str,
		session_digest: &str,
		now: i64,
	) -> Result<Option<StoredDeviceAuthorization>, String> {
		let mut state = self.state.lock().await;
		let Some(record) = state
			.device_authorizations
			.values_mut()
			.find(|record| record.user_code_digest == user_code_digest)
		else {
			return Ok(None);
		};
		Ok(record
			.bind_session(session_digest, now)
			.then(|| record.clone()))
	}
	async fn decide_device_authorization(
		&self,
		commit: DeviceDecisionCommit<'_>,
	) -> Result<bool, String> {
		let mut state = self.state.lock().await;
		Ok(state
			.device_authorizations
			.values_mut()
			.find(|record| record.id == commit.id)
			.is_some_and(|record| record.decide(&commit)))
	}
	async fn poll_device_authorization(
		&self,
		device_code_digest: &str,
		client_id: &str,
		now: i64,
	) -> Result<DevicePoll, String> {
		let mut guard = self.state.lock().await;
		let state = &mut *guard;
		let Some(record) = state.device_authorizations.get_mut(device_code_digest) else {
			return Ok(DevicePoll::Invalid);
		};
		let outcome = record.poll(client_id, now);
		if matches!(outcome, DevicePoll::Replay)
			&& let Some(token_digest) = record.token_digest.clone()
			&& let Some(token) = state.tokens.get_mut(&token_digest)
		{
			token.revoked = true;
		}
		Ok(outcome)
	}
	async fn redeem_device_authorization(
		&self,
		request: DeviceRedemptionRequest<'_>,
	) -> Result<DeviceRedemption, String> {
		let mut guard = self.state.lock().await;
		let state = &mut *guard;
		let token_collision = state.tokens.contains_key(&request.token.digest);
		let Some(record) = state
			.device_authorizations
			.get_mut(request.device_code_digest)
		else {
			return Ok(DeviceRedemption::Invalid);
		};
		let status_before = record.status;
		if status_before == DeviceAuthorizationStatus::Approved && token_collision {
			return Err("token digest collision".to_owned());
		}
		let outcome = record.redeem(&request);
		match &outcome {
			DeviceRedemption::Valid(_) => {
				state
					.tokens
					.insert(request.token.digest.clone(), request.token);
			}
			DeviceRedemption::Replay => {
				if let Some(token_digest) = record.token_digest.clone()
					&& let Some(token) = state.tokens.get_mut(&token_digest)
				{
					token.revoked = true;
				}
			}
			DeviceRedemption::Invalid => {}
		}
		Ok(outcome)
	}
}

/// Whether a family, its first refresh token, and the first access token describe
/// exactly the grant carried by the redeemed code.
fn family_matches_code(
	family: &StoredTokenFamily,
	refresh: &StoredRefreshToken,
	token: &StoredToken,
	code: &StoredCode,
) -> bool {
	!family.revoked
		&& family.code_digest == code.digest
		&& family.client_id == code.client_id
		&& family.user_id == code.user_id
		&& family.scopes == code.scopes
		&& family.audience == code.audience
		&& token.family_id.as_deref() == Some(family.family_id.as_str())
		&& refresh.family_id == family.family_id
		&& refresh.parent_digest.is_none()
		&& !refresh.rotated
}

fn invalidate_user_device_authorizations(state: &mut MemoryState, user_id: &str) {
	for record in state
		.device_authorizations
		.values_mut()
		.filter(|record| record.user_id.as_deref() == Some(user_id))
	{
		if record.status == DeviceAuthorizationStatus::Approved {
			record.invalidate();
		}
	}
}

pub(crate) fn token_matches_code(token: &StoredToken, code: &StoredCode) -> bool {
	token.client_id == code.client_id
		&& token.principal == TokenPrincipal::User(code.user_id.clone())
		&& token.scopes == code.scopes
		&& token.audience == code.audience
		&& token.code_digest.as_deref() == Some(code.digest.as_str())
		&& !token.revoked
}

pub(crate) fn token_matches_device_authorization(
	token: &StoredToken,
	record: &StoredDeviceAuthorization,
) -> bool {
	token.client_id == record.client_id
		&& record
			.user_id
			.as_ref()
			.is_some_and(|user_id| token.principal == TokenPrincipal::User(user_id.clone()))
		&& token.scopes == record.approved_scopes
		&& token.audience == record.audience
		&& token.code_digest.is_none()
		&& !token.revoked
}
