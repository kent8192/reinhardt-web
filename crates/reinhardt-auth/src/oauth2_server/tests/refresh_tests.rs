//! Coverage for refresh tokens: issuance, rotation, reuse detection, and revocation.

use super::*;
use bytes::Bytes;
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Request};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const REDIRECT: &str = "https://client.example/callback";
const AUDIENCE: &str = "https://api.example";

fn sha(value: &str) -> String {
	hex::encode(Sha256::digest(value.as_bytes()))
}

fn now_secs() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs() as i64
}

fn days(count: u64) -> Duration {
	Duration::from_secs(count * 24 * 3600)
}

fn policy() -> RefreshTokenPolicy {
	RefreshTokenPolicy::new(days(10), days(30)).unwrap()
}

struct Harness {
	server: Arc<OAuthServer>,
	store: Arc<MemoryOAuthStore>,
	secret: Option<String>,
}
impl Harness {
	fn secret(&self) -> Option<&str> {
		self.secret.as_deref()
	}
	/// Issue a first token pair for `user-a` approving `scopes`.
	async fn first_tokens(&self, scopes: &[&str]) -> IssuedToken {
		let code = self.code("user-a", scopes).await;
		self.server
			.exchange_code(
				&code,
				"client-a",
				self.secret(),
				REDIRECT,
				verifier(),
				Some(AUDIENCE),
			)
			.await
			.unwrap()
	}
	async fn code(&self, user_id: &str, scopes: &[&str]) -> String {
		let mut input = request(REDIRECT);
		input.scope = Some(scopes.join(" "));
		let pending = self
			.server
			.begin_authorization(input, "session")
			.await
			.unwrap();
		let location = self
			.server
			.complete_authorization(
				&pending.id,
				"session",
				AuthorizationDecision::Approve {
					user_id: user_id.into(),
					scopes: scopes.iter().map(|s| (*s).to_owned()).collect(),
				},
			)
			.await
			.unwrap();
		url::Url::parse(&location)
			.unwrap()
			.query_pairs()
			.find(|(key, _)| key == "code")
			.unwrap()
			.1
			.into_owned()
	}
	async fn refresh(
		&self,
		refresh_token: &str,
		scope: Option<&str>,
	) -> Result<IssuedToken, OAuthError> {
		self.server
			.refresh(refresh_token, "client-a", self.secret(), scope, None)
			.await
	}
	async fn access_active(&self, token: &str) -> bool {
		self.server.token_info(token).await.unwrap().is_some()
	}
	async fn inspect(&self, refresh_token: &str) -> RefreshInspection {
		self.store
			.inspect_refresh_token(&sha(refresh_token), "client-a", now_secs())
			.await
			.unwrap()
	}
	async fn replace_client(&self, change: impl FnOnce(&mut ClientRegistration)) {
		let mut registration = self.store.client("client-a").await.unwrap().unwrap();
		change(&mut registration);
		self.store.put_client(registration).await.unwrap();
	}
}

async fn harness_with(
	policy: Option<RefreshTokenPolicy>,
	refresh_flag: bool,
	kind: ClientKind,
	users: Arc<dyn crate::UserRepository>,
) -> Harness {
	let mut config = config();
	config.refresh_tokens = policy;
	let store = Arc::new(MemoryOAuthStore::new());
	let server = Arc::new(
		OAuthServer::for_development(config, store.clone(), users, Arc::new(AllowAll)).unwrap(),
	);
	server
		.register_resource("resource-a", AUDIENCE)
		.await
		.unwrap();
	let mut registration = client(kind);
	registration.refresh_token = refresh_flag;
	let secret = server.register_client(registration).await.unwrap();
	Harness {
		server,
		store,
		secret,
	}
}

async fn harness(policy: Option<RefreshTokenPolicy>, refresh_flag: bool) -> Harness {
	harness_with(
		policy,
		refresh_flag,
		ClientKind::Public,
		Arc::new(SimpleUserRepository),
	)
	.await
}

async fn enabled() -> Harness {
	harness(Some(policy()), true).await
}

// ---------------------------------------------------------------------------
// RefreshTokenPolicy and configuration
// ---------------------------------------------------------------------------

#[rstest]
#[case::default_like(days(30), days(90), true)]
#[case::equal(days(7), days(7), true)]
#[case::upper_bounds(days(90), days(365), true)]
#[case::one_second(Duration::from_secs(1), Duration::from_secs(1), true)]
#[case::zero_idle(Duration::ZERO, days(1), false)]
#[case::zero_max(days(1), Duration::ZERO, false)]
#[case::idle_over_bound(days(91), days(365), false)]
#[case::max_over_bound(days(30), days(366), false)]
#[case::idle_over_max(days(10), days(5), false)]
#[case::sub_second_idle(Duration::from_millis(500), days(1), false)]
fn refresh_policy_enforces_documented_bounds(
	#[case] idle: Duration,
	#[case] max: Duration,
	#[case] valid: bool,
) {
	// Act
	let result = RefreshTokenPolicy::new(idle, max);

	// Assert
	match result {
		Ok(policy) => {
			assert!(valid);
			assert_eq!((policy.idle_ttl, policy.max_lifetime), (idle, max));
		}
		Err(error) => {
			assert!(!valid);
			assert_eq!(error, OAuthError::InvalidRequest);
		}
	}
}

#[rstest]
fn refresh_policy_default_is_thirty_and_ninety_days() {
	// Act
	let policy = RefreshTokenPolicy::default();

	// Assert
	assert_eq!(policy.idle_ttl, days(30));
	assert_eq!(policy.max_lifetime, days(90));
}

