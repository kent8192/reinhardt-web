//! Durable OIDC state contracts, separate from shared OAuth client and token state.

use crate::oauth2_server::{AuthorizationCommit, OAuthServerStore};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;

/// Public RSA signing key published in JWKS.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublicRsaJwk {
	/// Stable key identifier.
	pub kid: String,
	/// Base64url RSA modulus.
	pub n: String,
	/// Base64url RSA exponent.
	pub e: String,
}

/// Shared signing-key lifecycle state. Private material stays with the signer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcKey {
	/// Public key parameters.
	pub public: PublicRsaJwk,
	/// UNIX time when this key became active.
	pub activated_at: i64,
	/// Whether this key signs new tokens.
	pub active: bool,
	/// UNIX time after which a retired public key is no longer published.
	pub publish_until: Option<i64>,
	/// Emergency removal prevents publication regardless of cache lifetime.
	pub compromised: bool,
}

/// OIDC fields bound to the OAuth pending authorization identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OidcPending {
	/// SHA-256 digest of the browser session allowed to consume this continuation.
	pub session_digest: String,
	/// UNIX time when the validated request began.
	pub requested_at: i64,
	/// Registered client identifier.
	pub client_id: String,
	/// Exact validated redirect URI.
	pub redirect_uri: String,
	/// Client state to echo in authorization responses.
	pub state: Option<String>,
	/// Client-supplied nonce, if any.
	pub nonce: Option<String>,
	/// Requested prompt values.
	pub prompts: Vec<String>,
	/// Maximum permitted age of active authentication in seconds.
	pub max_age: Option<u64>,
	/// UNIX expiry time.
	pub expires_at: i64,
}

/// OIDC data bound to an OAuth authorization-code digest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcCodeContext {
	/// SHA-256 digest of the code.
	pub digest: String,
	/// Host local user identifier.
	pub user_id: String,
	/// Registered client identifier.
	pub client_id: String,
	/// Nonce to copy into the ID Token.
	pub nonce: Option<String>,
	/// UNIX time of active host authentication.
	pub auth_time: i64,
	/// UNIX expiry time.
	pub expires_at: i64,
}

/// Atomic state contract for subjects, OIDC continuations, and signing keys.
#[async_trait]
pub trait OidcStateStore: Send + Sync {
	/// Return the stable opaque subject, atomically creating it if absent.
	async fn subject_or_insert(&self, user_id: &str, proposed: &str) -> Result<String, String>;
	/// Return an existing live subject without creating one.
	async fn subject(&self, user_id: &str) -> Result<Option<String>, String>;
	/// Remove a user's live link while permanently reserving the subject.
	async fn retire_subject(&self, user_id: &str) -> Result<(), String>;
	/// Invalidate a retired user's code contexts and subject; PostgreSQL also invalidates OAuth codes and tokens in the same transaction.
	async fn retire_user(&self, user_id: &str) -> Result<(), String>;
	/// Persist a validated pending OIDC request.
	async fn put_pending(&self, id: &str, pending: OidcPending) -> Result<(), String>;
	/// Read a continuation without consuming it before fallible host validation.
	async fn pending(&self, id: &str) -> Result<Option<OidcPending>, String>;
	/// Atomically consume both matching continuations and store both code records.
	/// PostgreSQL stores must share a database; in-memory stores use a synchronous
	/// commit callback under both locks. Incompatible backends fail without changes.
	async fn complete_authorization(
		&self,
		oauth: &dyn OAuthServerStore,
		request: AuthorizationCommit<'_>,
		pending: &OidcPending,
		context: Option<&OidcCodeContext>,
	) -> Result<bool, String>;
	/// Consume an unexpired OIDC request only for its browser session.
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		now: i64,
	) -> Result<Option<OidcPending>, String>;
	/// Bind OIDC claims to a code before disclosing it to the RP.
	async fn put_code(&self, context: OidcCodeContext) -> Result<(), String>;
	/// Read claims for a code that is atomically redeemed by the OAuth store.
	async fn code(&self, digest: &str) -> Result<Option<OidcCodeContext>, String>;
	/// Activate a new key, retaining the old public key for a bounded time.
	async fn rotate_key(
		&self,
		public: PublicRsaJwk,
		now: i64,
		retention: i64,
	) -> Result<(), String>;
	/// Return the key used to sign new tokens.
	async fn active_key(&self) -> Result<Option<OidcKey>, String>;
	/// Return public keys that should still be advertised.
	async fn public_keys(&self, now: i64) -> Result<Vec<OidcKey>, String>;
	/// Remove a compromised key from signing and JWKS immediately.
	async fn compromise_key(&self, kid: &str) -> Result<(), String>;
}

pub(crate) fn authorization_matches(
	request: AuthorizationCommit<'_>,
	pending: &OidcPending,
	context: Option<&OidcCodeContext>,
) -> bool {
	let oauth = request.pending;
	request.is_valid()
		&& oauth.oidc
		&& pending.expires_at > request.now
		&& pending.session_digest == oauth.session_digest
		&& pending.client_id == oauth.request.client_id
		&& pending.redirect_uri == oauth.request.redirect_uri
		&& pending.state == oauth.request.state
		&& match (request.code, context) {
			(None, None) => true,
			(Some(code), Some(context)) => {
				context.digest == code.digest
					&& context.user_id == code.user_id
					&& context.client_id == code.client_id
					&& context.nonce == pending.nonce
					&& context.expires_at == code.expires_at
					&& context.auth_time > 0
			}
			_ => false,
		}
}

