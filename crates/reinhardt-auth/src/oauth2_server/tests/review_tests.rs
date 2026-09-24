//! Regression coverage for authorization-state review findings.

use super::*;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

const REDIRECT: &str = "https://client.example/callback";

async fn issue_code(server: &OAuthServer, user_id: &str) -> String {
	let pending = server
		.begin_authorization(request(REDIRECT), "session")
		.await
		.unwrap();
	let location = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: user_id.into(),
				scopes: vec!["read".into()],
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

struct UserState(u8);
impl crate::AuthIdentity for UserState {
	fn id(&self) -> String {
		"user-a".into()
	}
	fn is_authenticated(&self) -> bool {
		self.0 != 3
	}
	fn is_admin(&self) -> bool {
		false
	}
	fn is_account_active(&self) -> bool {
		self.0 != 1
	}
}
struct ChangingUsers(Arc<AtomicU8>);
#[async_trait]
impl crate::UserRepository for ChangingUsers {
	async fn get_user_by_id(
		&self,
		_: &str,
	) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
		match self.0.load(Ordering::SeqCst) {
			2 => Ok(None),
			4 => Err("user repository unavailable".into()),
			state => Ok(Some(Box::new(UserState(state)))),
		}
	}
}

#[rstest]
#[case::disabled(1, OAuthError::InvalidGrant)]
#[case::deleted(2, OAuthError::InvalidGrant)]
#[case::unauthenticated(3, OAuthError::InvalidGrant)]
#[case::unavailable(4, OAuthError::ServerError)]
#[tokio::test]
async fn exchange_rechecks_user_without_consuming_code(
	#[case] state: u8,
	#[case] error: OAuthError,
) {
	// Arrange: approval happened while the account was active.
	let users = Arc::new(AtomicU8::new(0));
	let store = Arc::new(MemoryOAuthStore::new());
	let server = OAuthServer::for_development(
		config(),
		store.clone(),
		Arc::new(ChangingUsers(users.clone())),
		Arc::new(AllowAll),
	)
	.unwrap();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	server
		.register_client(client(ClientKind::Public))
		.await
		.unwrap();
	let code = issue_code(&server, "user-a").await;
	users.store(state, Ordering::SeqCst);

	// Act and assert: a changed or unavailable account cannot mint a token.
	let result = server
		.exchange_code(&code, "client-a", None, REDIRECT, verifier(), None)
		.await;
	assert_eq!(result.unwrap_err(), error);
	assert!(
		!store
			.code(&hex::encode(Sha256::digest(code.as_bytes())))
			.await
			.unwrap()
			.unwrap()
			.redeemed
	);

	// Recovery: a transient failure did not burn the authorization code.
	users.store(0, Ordering::SeqCst);
	let token = server
		.exchange_code(&code, "client-a", None, REDIRECT, verifier(), None)
		.await
		.unwrap();
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_some()
	);
}