#[rstest]
fn refresh_tokens_are_disabled_by_default() {
	// Assert
	assert!(config().refresh_tokens.is_none());
	let loopback = OAuthServerConfig::for_loopback_development(
		"http://localhost",
		"http://localhost/authorize",
		"http://localhost/token",
		"http://localhost/revoke",
		"http://localhost/introspect",
	)
	.unwrap();
	assert!(loopback.refresh_tokens.is_none());
}

#[rstest]
fn server_construction_revalidates_the_refresh_policy() {
	// Arrange: the public fields allow constructing an out-of-bounds policy directly.
	let mut config = config();
	config.refresh_tokens = Some(RefreshTokenPolicy {
		idle_ttl: days(100),
		max_lifetime: days(100),
	});

	// Act
	let result = OAuthServer::for_development(
		config,
		Arc::new(MemoryOAuthStore::new()),
		Arc::new(SimpleUserRepository),
		Arc::new(AllowAll),
	);

	// Assert
	assert_eq!(result.err(), Some(OAuthError::InvalidRequest));
}

#[rstest]
#[tokio::test]
async fn registration_rejects_refresh_flag_without_authorization_code() {
	// Arrange
	let harness = harness(Some(policy()), false).await;
	let mut registration = client(ClientKind::Confidential);
	registration.client_id = "client-credentials-only".into();
	registration.authorization_code = false;
	registration.redirect_uris.clear();
	registration.refresh_token = true;

	// Act
	let result = harness.server.register_client(registration.clone()).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidRequest);
	registration.refresh_token = false;
	assert!(harness.server.register_client(registration).await.is_ok());
}

#[rstest]
#[tokio::test]
async fn registration_allows_refresh_flag_for_public_clients() {
	// Arrange
	let harness = harness(Some(policy()), false).await;
	let mut registration = client(ClientKind::Public);
	registration.client_id = "public-refresh".into();
	registration.refresh_token = true;

	// Act
	let result = harness.server.register_client(registration).await;

	// Assert
	assert_eq!(result.unwrap(), None);
}

#[rstest]
fn legacy_json_without_refresh_fields_still_deserializes() {
	// Arrange: records persisted before refresh-token support lack the new keys.
	let mut registration = serde_json::to_value(client(ClientKind::Public)).unwrap();
	registration
		.as_object_mut()
		.unwrap()
		.remove("refresh_token");
	let token = StoredToken {
		digest: "digest".into(),
		client_id: "client-a".into(),
		principal: TokenPrincipal::User("user-a".into()),
		scopes: vec!["read".into()],
		audience: AUDIENCE.into(),
		issued_at: 1,
		expires_at: 2,
		revoked: false,
		code_digest: None,
		family_id: Some("family".into()),
	};
	let mut token_json = serde_json::to_value(&token).unwrap();
	token_json.as_object_mut().unwrap().remove("family_id");

	// Act
	let registration: ClientRegistration = serde_json::from_value(registration).unwrap();
	let token: StoredToken = serde_json::from_value(token_json).unwrap();

	// Assert
	assert!(!registration.refresh_token);
	assert_eq!(token.family_id, None);
}

#[rstest]
fn issued_token_omits_absent_refresh_token_in_json() {
	// Arrange
	let mut token = IssuedToken {
		access_token: "access".into(),
		token_type: "Bearer",
		expires_in: 60,
		scope: "read".into(),
		refresh_token: None,
	};

	// Act and assert
	assert!(
		serde_json::to_value(&token)
			.unwrap()
			.get("refresh_token")
			.is_none()
	);
	token.refresh_token = Some("refresh".into());
	assert_eq!(
		serde_json::to_value(&token).unwrap()["refresh_token"],
		"refresh"
	);
}

// ---------------------------------------------------------------------------
// Issuance
// ---------------------------------------------------------------------------

#[rstest]
#[case::both_opt_in(true, true, true)]
#[case::client_not_opted_in(true, false, false)]
#[case::server_policy_missing(false, true, false)]
#[case::neither(false, false, false)]
#[tokio::test]
async fn refresh_token_is_issued_only_when_server_and_client_opt_in(
	#[case] policy_enabled: bool,
	#[case] client_flag: bool,
	#[case] expect_refresh: bool,
) {
	// Arrange
	let harness = harness(policy_enabled.then(policy), client_flag).await;

	// Act
	let issued = harness.first_tokens(&["read"]).await;

	// Assert
	assert_eq!(issued.refresh_token.is_some(), expect_refresh);
	assert!(harness.access_active(&issued.access_token).await);
	assert_eq!(issued.scope, "read");
}

#[rstest]
#[tokio::test]
async fn issued_refresh_token_is_a_digest_only_family_member() {
	// Arrange
	let harness = enabled().await;

	// Act
	let issued = harness.first_tokens(&["read", "write"]).await;
	let raw = issued.refresh_token.unwrap();

	// Assert: the raw value is not a lookup key, its digest is.
	assert!(matches!(
		harness
			.store
			.inspect_refresh_token(&raw, "client-a", now_secs())
			.await
			.unwrap(),
		RefreshInspection::Invalid
	));
	let RefreshInspection::Active(token, family) = harness.inspect(&raw).await else {
		panic!("fresh refresh token must be active");
	};
	assert_eq!(token.digest, sha(&raw));
	assert_eq!(token.parent_digest, None);
	assert!(!token.rotated);
	assert_eq!(family.client_id, "client-a");
	assert_eq!(family.user_id, "user-a");
	assert_eq!(family.scopes, vec!["read", "write"]);
	assert_eq!(family.audience, AUDIENCE);
	assert_eq!(
		family.absolute_expires_at - family.created_at,
		30 * 24 * 3600
	);
	assert_eq!(token.idle_expires_at - token.issued_at, 10 * 24 * 3600);
	let access = harness
		.store
		.token(&sha(&issued.access_token))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(access.family_id.as_deref(), Some(family.family_id.as_str()));
}

