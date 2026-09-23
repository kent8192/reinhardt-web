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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientRegistration {
	/// Unique identifier.
	pub client_id: String,
	/// Public or confidential.
	pub kind: ClientKind,
	/// Password hash for a confidential client's secret.
	pub secret_hash: Option<String>,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingRecord {
	/// Request data to present to the host.
	pub request: PendingAuthorization,
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
	/// UNIX expiry time.
	pub expires_at: i64,
	/// Whether a successful redemption has occurred.
	pub redeemed: bool,
	/// Whether the redeemed code was replayed.
	pub replayed: bool,
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
	/// Find a registered resource by its unique audience.
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String>;
	/// Save a pending request with a random identifier.
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String>;
	/// Atomically consume a pending request.
	async fn take_pending(&self, id: &str) -> Result<Option<PendingRecord>, String>;
	/// Save an authorization code.
	async fn put_code(&self, code: StoredCode) -> Result<(), String>;
	/// Atomically redeem a code only when all bindings match; revoke linked tokens on replay.
	async fn redeem_code(
		&self,
		digest: &str,
		client_id: &str,
		redirect_uri: &str,
		challenge: &str,
		resource: Option<&str>,
		now: i64,
	) -> Result<CodeRedemption, String>;
	/// Save access token metadata.
	async fn put_token(&self, token: StoredToken) -> Result<(), String>;
	/// Look up opaque token metadata.
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String>;
	/// Revoke a token belonging to the specified client.
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String>;
	/// Revoke all user tokens (for host account-security events).
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String>;
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
	async fn take_pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		Ok(self.state.lock().await.pending.remove(id))
	}
	async fn put_code(&self, code: StoredCode) -> Result<(), String> {
		self.state
			.lock()
			.await
			.codes
			.insert(code.digest.clone(), code);
		Ok(())
	}
	async fn redeem_code(
		&self,
		digest: &str,
		client_id: &str,
		redirect_uri: &str,
		challenge: &str,
		resource: Option<&str>,
		now: i64,
	) -> Result<CodeRedemption, String> {
		let mut state = self.state.lock().await;
		let Some(code) = state.codes.get_mut(digest) else {
			return Ok(CodeRedemption::Invalid);
		};
		if code.client_id != client_id
			|| code.redirect_uri != redirect_uri
			|| code.challenge != challenge
			|| resource.is_some_and(|r| r != code.audience)
		{
			return Ok(CodeRedemption::Invalid);
		}
		if code.redeemed {
			code.replayed = true;
			for token in state
				.tokens
				.values_mut()
				.filter(|t| t.code_digest.as_deref() == Some(digest))
			{
				token.revoked = true;
			}
			return Ok(CodeRedemption::Replay);
		}
		if code.expires_at <= now {
			return Ok(CodeRedemption::Invalid);
		}
		code.redeemed = true;
		Ok(CodeRedemption::Valid(code.clone()))
	}
	async fn put_token(&self, token: StoredToken) -> Result<(), String> {
		let mut state = self.state.lock().await;
		let mut token = token;
		if token
			.code_digest
			.as_ref()
			.is_some_and(|digest| state.codes.get(digest).is_none_or(|code| code.replayed))
		{
			token.revoked = true;
		}
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
		let mut count = 0;
		for token in state.tokens.values_mut() {
			if token.principal == TokenPrincipal::User(user_id.to_owned()) && !token.revoked {
				token.revoked = true;
				count += 1;
			}
		}
		Ok(count)
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
