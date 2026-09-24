//! Registration and state storage for the authorization server.

use super::protocol::{PendingAuthorization, TokenPrincipal};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Whether a client has a server-side credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClientKind {
	/// Client without a server-side secret.
	Public,
	/// Client authenticated with a server-side secret.
	Confidential,
}

/// Administrative registration of an OAuth client.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
	/// Whether the client credentials grant is enabled.
	pub client_credentials: bool,
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
}

/// Atomic storage contract for a multi-instance OAuth server.
#[async_trait]
pub trait OAuthServerStore: Send + Sync {
	/// Save a validated registration.
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String>;
	/// Replace a registration only if its complete current value matches the snapshot.
	/// A concurrent administrative change returns false without writing.
	async fn compare_and_swap_client(
		&self,
		expected: &ClientRegistration,
		replacement: ClientRegistration,
	) -> Result<bool, String>;
	/// Atomically disable a client and invalidate its pending requests, codes, and tokens.
	/// Returns the number of newly revoked tokens, or none for a missing registration.
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
	/// A correctly bound replay atomically revokes linked tokens, even if expired.
	/// Returns none for missing, mismatched, expired, or replayed codes.
	async fn inspect_code_for_exchange(
		&self,
		request: CodeInspection<'_>,
	) -> Result<Option<StoredCode>, String>;
	/// Atomically redeem a bound code and save its token, rolling both back on failure.
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
	/// Invalidate outstanding user codes and revoke tokens atomically. Returns newly revoked token count.
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String>;
	/// Invalidate a retired user's codes and tokens together.
	async fn retire_user(&self, user_id: &str) -> Result<(), String>;
	/// Revoke all client tokens (administrative operation).
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String>;
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
		let mut count = 0;
		for token in state
			.tokens
			.values_mut()
			.filter(|token| token.client_id == id && !token.revoked)
		{
			token.revoked = true;
			count += 1;
		}
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
			return Ok(None);
		}
		Ok((code.expires_at > request.now).then(|| code.clone()))
	}
	async fn redeem_code_and_store_token(
		&self,
		request: CodeRedemptionRequest<'_>,
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
		let mut state = self.state.lock().await;
		let token_collision = state.tokens.contains_key(&token.digest);
		let Some(code) = state.codes.get_mut(digest) else {
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
			for linked in state
				.tokens
				.values_mut()
				.filter(|t| t.code_digest.as_deref() == Some(digest))
			{
				linked.revoked = true;
			}
			return Ok(CodeRedemption::Replay);
		}
		if code.expires_at <= now || !token_matches_code(&token, code) {
			return Ok(CodeRedemption::Invalid);
		}
		if token_collision {
			return Err("token digest collision".to_owned());
		}
		code.redeemed = true;
		let redeemed = code.clone();
		state.tokens.insert(token.digest.clone(), token);
		Ok(CodeRedemption::Valid(redeemed))
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
		Ok(count)
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		let mut state = self.state.lock().await;
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
		Ok(count)
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