#[rstest]
#[tokio::test]
async fn idle_lifetime_is_capped_at_the_absolute_lifetime() {
	// Arrange: the idle limit equals the absolute limit.
	let harness = harness(
		Some(RefreshTokenPolicy::new(days(30), days(30)).unwrap()),
		true,
	)
	.await;

	// Act
	let issued = harness.first_tokens(&["read"]).await;
	let refreshed = harness
		.refresh(&issued.refresh_token.unwrap(), None)
		.await
		.unwrap();

	// Assert
	let RefreshInspection::Active(token, family) =
		harness.inspect(&refreshed.refresh_token.unwrap()).await
	else {
		panic!("rotated refresh token must be active");
	};
	assert!(token.idle_expires_at <= family.absolute_expires_at);
}

#[rstest]
#[tokio::test]
async fn client_credentials_never_issue_refresh_tokens() {
	// Arrange
	let harness = harness_with(
		Some(policy()),
		true,
		ClientKind::Confidential,
		Arc::new(SimpleUserRepository),
	)
	.await;

	// Act
	let issued = harness
		.server
		.client_credentials("client-a", harness.secret().unwrap(), None, None)
		.await
		.unwrap();

	// Assert
	assert_eq!(issued.refresh_token, None);
}

#[cfg(feature = "oidc-op")]
#[rstest]
#[tokio::test]
async fn oidc_code_exchange_never_issues_refresh_tokens() {
	use super::super::CodeExchangeRequest;
	// Arrange: an OIDC-enabled client that also opted into refresh tokens.
	let mut config = config();
	config.refresh_tokens = Some(policy());
	let store = Arc::new(MemoryOAuthStore::new());
	let server = OAuthServer::for_development(
		config,
		store.clone(),
		Arc::new(SimpleUserRepository),
		Arc::new(AllowAll),
	)
	.unwrap();
	server
		.register_resource("resource-a", AUDIENCE)
		.await
		.unwrap();
	let mut registration = client(ClientKind::Confidential);
	registration.oidc_enabled = true;
	registration.refresh_token = true;
	registration.scopes = vec!["openid".into()];
	registration.default_scopes = vec!["openid".into()];
	registration.client_credentials = false;
	registration.browser_origins.clear();
	let secret = server.register_client(registration).await.unwrap().unwrap();
	let mut input = request(REDIRECT);
	input.scope = Some("openid".into());
	let pending = server
		.begin_oidc_authorization(input, "session")
		.await
		.unwrap();
	let prepared = server
		.prepare_oidc_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["openid".into()],
			},
		)
		.await
		.unwrap();
	assert!(
		store
			.complete_pending(AuthorizationCommit {
				pending: &prepared.pending,
				code: prepared.code.as_ref(),
				now: now_secs(),
			})
			.await
			.unwrap()
	);
	let code = url::Url::parse(&prepared.redirect)
		.unwrap()
		.query_pairs()
		.find(|(key, _)| key == "code")
		.unwrap()
		.1
		.into_owned();

	// Act
	let (issued, user_id) = server
		.exchange_oidc_code(CodeExchangeRequest {
			code: &code,
			client_id: "client-a",
			client_secret: Some(&secret),
			redirect_uri: REDIRECT,
			verifier: verifier(),
			resource: None,
			ttl: Duration::from_secs(300),
		})
		.await
		.unwrap();

	// Assert
	assert_eq!(user_id, "user-a");
	assert_eq!(issued.refresh_token, None);
	let stored = store
		.token(&sha(&issued.access_token))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(stored.family_id, None);
}

#[rstest]
#[tokio::test]
async fn confidential_clients_authenticate_when_refreshing() {
	// Arrange
	let harness = harness_with(
		Some(policy()),
		true,
		ClientKind::Confidential,
		Arc::new(SimpleUserRepository),
	)
	.await;
	let issued = harness.first_tokens(&["read"]).await;
	let refresh_token = issued.refresh_token.unwrap();

	// Act and assert: a missing or wrong secret is rejected before the token is consumed.
	assert_eq!(
		harness
			.server
			.refresh(&refresh_token, "client-a", None, None, None)
			.await
			.unwrap_err(),
		OAuthError::InvalidClient
	);
	assert_eq!(
		harness
			.server
			.refresh(&refresh_token, "client-a", Some("wrong"), None, None)
			.await
			.unwrap_err(),
		OAuthError::InvalidClient
	);
	assert!(harness.refresh(&refresh_token, None).await.is_ok());
}

// ---------------------------------------------------------------------------
// Rotation and reuse detection
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn refresh_rotates_to_a_new_token_and_keeps_the_family() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read", "write"]).await;
	let first_refresh = first.refresh_token.clone().unwrap();
	let RefreshInspection::Active(_, original_family) = harness.inspect(&first_refresh).await
	else {
		panic!("fresh refresh token must be active");
	};

	// Act
	let second = harness.refresh(&first_refresh, None).await.unwrap();

	// Assert
	let second_refresh = second.refresh_token.clone().unwrap();
	assert_ne!(second_refresh, first_refresh);
	assert_ne!(second.access_token, first.access_token);
	assert_eq!(second.scope, "read write");
	assert_eq!(second.token_type, "Bearer");
	assert_eq!(second.expires_in, 3600);
	assert!(harness.access_active(&second.access_token).await);
	assert!(harness.access_active(&first.access_token).await);
	let RefreshInspection::Active(token, family) = harness.inspect(&second_refresh).await else {
		panic!("replacement refresh token must be active");
	};
	assert_eq!(token.parent_digest, Some(sha(&first_refresh)));
	assert_eq!(family.family_id, original_family.family_id);
	assert_eq!(
		family.absolute_expires_at,
		original_family.absolute_expires_at
	);
	assert_eq!(family.scopes, original_family.scopes);
	let access = harness
		.store
		.token(&sha(&second.access_token))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(access.family_id.as_deref(), Some(family.family_id.as_str()));
	assert_eq!(access.principal, TokenPrincipal::User("user-a".into()));
	assert_eq!(access.audience, AUDIENCE);
}