#[rstest]
#[tokio::test]
async fn security_revocation_invalidates_unused_codes_without_retiring_user() {
	// Arrange: one issued token and two unused grants belonging to different users.
	let (server, _) = setup(ClientKind::Public).await;
	let issued_code = issue_code(&server, "user-a").await;
	let unused = issue_code(&server, "user-a").await;
	let other = issue_code(&server, "user-b").await;
	let token = server
		.exchange_code(&issued_code, "client-a", None, REDIRECT, verifier(), None)
		.await
		.unwrap();

	// Act: a password/security event revokes both existing tokens and old grants.
	assert_eq!(server.revoke_user("user-a").await.unwrap(), 1);
	assert_eq!(server.revoke_user("user-a").await.unwrap(), 0);

	// Assert: unrelated users and newly approved grants remain usable.
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(
		server
			.exchange_code(&unused, "client-a", None, REDIRECT, verifier(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert!(
		server
			.exchange_code(&other, "client-a", None, REDIRECT, verifier(), None)
			.await
			.is_ok()
	);
	let fresh = issue_code(&server, "user-a").await;
	assert!(
		server
			.exchange_code(&fresh, "client-a", None, REDIRECT, verifier(), None)
			.await
			.is_ok()
	);
}

#[rstest]
#[tokio::test]
async fn concurrent_rotations_never_return_an_unusable_secret() {
	// Arrange: both operations initially observe the same registration.
	let (server, _) = setup(ClientKind::Confidential).await;
	let overlap = Duration::from_secs(60);

	// Act: Argon2 runs off-thread, allowing the read/modify/write paths to overlap.
	let (left, right) = tokio::join!(
		server.rotate_client_secret_with_overlap("client-a", overlap),
		server.rotate_client_secret_with_overlap("client-a", overlap),
	);

	// Assert: a stale writer may fail, but every reported success authenticates.
	let successes: Vec<_> = [left, right].into_iter().filter_map(Result::ok).collect();
	assert!(!successes.is_empty());
	for secret in successes {
		assert!(
			server
				.client_credentials("client-a", &secret, None, None)
				.await
				.is_ok()
		);
	}
}

#[cfg(feature = "database")]
#[rstest]
#[case::storage_failure(true, hyper::StatusCode::INTERNAL_SERVER_ERROR, "server_error")]
#[case::unregistered_client(false, hyper::StatusCode::BAD_REQUEST, "invalid_client")]
#[tokio::test]
async fn nonredirectable_authorization_errors_keep_their_http_classification(
	#[case] closed: bool,
	#[case] status: hyper::StatusCode,
	#[case] error: &str,
) {
	use reinhardt_http::{Handler, Request};
	// Arrange: a closed pool fails deterministically without external I/O.
	let store: Arc<dyn OAuthServerStore> = if closed {
		let pool = sqlx::postgres::PgPoolOptions::new()
			.connect_lazy("postgres://postgres:postgres@localhost/postgres")
			.unwrap();
		pool.close().await;
		Arc::new(PostgresOAuthStore::new(pool))
	} else {
		Arc::new(MemoryOAuthStore::new())
	};
	let server = Arc::new(
		OAuthServer::for_development(
			config(),
			store,
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	let handler = OAuthHandler::authorization(server, Arc::new(PresentPending));
	let query = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("response_type", "code")
		.append_pair("client_id", "client-a")
		.append_pair("redirect_uri", REDIRECT)
		.append_pair("code_challenge", &challenge(verifier()))
		.append_pair("code_challenge_method", "S256")
		.finish();
	let request = Request::builder()
		.uri(format!("/oauth/authorize?{query}"))
		.build()
		.unwrap();
	request
		.extensions
		.insert(OAuthBrowserSession("session".into()));

	// Act.
	let response = handler.handle(request).await.unwrap();

	// Assert: no unvalidated redirect and no caching of the error response.
	assert_eq!(response.status, status);
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(&response.body).unwrap()["error"],
		error
	);
	assert!(!response.headers.contains_key("location"));
	assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
	assert_eq!(response.headers.get("pragma").unwrap(), "no-cache");
}

#[rstest]
#[tokio::test]
async fn client_compare_and_swap_rejects_stale_administrative_writes() {
	// Arrange: two administrative operations read the same client revision.
	let store = MemoryOAuthStore::new();
	let expected = client(ClientKind::Confidential);
	store.put_client(expected.clone()).await.unwrap();
	let mut disabled = expected.clone();
	disabled.enabled = false;
	let mut rotated = expected.clone();
	rotated.secret_hash = Some("replacement-hash".into());

	// Act: the disable wins; a stale credential update must not undo it.
	assert!(
		store
			.compare_and_swap_client(&expected, disabled.clone())
			.await
			.unwrap()
	);
	assert!(
		!store
			.compare_and_swap_client(&expected, rotated)
			.await
			.unwrap()
	);

	// Assert.
	assert_eq!(store.client("client-a").await.unwrap(), Some(disabled));
}

#[rstest]
#[tokio::test]
async fn disabling_a_public_client_invalidates_old_pending_and_codes() {
	// Arrange: a pending consent, an unused code, and an issued token predate disable.
	let (server, _) = setup(ClientKind::Public).await;
	let pending = server
		.begin_authorization(request(REDIRECT), "session")
		.await
		.unwrap();
	let unused = issue_code(&server, "user-a").await;
	let used = issue_code(&server, "user-a").await;
	let token = server
		.exchange_code(&used, "client-a", None, REDIRECT, verifier(), None)
		.await
		.unwrap();

	// Act: intentionally re-register the same public identifier after disabling it.
	assert_eq!(server.disable_client("client-a").await.unwrap(), 1);
	server
		.register_client(client(ClientKind::Public))
		.await
		.unwrap();

	// Assert: reactivation cannot revive consent, codes, or tokens from before disable.
	assert_eq!(
		server
			.complete_authorization(
				&pending.id,
				"session",
				AuthorizationDecision::Approve {
					user_id: "user-a".into(),
					scopes: vec!["read".into()],
				}
			)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert_eq!(
		server
			.exchange_code(&unused, "client-a", None, REDIRECT, verifier(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	let fresh = issue_code(&server, "user-a").await;
	assert!(
		server
			.exchange_code(&fresh, "client-a", None, REDIRECT, verifier(), None)
			.await
			.is_ok()
	);
}

#[rstest]
#[case::resource_disabled(false)]
#[case::user_disabled(true)]
#[tokio::test]
async fn replay_revokes_before_mutable_user_or_resource_validation(#[case] disable_user: bool) {
	// Arrange: a delegated token, plus a still-unused code whose retry must be preserved.
	let users = Arc::new(AtomicU8::new(0));
	let store = Arc::new(MemoryOAuthStore::new());
	let server = OAuthServer::for_development(
		config(),
		store.clone(),
		Arc::new(ChangingUsers(users.clone())),
		Arc::new(AllowAll),
	)
	.unwrap();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	server
		.register_client(client(ClientKind::Public))
		.await
		.unwrap();
	let used = issue_code(&server, "user-a").await;
	let unused = issue_code(&server, "user-a").await;
	let token = server
		.exchange_code(&used, "client-a", None, REDIRECT, verifier(), None)
		.await
		.unwrap();
	let mut resource = store.resource("resource-a").await.unwrap().unwrap();
	if disable_user {
		users.store(1, Ordering::SeqCst);
	} else {
		resource.enabled = false;
		store.put_resource(resource.clone()).await.unwrap();
	}

	// Act: only a replay with the correct bindings may revoke the linked token.
	assert!(
		server
			.exchange_code(&unused, "client-a", None, REDIRECT, verifier(), None)
			.await
			.is_err()
	);
	assert_eq!(
		server
			.exchange_code(&used, "client-a", None, REDIRECT, &"x".repeat(64), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	let digest = hex::encode(Sha256::digest(token.access_token.as_bytes()));
	assert!(!store.token(&digest).await.unwrap().unwrap().revoked);
	assert_eq!(
		server
			.exchange_code(&used, "client-a", None, REDIRECT, verifier(), None)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);

	// Assert: reenabling dependencies cannot resurrect a replayed token.
	users.store(0, Ordering::SeqCst);
	resource.enabled = true;
	store.put_resource(resource).await.unwrap();
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		server
			.exchange_code(&unused, "client-a", None, REDIRECT, verifier(), None)
			.await
			.is_ok()
	);
}

#[rstest]
#[case::localhost("http://localhost:3000", true, true)]
#[case::ipv4("http://127.0.0.1:3000", true, true)]
#[case::ipv6("http://[::1]:3000", true, true)]
#[case::non_loopback("http://example.com:3000", true, false)]
#[case::production("http://localhost:3000", false, false)]
#[case::path("http://localhost:3000/path", true, false)]
#[case::credentials("http://user@localhost:3000", true, false)]
#[case::query("http://localhost:3000?x=y", true, false)]
#[case::https("https://client.example", false, true)]
#[tokio::test]
async fn loopback_development_origins_are_explicit_and_cors_usable(
	#[case] origin: &str,
	#[case] development: bool,
	#[case] accepted: bool,
) {
	use reinhardt_http::{Handler, Request};
	// Arrange: HTTP origins are permitted only by an explicitly loopback HTTP issuer.
	let configuration = if development {
		OAuthServerConfig::for_loopback_development(
			"http://localhost:8080",
			"http://localhost:8080/authorize",
			"http://localhost:8080/token",
			"http://localhost:8080/revoke",
			"http://localhost:8080/introspect",
		)
		.unwrap()
	} else {
		config()
	};
	let server = Arc::new(
		OAuthServer::for_development(
			configuration,
			Arc::new(MemoryOAuthStore::new()),
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	let mut registration = client(ClientKind::Public);
	registration.browser_origins = vec![origin.into()];

	// Act and assert: unsafe or non-origin URLs are still rejected.
	assert_eq!(server.register_client(registration).await.is_ok(), accepted);
	if accepted {
		let response = OAuthHandler::new(server, OAuthEndpoint::Token)
			.handle(
				Request::builder()
					.uri("/token")
					.header("Origin", origin)
					.build()
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status, hyper::StatusCode::METHOD_NOT_ALLOWED);
		assert_eq!(
			response.headers.get("access-control-allow-origin").unwrap(),
			origin
		);
	}
}

// Force both rotations to read the same revision, independently of scheduler timing.
struct ResourceReadBarrier {
	inner: Arc<dyn OAuthServerStore>,
	reads: std::sync::atomic::AtomicUsize,
	gate: tokio::sync::Barrier,
}
#[async_trait]
impl OAuthServerStore for ResourceReadBarrier {
	async fn compare_and_swap_resource(
		&self,
		expected: &ResourceRegistration,
		replacement: ResourceRegistration,
	) -> Result<bool, String> {
		self.inner
			.compare_and_swap_resource(expected, replacement)
			.await
	}

	async fn put_client(&self, client: ClientRegistration) -> Result<(), String> {
		self.inner.put_client(client).await
	}
	async fn compare_and_swap_client(
		&self,
		expected: &ClientRegistration,
		replacement: ClientRegistration,
	) -> Result<bool, String> {
		self.inner
			.compare_and_swap_client(expected, replacement)
			.await
	}
	async fn disable_client(&self, client_id: &str) -> Result<Option<u64>, String> {
		self.inner.disable_client(client_id).await
	}
	async fn client(&self, client_id: &str) -> Result<Option<ClientRegistration>, String> {
		self.inner.client(client_id).await
	}
	async fn public_client_for_origin(
		&self,
		origin: &str,
	) -> Result<Option<ClientRegistration>, String> {
		self.inner.public_client_for_origin(origin).await
	}
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String> {
		self.inner.put_resource(resource).await
	}
	async fn resource(&self, resource_id: &str) -> Result<Option<ResourceRegistration>, String> {
		let snapshot = self.inner.resource(resource_id).await?;
		if self.reads.fetch_add(1, Ordering::SeqCst) < 2 {
			self.gate.wait().await;
		}
		Ok(snapshot)
	}
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String> {
		self.inner.resource_for_audience(audience).await
	}
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String> {
		self.inner.put_pending(id, pending).await
	}
	async fn pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		self.inner.pending(id).await
	}
	async fn complete_pending(&self, request: AuthorizationCommit<'_>) -> Result<bool, String> {
		self.inner.complete_pending(request).await
	}
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		oidc: bool,
		now: i64,
	) -> Result<Option<PendingRecord>, String> {
		self.inner.take_pending(id, session_digest, oidc, now).await
	}
	async fn put_code(&self, code: StoredCode) -> Result<(), String> {
		self.inner.put_code(code).await
	}
	async fn code(&self, digest: &str) -> Result<Option<StoredCode>, String> {
		self.inner.code(digest).await
	}
	async fn inspect_code_for_exchange(
		&self,
		request: CodeInspection<'_>,
	) -> Result<Option<StoredCode>, String> {
		self.inner.inspect_code_for_exchange(request).await
	}
	async fn redeem_code_and_store_token(
		&self,
		request: CodeRedemptionRequest<'_>,
	) -> Result<CodeRedemption, String> {
		self.inner.redeem_code_and_store_token(request).await
	}
	async fn put_token(&self, token: StoredToken) -> Result<(), String> {
		self.inner.put_token(token).await
	}
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String> {
		self.inner.token(digest).await
	}
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String> {
		self.inner.revoke_token(digest, client_id).await
	}
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String> {
		self.inner.revoke_user(user_id).await
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		self.inner.retire_user(user_id).await
	}
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String> {
		self.inner.revoke_client(client_id).await
	}
}
#[rstest]
#[tokio::test]
async fn late_review_resource_rotation_has_one_winner_for_a_shared_snapshot() {
	let store = Arc::new(ResourceReadBarrier {
		inner: Arc::new(MemoryOAuthStore::new()),
		reads: std::sync::atomic::AtomicUsize::new(0),
		gate: tokio::sync::Barrier::new(2),
	});
	let server = OAuthServer::for_development(
		config(),
		store,
		Arc::new(SimpleUserRepository),
		Arc::new(AllowAll),
	)
	.unwrap();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	let (left, right) = tokio::time::timeout(Duration::from_secs(10), async {
		tokio::join!(
			server.rotate_resource_secret("resource-a"),
			server.rotate_resource_secret("resource-a")
		)
	})
	.await
	.expect("rotation must not retain a store lock over host or hashing work");
	let results = [left, right];
	assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
	assert_eq!(
		results
			.iter()
			.filter(|result| matches!(result, Err(OAuthError::ServerError)))
			.count(),
		1
	);
	for secret in results.into_iter().filter_map(Result::ok) {
		assert!(
			server
				.introspect("unknown-token", "resource-a", &secret)
				.await
				.is_ok()
		);
	}
}