/// Development and test store; production uses a shared SQL store.
#[derive(Default)]
pub struct MemoryOidcStore {
	state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
	subjects: HashMap<String, String>,
	reserved_subjects: HashSet<String>,
	pending: HashMap<String, OidcPending>,
	codes: HashMap<String, OidcCodeContext>,
	keys: HashMap<String, OidcKey>,
}

impl MemoryOidcStore {
	/// Create an empty in-memory OIDC state store.
	pub fn new() -> Self {
		Self::default()
	}
}

#[async_trait]
impl OidcStateStore for MemoryOidcStore {
	async fn subject_or_insert(&self, user_id: &str, proposed: &str) -> Result<String, String> {
		let mut state = self.state.lock().await;
		if let Some(subject) = state.subjects.get(user_id) {
			return Ok(subject.clone());
		}
		if !state.reserved_subjects.insert(proposed.to_owned()) {
			return Err("subject collision".to_owned());
		}
		state
			.subjects
			.insert(user_id.to_owned(), proposed.to_owned());
		Ok(proposed.to_owned())
	}
	async fn subject(&self, user_id: &str) -> Result<Option<String>, String> {
		Ok(self.state.lock().await.subjects.get(user_id).cloned())
	}
	async fn retire_subject(&self, user_id: &str) -> Result<(), String> {
		self.state.lock().await.subjects.remove(user_id);
		Ok(())
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		let mut state = self.state.lock().await;
		state.subjects.remove(user_id);
		state.codes.retain(|_, code| code.user_id != user_id);
		Ok(())
	}
	async fn put_pending(&self, id: &str, pending: OidcPending) -> Result<(), String> {
		self.state
			.lock()
			.await
			.pending
			.insert(id.to_owned(), pending);
		Ok(())
	}
	async fn pending(&self, id: &str) -> Result<Option<OidcPending>, String> {
		Ok(self.state.lock().await.pending.get(id).cloned())
	}
	async fn complete_authorization(
		&self,
		oauth: &dyn OAuthServerStore,
		request: AuthorizationCommit<'_>,
		pending: &OidcPending,
		context: Option<&OidcCodeContext>,
	) -> Result<bool, String> {
		// Always acquire OIDC before OAuth. Waiting for either lock changes no state.
		let mut state = self.state.lock().await;
		let id = &request.pending.request.id;
		if state.pending.get(id) != Some(pending)
			|| !authorization_matches(request, pending, context)
		{
			return Ok(false);
		}
		if context.is_some_and(|context| state.codes.contains_key(&context.digest)) {
			return Err("OIDC code digest collision".to_owned());
		}
		let mut on_commit = || {
			if let Some(context) = context {
				state.codes.insert(context.digest.clone(), context.clone());
			}
			state.pending.remove(id);
		};
		oauth
			.complete_pending_in_memory(request, &mut on_commit)
			.await
	}
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		now: i64,
	) -> Result<Option<OidcPending>, String> {
		let mut state = self.state.lock().await;
		if state.pending.get(id).is_none_or(|pending| {
			pending.session_digest != session_digest || pending.expires_at <= now
		}) {
			return Ok(None);
		}
		Ok(state.pending.remove(id))
	}
	async fn put_code(&self, context: OidcCodeContext) -> Result<(), String> {
		self.state
			.lock()
			.await
			.codes
			.insert(context.digest.clone(), context);
		Ok(())
	}
	async fn code(&self, digest: &str) -> Result<Option<OidcCodeContext>, String> {
		Ok(self.state.lock().await.codes.get(digest).cloned())
	}
	async fn rotate_key(
		&self,
		public: PublicRsaJwk,
		now: i64,
		retention: i64,
	) -> Result<(), String> {
		let mut state = self.state.lock().await;
		if state.keys.contains_key(&public.kid) {
			return Err("key identifier already exists".to_owned());
		}
		for key in state.keys.values_mut().filter(|key| key.active) {
			key.active = false;
			key.publish_until = Some(now + retention);
		}
		state.keys.insert(
			public.kid.clone(),
			OidcKey {
				public,
				activated_at: now,
				active: true,
				publish_until: None,
				compromised: false,
			},
		);
		Ok(())
	}
	async fn active_key(&self) -> Result<Option<OidcKey>, String> {
		Ok(self
			.state
			.lock()
			.await
			.keys
			.values()
			.find(|key| key.active)
			.cloned())
	}
	async fn public_keys(&self, now: i64) -> Result<Vec<OidcKey>, String> {
		let state = self.state.lock().await;
		let mut keys: Vec<_> = state
			.keys
			.values()
			.filter(|key| {
				!key.compromised && (key.active || key.publish_until.is_some_and(|end| end > now))
			})
			.cloned()
			.collect();
		keys.sort_by(|a, b| a.public.kid.cmp(&b.public.kid));
		Ok(keys)
	}
	async fn compromise_key(&self, kid: &str) -> Result<(), String> {
		if let Some(key) = self.state.lock().await.keys.get_mut(kid) {
			key.active = false;
			key.compromised = true;
			key.publish_until = None;
		}
		Ok(())
	}
}