#[rstest]
#[tokio::test]
async fn reusing_a_rotated_token_revokes_the_whole_family() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let first_refresh = first.refresh_token.clone().unwrap();
	let second = harness.refresh(&first_refresh, None).await.unwrap();
	let second_refresh = second.refresh_token.clone().unwrap();

	// Act: the already-rotated token is presented again.
	let replay = harness.refresh(&first_refresh, None).await;

	// Assert: reuse is invalid_grant and takes down every token of the family.
	assert_eq!(replay.unwrap_err(), OAuthError::InvalidGrant);
	assert!(!harness.access_active(&first.access_token).await);
	assert!(!harness.access_active(&second.access_token).await);
	assert_eq!(
		harness.refresh(&second_refresh, None).await.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert!(matches!(
		harness.inspect(&second_refresh).await,
		RefreshInspection::Invalid
	));
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_refresh_has_exactly_one_winner_and_revokes_the_family() {
	// Arrange
	let harness = Arc::new(enabled().await);
	let first = harness.first_tokens(&["read"]).await;
	let first_refresh = first.refresh_token.clone().unwrap();

	// Act
	let mut attempts = tokio::task::JoinSet::new();
	for _ in 0..8 {
		let harness = harness.clone();
		let token = first_refresh.clone();
		attempts.spawn(async move { harness.refresh(&token, None).await });
	}
	let mut winners = Vec::new();
	let mut losers = 0;
	while let Some(result) = attempts.join_next().await {
		match result.unwrap() {
			Ok(issued) => winners.push(issued),
			Err(error) => {
				assert_eq!(error, OAuthError::InvalidGrant);
				losers += 1;
			}
		}
	}

	// Assert: one winner; the losers' reuse revoked the family, including the winner's tokens.
	assert_eq!(winners.len(), 1);
	assert_eq!(losers, 7);
	let winner = winners.pop().unwrap();
	assert!(!harness.access_active(&winner.access_token).await);
	assert_eq!(
		harness
			.refresh(&winner.refresh_token.unwrap(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
}

#[rstest]
#[tokio::test]
async fn refresh_requires_a_known_token_value() {
	// Arrange
	let harness = enabled().await;
	harness.first_tokens(&["read"]).await;

	// Act and assert
	assert_eq!(
		harness.refresh("not-a-token", None).await.unwrap_err(),
		OAuthError::InvalidGrant
	);
}

// ---------------------------------------------------------------------------
// Scope, resource, and account rules
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn refresh_can_narrow_scope_without_losing_the_original_grant() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read", "write"]).await;

	// Act
	let narrowed = harness
		.refresh(&first.refresh_token.unwrap(), Some("read"))
		.await
		.unwrap();
	let restored = harness
		.refresh(&narrowed.refresh_token.clone().unwrap(), None)
		.await
		.unwrap();

	// Assert
	assert_eq!(narrowed.scope, "read");
	let info = harness
		.server
		.token_info(&narrowed.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(info.scopes, vec!["read"]);
	assert_eq!(restored.scope, "read write");
}

#[rstest]
#[case::widening("read write", OAuthError::InvalidScope)]
#[case::unrelated("admin", OAuthError::InvalidScope)]
#[case::empty("", OAuthError::InvalidScope)]
#[case::duplicate("read read", OAuthError::InvalidScope)]
#[tokio::test]
async fn refresh_cannot_widen_scope_and_does_not_consume_the_token(
	#[case] requested: &str,
	#[case] expected: OAuthError,
) {
	// Arrange: the grant covers only `read` even though the client may use `write`.
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();

	// Act
	let result = harness.refresh(&refresh_token, Some(requested)).await;

	// Assert
	assert_eq!(result.unwrap_err(), expected);
	assert!(harness.refresh(&refresh_token, None).await.is_ok());
}

#[rstest]
#[case::shrunk_allowlist(vec!["read"], Ok("read"))]
#[case::disjoint_allowlist(vec!["admin"], Err(OAuthError::InvalidGrant))]
#[tokio::test]
async fn refresh_intersects_scope_with_the_clients_current_allowlist(
	#[case] allowlist: Vec<&str>,
	#[case] expected: Result<&str, OAuthError>,
) {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read", "write"]).await;
	harness
		.replace_client(|client| {
			client.scopes = allowlist.iter().map(|s| (*s).to_owned()).collect();
			client.default_scopes = client.scopes.clone();
		})
		.await;

	// Act
	let result = harness.refresh(&first.refresh_token.unwrap(), None).await;

	// Assert
	assert_eq!(
		result.map(|issued| issued.scope),
		expected.map(str::to_owned)
	);
}

#[rstest]
#[tokio::test]
async fn refresh_rejects_a_different_resource() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();

	// Act
	let mismatch = harness
		.server
		.refresh(
			&refresh_token,
			"client-a",
			None,
			None,
			Some("https://other.example"),
		)
		.await;
	let matching = harness
		.server
		.refresh(&refresh_token, "client-a", None, None, Some(AUDIENCE))
		.await;

	// Assert
	assert_eq!(mismatch.unwrap_err(), OAuthError::InvalidTarget);
	assert!(matching.is_ok());
}

struct ToggleUser(Arc<AtomicBool>);
impl crate::AuthIdentity for ToggleUser {
	fn id(&self) -> String {
		"user-a".into()
	}
	fn is_authenticated(&self) -> bool {
		true
	}
	fn is_admin(&self) -> bool {
		false
	}
	fn is_account_active(&self) -> bool {
		self.0.load(Ordering::SeqCst)
	}
}
struct ToggleUsers(Arc<AtomicBool>);
#[async_trait]
impl crate::UserRepository for ToggleUsers {
	async fn get_user_by_id(
		&self,
		_: &str,
	) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
		Ok(Some(Box::new(ToggleUser(self.0.clone()))))
	}
}

#[rstest]
#[tokio::test]
async fn inactive_user_blocks_refresh_without_revoking_the_family() {
	// Arrange
	let active = Arc::new(AtomicBool::new(true));
	let harness = harness_with(
		Some(policy()),
		true,
		ClientKind::Public,
		Arc::new(ToggleUsers(active.clone())),
	)
	.await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	active.store(false, Ordering::SeqCst);

	// Act
	let blocked = harness.refresh(&refresh_token, None).await;
	active.store(true, Ordering::SeqCst);
	let recovered = harness.refresh(&refresh_token, None).await;

	// Assert
	assert_eq!(blocked.unwrap_err(), OAuthError::InvalidGrant);
	assert!(recovered.is_ok());
}

#[rstest]
#[tokio::test]
async fn disabled_audience_blocks_refresh_without_revoking_the_family() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	let resource = harness.store.resource("resource-a").await.unwrap().unwrap();
	let mut disabled = resource.clone();
	disabled.enabled = false;
	harness.store.put_resource(disabled).await.unwrap();

	// Act
	let blocked = harness.refresh(&refresh_token, None).await;
	harness.store.put_resource(resource).await.unwrap();
	let recovered = harness.refresh(&refresh_token, None).await;

	// Assert
	assert_eq!(blocked.unwrap_err(), OAuthError::InvalidGrant);
	assert!(recovered.is_ok());
}

// ---------------------------------------------------------------------------
// Server and client switches
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn refresh_is_unsupported_without_a_server_policy() {
	// Arrange
	let harness = harness(None, true).await;

	// Act
	let result = harness.refresh("anything", None).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::UnsupportedGrantType);
}

#[rstest]
#[tokio::test]
async fn removing_the_client_flag_stops_refreshing() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	harness
		.replace_client(|client| client.refresh_token = false)
		.await;

	// Act
	let result = harness.refresh(&first.refresh_token.unwrap(), None).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::UnauthorizedClient);
}

#[rstest]
#[tokio::test]
async fn another_clients_refresh_token_is_invalid_and_stays_usable() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	let mut other = client(ClientKind::Public);
	other.client_id = "client-b".into();
	other.refresh_token = true;
	harness.server.register_client(other).await.unwrap();

	// Act
	let foreign = harness
		.server
		.refresh(&refresh_token, "client-b", None, None, None)
		.await;

	// Assert: indistinguishable from an unknown token, and the owner is unaffected.
	assert_eq!(foreign.unwrap_err(), OAuthError::InvalidGrant);
	assert!(harness.access_active(&first.access_token).await);
	assert!(harness.refresh(&refresh_token, None).await.is_ok());
}

#[rstest]
#[tokio::test]
async fn realtime_idle_expiry_invalidates_the_token() {
	// Arrange: the shortest legal lifetime.
	let harness = harness(
		Some(RefreshTokenPolicy::new(Duration::from_secs(1), Duration::from_secs(1)).unwrap()),
		true,
	)
	.await;
	let first = harness.first_tokens(&["read"]).await;
	tokio::time::sleep(Duration::from_millis(2100)).await;

	// Act
	let result = harness.refresh(&first.refresh_token.unwrap(), None).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidGrant);
}

// ---------------------------------------------------------------------------
// Store contract (memory implementation)
// ---------------------------------------------------------------------------

fn stored_code(digest: &str) -> StoredCode {
	StoredCode {
		digest: digest.into(),
		client_id: "client-a".into(),
		redirect_uri: REDIRECT.into(),
		challenge: challenge(verifier()),
		user_id: "user-a".into(),
		scopes: vec!["read".into()],
		audience: AUDIENCE.into(),
		oidc: false,
		expires_at: i64::MAX,
		redeemed: false,
		replayed: false,
	}
}

fn access_token(digest: &str, code_digest: &str, family_id: &str) -> StoredToken {
	StoredToken {
		digest: digest.into(),
		client_id: "client-a".into(),
		principal: TokenPrincipal::User("user-a".into()),
		scopes: vec!["read".into()],
		audience: AUDIENCE.into(),
		issued_at: 0,
		expires_at: i64::MAX,
		revoked: false,
		code_digest: Some(code_digest.into()),
		family_id: Some(family_id.into()),
	}
}

fn family(family_id: &str, code_digest: &str, absolute_expires_at: i64) -> StoredTokenFamily {
	StoredTokenFamily {
		family_id: family_id.into(),
		client_id: "client-a".into(),
		user_id: "user-a".into(),
		scopes: vec!["read".into()],
		audience: AUDIENCE.into(),
		code_digest: code_digest.into(),
		created_at: 0,
		absolute_expires_at,
		revoked: false,
	}
}

fn refresh_record(digest: &str, family_id: &str, idle_expires_at: i64) -> StoredRefreshToken {
	StoredRefreshToken {
		digest: digest.into(),
		family_id: family_id.into(),
		parent_digest: None,
		issued_at: 0,
		idle_expires_at,
		rotated: false,
	}
}

async fn redeem_family(
	store: &MemoryOAuthStore,
	code_digest: &str,
	family_id: &str,
	idle_expires_at: i64,
	absolute_expires_at: i64,
) -> CodeRedemption {
	let challenge = challenge(verifier());
	store
		.redeem_code_and_store_tokens(CodeRedemptionWithRefresh {
			redemption: CodeRedemptionRequest {
				digest: code_digest,
				client_id: "client-a",
				redirect_uri: REDIRECT,
				challenge: &challenge,
				resource: None,
				expect_oidc: false,
				now: 0,
				token: access_token(&format!("access-{family_id}"), code_digest, family_id),
			},
			family: family(family_id, code_digest, absolute_expires_at),
			refresh_token: refresh_record(
				&format!("refresh-{family_id}"),
				family_id,
				idle_expires_at,
			),
		})
		.await
		.unwrap()
}

#[rstest]
#[case::inside_both(1000, 2000, 999, true)]
#[case::idle_expired(1000, 2000, 1000, false)]
#[case::absolute_binds_before_idle(3000, 2000, 1999, true)]
#[case::absolute_expired(3000, 2000, 2000, false)]
#[tokio::test]
async fn store_inspection_enforces_idle_and_absolute_expiry(
	#[case] idle: i64,
	#[case] absolute: i64,
	#[case] at: i64,
	#[case] active: bool,
) {
	// Arrange
	let store = MemoryOAuthStore::new();
	store.put_code(stored_code("code")).await.unwrap();
	assert!(matches!(
		redeem_family(&store, "code", "family", idle, absolute).await,
		CodeRedemption::Valid(_)
	));

	// Act
	let inspection = store
		.inspect_refresh_token("refresh-family", "client-a", at)
		.await
		.unwrap();

	// Assert
	assert_eq!(matches!(inspection, RefreshInspection::Active(..)), active);
	if !active {
		assert!(matches!(inspection, RefreshInspection::Invalid));
	}
}

#[rstest]
#[tokio::test]
async fn store_rotation_rechecks_expiry_and_writes_nothing_when_invalid() {
	// Arrange
	let store = MemoryOAuthStore::new();
	store.put_code(stored_code("code")).await.unwrap();
	redeem_family(&store, "code", "family", 1000, 2000).await;
	let mut replacement = refresh_record("replacement", "family", 1500);
	replacement.parent_digest = Some("refresh-family".into());

	// Act: the token idled out between inspection and rotation.
	let rotation = store
		.rotate_refresh_token(RefreshRotationRequest {
			presented_digest: "refresh-family",
			client_id: "client-a",
			now: 1000,
			replacement,
			token: access_token("access-new", "code", "family"),
		})
		.await
		.unwrap();

	// Assert
	assert!(matches!(rotation, RefreshRotation::Invalid));
	assert!(store.token("access-new").await.unwrap().is_none());
	assert!(matches!(
		store
			.inspect_refresh_token("replacement", "client-a", 0)
			.await
			.unwrap(),
		RefreshInspection::Invalid
	));
}

#[rstest]
#[case::wrong_family("other-family", Some("refresh-family"))]
#[case::wrong_parent("family", Some("different-parent"))]
#[case::no_parent("family", None)]
#[tokio::test]
async fn store_rotation_rejects_inconsistent_replacements(
	#[case] replacement_family: &str,
	#[case] parent: Option<&str>,
) {
	// Arrange
	let store = MemoryOAuthStore::new();
	store.put_code(stored_code("code")).await.unwrap();
	redeem_family(&store, "code", "family", 1000, 2000).await;
	let mut replacement = refresh_record("replacement", replacement_family, 1500);
	replacement.parent_digest = parent.map(str::to_owned);

	// Act
	let result = store
		.rotate_refresh_token(RefreshRotationRequest {
			presented_digest: "refresh-family",
			client_id: "client-a",
			now: 10,
			replacement,
			token: access_token("access-new", "code", "family"),
		})
		.await;

	// Assert
	assert!(result.is_err());
	assert!(store.token("access-new").await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn store_code_replay_through_atomic_redemption_revokes_the_family() {
	// Arrange
	let store = MemoryOAuthStore::new();
	store.put_code(stored_code("code")).await.unwrap();
	assert!(matches!(
		redeem_family(&store, "code", "family", 1000, 2000).await,
		CodeRedemption::Valid(_)
	));

	// Act: the same code is redeemed again (even by another instance's request).
	let replay = redeem_family(&store, "code", "second-family", 1000, 2000).await;

	// Assert
	assert!(matches!(replay, CodeRedemption::Replay));
	assert!(store.token("access-family").await.unwrap().unwrap().revoked);
	assert!(matches!(
		store
			.inspect_refresh_token("refresh-family", "client-a", 1)
			.await
			.unwrap(),
		RefreshInspection::Invalid
	));
	assert!(store.token("access-second-family").await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn store_rejects_family_that_does_not_match_the_code() {
	// Arrange
	let store = MemoryOAuthStore::new();
	store.put_code(stored_code("code")).await.unwrap();
	let challenge = challenge(verifier());
	let mut foreign_family = family("family", "code", 2000);
	foreign_family.user_id = "someone-else".into();

	// Act
	let result = store
		.redeem_code_and_store_tokens(CodeRedemptionWithRefresh {
			redemption: CodeRedemptionRequest {
				digest: "code",
				client_id: "client-a",
				redirect_uri: REDIRECT,
				challenge: &challenge,
				resource: None,
				expect_oidc: false,
				now: 0,
				token: access_token("access", "code", "family"),
			},
			family: foreign_family,
			refresh_token: refresh_record("refresh", "family", 1000),
		})
		.await;

	// Assert: nothing was consumed.
	assert!(result.is_err());
	assert!(!store.code("code").await.unwrap().unwrap().redeemed);
	assert!(store.token("access").await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Replay and revocation
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn replaying_an_authorization_code_revokes_its_family() {
	// Arrange
	let harness = enabled().await;
	let code = harness.code("user-a", &["read"]).await;
	let issued = harness
		.server
		.exchange_code(&code, "client-a", None, REDIRECT, verifier(), None)
		.await
		.unwrap();

	// Act
	let replay = harness
		.server
		.exchange_code(&code, "client-a", None, REDIRECT, verifier(), None)
		.await;

	// Assert
	assert_eq!(replay.unwrap_err(), OAuthError::InvalidGrant);
	assert!(!harness.access_active(&issued.access_token).await);
	assert_eq!(
		harness
			.refresh(&issued.refresh_token.unwrap(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
}

#[rstest]
#[case::current_no_hint(false, None)]
#[case::current_refresh_hint(false, Some("refresh_token"))]
#[case::current_wrong_hint(false, Some("access_token"))]
#[case::current_unknown_hint(false, Some("bogus"))]
#[case::rotated_no_hint(true, None)]
#[case::rotated_refresh_hint(true, Some("refresh_token"))]
#[tokio::test]
async fn revoking_a_refresh_token_revokes_the_family(
	#[case] rotate_first: bool,
	#[case] hint: Option<&str>,
) {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let first_refresh = first.refresh_token.clone().unwrap();
	let (presented, latest) = if rotate_first {
		let second = harness.refresh(&first_refresh, None).await.unwrap();
		(first_refresh, second)
	} else {
		(first_refresh, first.clone())
	};

	// Act
	let result = harness
		.server
		.revoke_with_hint(&presented, "client-a", None, hint)
		.await;

	// Assert
	assert!(result.is_ok());
	assert!(!harness.access_active(&first.access_token).await);
	assert!(!harness.access_active(&latest.access_token).await);
	assert_eq!(
		harness
			.refresh(&latest.refresh_token.unwrap(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
}

#[rstest]
#[case::no_hint(None)]
#[case::matching_hint(Some("access_token"))]
#[case::wrong_hint(Some("refresh_token"))]
#[tokio::test]
async fn revoking_an_access_token_leaves_the_family_alive(#[case] hint: Option<&str>) {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;

	// Act
	harness
		.server
		.revoke_with_hint(&first.access_token, "client-a", None, hint)
		.await
		.unwrap();

	// Assert
	assert!(!harness.access_active(&first.access_token).await);
	assert!(
		harness
			.refresh(&first.refresh_token.unwrap(), None)
			.await
			.is_ok()
	);
}

#[rstest]
#[tokio::test]
async fn another_clients_revocation_has_no_effect() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	let mut other = client(ClientKind::Public);
	other.client_id = "client-b".into();
	harness.server.register_client(other).await.unwrap();

	// Act
	let result = harness
		.server
		.revoke(&refresh_token, "client-b", None)
		.await;

	// Assert
	assert!(result.is_ok());
	assert!(harness.access_active(&first.access_token).await);
	assert!(harness.refresh(&refresh_token, None).await.is_ok());
}

#[rstest]
#[tokio::test]
async fn revoking_an_unknown_token_succeeds() {
	// Arrange
	let harness = enabled().await;

	// Act and assert
	assert!(
		harness
			.server
			.revoke("unknown", "client-a", None)
			.await
			.is_ok()
	);
}

#[rstest]
#[case::revoke_user("revoke_user")]
#[case::retire_user("retire_user")]
#[case::revoke_client("revoke_client")]
#[case::disable_client("disable_client")]
#[tokio::test]
async fn administrative_revocation_ends_refresh_families(#[case] action: &str) {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.clone().unwrap();

	// Act: counts stay access-token counts even though a family was revoked too.
	let (count, refresh_error) = match action {
		"revoke_user" => (
			Some(harness.server.revoke_user("user-a").await.unwrap()),
			OAuthError::InvalidGrant,
		),
		"retire_user" => {
			harness.store.retire_user("user-a").await.unwrap();
			(None, OAuthError::InvalidGrant)
		}
		"revoke_client" => (
			Some(harness.server.revoke_client("client-a").await.unwrap()),
			OAuthError::InvalidGrant,
		),
		"disable_client" => (
			Some(harness.server.disable_client("client-a").await.unwrap()),
			OAuthError::InvalidClient,
		),
		other => unreachable!("unknown action {other}"),
	};

	// Assert
	if let Some(count) = count {
		assert_eq!(count, 1);
	}
	assert!(!harness.access_active(&first.access_token).await);
	assert_eq!(
		harness.refresh(&refresh_token, None).await.unwrap_err(),
		refresh_error
	);
	assert!(matches!(
		harness.inspect(&refresh_token).await,
		RefreshInspection::Invalid
	));
}

#[rstest]
#[tokio::test]
async fn refresh_tokens_are_inactive_for_introspection() {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	let resource_secret = harness
		.server
		.rotate_resource_secret("resource-a")
		.await
		.unwrap();

	// Act
	let introspected = harness
		.server
		.introspect(&refresh_token, "resource-a", &resource_secret)
		.await
		.unwrap();
	let info = harness.server.token_info(&refresh_token).await.unwrap();

	// Assert
	assert!(introspected.is_none());
	assert!(info.is_none());
	assert!(
		harness
			.server
			.introspect(&first.access_token, "resource-a", &resource_secret)
			.await
			.unwrap()
			.is_some()
	);
}

// ---------------------------------------------------------------------------
// HTTP endpoints
// ---------------------------------------------------------------------------

async fn post(
	harness: &Harness,
	endpoint: OAuthEndpoint,
	pairs: &[(&str, &str)],
) -> (StatusCode, serde_json::Value) {
	let mut form = url::form_urlencoded::Serializer::new(String::new());
	for (key, value) in pairs {
		form.append_pair(key, value);
	}
	let response = OAuthHandler::new(harness.server.clone(), endpoint)
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oauth")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.body(Bytes::from(form.finish()))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	let body = if response.body.is_empty() {
		serde_json::Value::Null
	} else {
		serde_json::from_slice(&response.body).unwrap()
	};
	(response.status, body)
}

#[rstest]
#[tokio::test]
async fn token_endpoint_issues_and_rotates_refresh_tokens() {
	// Arrange
	let harness = enabled().await;
	let code = harness.code("user-a", &["read"]).await;

	// Act
	let (status, issued) = post(
		&harness,
		OAuthEndpoint::Token,
		&[
			("grant_type", "authorization_code"),
			("client_id", "client-a"),
			("code", &code),
			("redirect_uri", REDIRECT),
			("code_verifier", verifier()),
		],
	)
	.await;
	let first_refresh = issued["refresh_token"].as_str().unwrap().to_owned();
	let (refresh_status, refreshed) = post(
		&harness,
		OAuthEndpoint::Token,
		&[
			("grant_type", "refresh_token"),
			("client_id", "client-a"),
			("refresh_token", &first_refresh),
			("scope", "read"),
		],
	)
	.await;
	let (reuse_status, reuse) = post(
		&harness,
		OAuthEndpoint::Token,
		&[
			("grant_type", "refresh_token"),
			("client_id", "client-a"),
			("refresh_token", &first_refresh),
		],
	)
	.await;

	// Assert
	assert_eq!(status, StatusCode::OK);
	assert_eq!(refresh_status, StatusCode::OK);
	assert_eq!(refreshed["token_type"], "Bearer");
	assert_eq!(refreshed["scope"], "read");
	assert_ne!(refreshed["refresh_token"].as_str().unwrap(), first_refresh);
	assert_eq!(reuse_status, StatusCode::BAD_REQUEST);
	assert_eq!(reuse["error"], "invalid_grant");
	assert!(reuse.get("error_description").is_none());
	assert!(
		!harness
			.access_active(refreshed["access_token"].as_str().unwrap())
			.await
	);
}

#[rstest]
#[case::missing_token(vec![("grant_type", "refresh_token"), ("client_id", "client-a")], "invalid_request")]
#[case::empty_token(vec![("grant_type", "refresh_token"), ("client_id", "client-a"), ("refresh_token", "")], "invalid_request")]
#[case::unknown_token(vec![("grant_type", "refresh_token"), ("client_id", "client-a"), ("refresh_token", "nope")], "invalid_grant")]
#[tokio::test]
async fn token_endpoint_rejects_malformed_refresh_requests(
	#[case] pairs: Vec<(&str, &str)>,
	#[case] error: &str,
) {
	// Arrange
	let harness = enabled().await;

	// Act
	let (status, body) = post(&harness, OAuthEndpoint::Token, &pairs).await;

	// Assert
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert_eq!(body["error"], error);
}

#[rstest]
#[tokio::test]
async fn token_endpoint_reports_unsupported_refresh_grant_when_disabled() {
	// Arrange
	let harness = harness(None, true).await;

	// Act
	let (status, body) = post(
		&harness,
		OAuthEndpoint::Token,
		&[
			("grant_type", "refresh_token"),
			("client_id", "client-a"),
			("refresh_token", "anything"),
		],
	)
	.await;

	// Assert
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert_eq!(body["error"], "unsupported_grant_type");
}

#[rstest]
#[tokio::test]
async fn token_endpoint_omits_refresh_token_unless_enabled() {
	// Arrange
	let harness = harness(Some(policy()), false).await;
	let code = harness.code("user-a", &["read"]).await;

	// Act
	let (status, issued) = post(
		&harness,
		OAuthEndpoint::Token,
		&[
			("grant_type", "authorization_code"),
			("client_id", "client-a"),
			("code", &code),
			("redirect_uri", REDIRECT),
			("code_verifier", verifier()),
		],
	)
	.await;

	// Assert
	assert_eq!(status, StatusCode::OK);
	assert!(issued.get("refresh_token").is_none());
}

#[rstest]
#[case::refresh_hint(Some("refresh_token"))]
#[case::no_hint(None)]
#[case::unknown_hint(Some("something_else"))]
#[tokio::test]
async fn revocation_endpoint_revokes_the_family_of_a_refresh_token(#[case] hint: Option<&str>) {
	// Arrange
	let harness = enabled().await;
	let first = harness.first_tokens(&["read"]).await;
	let refresh_token = first.refresh_token.unwrap();
	let mut pairs = vec![("client_id", "client-a"), ("token", refresh_token.as_str())];
	if let Some(hint) = hint {
		pairs.push(("token_type_hint", hint));
	}

	// Act
	let (status, _) = post(&harness, OAuthEndpoint::Revocation, &pairs).await;

	// Assert
	assert_eq!(status, StatusCode::OK);
	assert!(!harness.access_active(&first.access_token).await);
	assert_eq!(
		harness.refresh(&refresh_token, None).await.unwrap_err(),
		OAuthError::InvalidGrant
	);
}

#[rstest]
#[case::enabled(true, serde_json::json!(["authorization_code", "client_credentials", "refresh_token"]))]
#[case::disabled(false, serde_json::json!(["authorization_code", "client_credentials"]))]
#[tokio::test]
async fn metadata_advertises_refresh_grant_only_when_enabled(
	#[case] policy_enabled: bool,
	#[case] expected: serde_json::Value,
) {
	// Arrange
	let harness = harness(policy_enabled.then(policy), true).await;
	let handler = OAuthHandler::new(harness.server.clone(), OAuthEndpoint::Metadata);

	// Act
	let response = handler
		.handle(Request::builder().uri("/metadata").build().unwrap())
		.await
		.unwrap();

	// Assert
	assert_eq!(response.status, StatusCode::OK);
	let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
	assert_eq!(body["grant_types_supported"], expected);
}
