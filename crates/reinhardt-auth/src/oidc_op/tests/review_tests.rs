//! Regression coverage for OIDC completion and authorization-code retention.

use super::*;
use crate::oauth2_server::{AuthorizationCommit, AuthorizationDecision, StoredCode};

struct FailOnceAccounts(AtomicBool);
#[async_trait]
impl OidcAccountStatus for FailOnceAccounts {
	async fn is_active(&self, _: &str) -> Result<bool, String> {
		if self.0.swap(false, Ordering::SeqCst) {
			Err("account service unavailable".into())
		} else {
			Ok(true)
		}
	}
}

#[rstest]
#[tokio::test]
async fn failed_completion_can_retry_the_same_continuation() {
	// Arrange: fail the host check once, after the continuation has been found.
	let issuer = issuer().await;
	let provider = OidcProvider::for_development(
		issuer.provider.config().clone(),
		issuer.oauth.clone(),
		issuer.state.clone(),
		Arc::new(FailOnceAccounts(AtomicBool::new(true))),
		issuer.signer.clone(),
	)
	.unwrap();
	let pending = provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();

	// Act and assert: a dependency failure does not consume the continuation.
	assert_eq!(
		provider
			.complete_authorization(&pending.id, "session", approve())
			.await
			.unwrap_err(),
		OidcError::ServerError
	);
	let redirect = provider
		.complete_authorization(&pending.id, "session", approve())
		.await
		.unwrap();
	assert!(!code_from(&redirect).is_empty());
	assert_eq!(
		provider
			.complete_authorization(&pending.id, "session", approve())
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
}

fn context_for(pending: &OidcPending, code: &StoredCode) -> OidcCodeContext {
	OidcCodeContext {
		digest: code.digest.clone(),
		user_id: code.user_id.clone(),
		client_id: code.client_id.clone(),
		nonce: pending.nonce.clone(),
		auth_time: chrono::Utc::now().timestamp(),
		expires_at: code.expires_at,
	}
}

#[rstest]
#[tokio::test]
async fn memory_completion_rejects_mismatched_context_without_consuming_pending() {
	// Arrange: prepare a valid code, then tamper with its OIDC nonce binding.
	let issuer = issuer().await;
	let pending = issuer
		.provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();
	let prepared = issuer
		.oauth
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
	let code = prepared.code.as_ref().unwrap();
	let context = context_for(&pending.request, code);
	let mut invalid = context.clone();
	invalid.nonce = Some("different-nonce".into());
	let commit = AuthorizationCommit {
		pending: &prepared.pending,
		code: Some(code),
		now: chrono::Utc::now().timestamp(),
	};

	// Act and assert: validation fails before either store mutates.
	assert!(
		!issuer
			.state
			.complete_authorization(
				issuer.oauth_store.as_ref(),
				commit,
				&pending.request,
				Some(&invalid)
			)
			.await
			.unwrap()
	);
	assert!(
		issuer
			.oauth_store
			.pending(&pending.id)
			.await
			.unwrap()
			.is_some()
	);
	assert!(issuer.state.pending(&pending.id).await.unwrap().is_some());
	assert!(
		issuer
			.oauth_store
			.code(&code.digest)
			.await
			.unwrap()
			.is_none()
	);
	assert!(issuer.state.code(&code.digest).await.unwrap().is_none());

	// Recovery: the exact original continuation can still commit once.
	assert!(
		issuer
			.state
			.complete_authorization(
				issuer.oauth_store.as_ref(),
				commit,
				&pending.request,
				Some(&context)
			)
			.await
			.unwrap()
	);
	assert!(
		!issuer
			.state
			.complete_authorization(
				issuer.oauth_store.as_ref(),
				commit,
				&pending.request,
				Some(&context)
			)
			.await
			.unwrap()
	);
	assert_eq!(
		issuer
			.state
			.code(&code.digest)
			.await
			.unwrap()
			.unwrap()
			.nonce,
		context.nonce
	);
	assert!(
		issuer
			.oauth_store
			.pending(&pending.id)
			.await
			.unwrap()
			.is_none()
	);
	assert!(issuer.state.pending(&pending.id).await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn concurrent_memory_approvals_commit_exactly_one_code_and_context() {
	// Arrange.
	let issuer = issuer().await;
	let pending = issuer
		.provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();

	// Act.
	let (left, right) = tokio::join!(
		issuer
			.provider
			.complete_authorization(&pending.id, "session", approve()),
		issuer
			.provider
			.complete_authorization(&pending.id, "session", approve()),
	);

	// Assert: the successful redirect has both OAuth and OIDC state.
	let (Ok(location), Err(error)) = (if left.is_ok() {
		(left, right)
	} else {
		(right, left)
	}) else {
		panic!("exactly one approval must succeed");
	};
	assert_eq!(error, OidcError::InvalidGrant);
	assert!(
		issuer
			.provider
			.exchange_code(
				&code_from(&location),
				"rp-a",
				&issuer.secret,
				REDIRECT,
				VERIFIER
			)
			.await
			.is_ok()
	);
}

struct RejectSigner(Arc<std::sync::atomic::AtomicUsize>);
#[async_trait]
impl OidcSigner for RejectSigner {
	async fn public_key(&self, _: &str) -> Result<Option<PublicRsaJwk>, String> {
		self.0.fetch_add(1, Ordering::SeqCst);
		Err("signer unavailable".into())
	}
	async fn sign(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, String> {
		self.0.fetch_add(1, Ordering::SeqCst);
		Err("signer unavailable".into())
	}
}

#[rstest]
#[tokio::test]
async fn unauthenticated_oidc_exchange_never_reaches_host_accounts_or_signer() {
	// Arrange: both host dependencies fail if an unauthenticated request reaches them.
	let issuer = issuer().await;
	let pending = issuer
		.provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();
	let code = code_from(
		&issuer
			.provider
			.complete_authorization(&pending.id, "session", approve())
			.await
			.unwrap(),
	);
	let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
	let accounts = Arc::new(FailOnceAccounts(AtomicBool::new(true)));
	let provider = OidcProvider::for_development(
		issuer.provider.config().clone(),
		issuer.oauth.clone(),
		issuer.state.clone(),
		accounts.clone(),
		Arc::new(RejectSigner(calls.clone())),
	)
	.unwrap();

	// Act and assert: existing and nonexistent codes have the same authentication error.
	for presented in [&code, "nonexistent-code"] {
		assert_eq!(
			provider
				.exchange_code(presented, "rp-a", "incorrect-secret", REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidClient
		);
	}
	assert!(accounts.0.load(Ordering::SeqCst));
	assert_eq!(calls.load(Ordering::SeqCst), 0);

	// Recovery: failed authentication did not consume the valid grant.
	assert!(
		issuer
			.provider
			.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.is_ok()
	);
}

struct CompletionBarrierUsers(Arc<tokio::sync::Barrier>);
#[async_trait]
impl crate::UserRepository for CompletionBarrierUsers {
	async fn get_user_by_id(
		&self,
		id: &str,
	) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
		self.0.wait().await;
		crate::UserRepository::get_user_by_id(&SimpleUserRepository, id).await
	}
}

#[rstest]
#[tokio::test]
async fn independent_review_memory_completion_race() {
	let issuer = issuer().await;
	let oauth = Arc::new(
		OAuthServer::for_development(
			issuer.oauth.config().clone(),
			issuer.oauth_store.clone(),
			Arc::new(CompletionBarrierUsers(Arc::new(tokio::sync::Barrier::new(
				2,
			)))),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	let provider = OidcProvider::for_development(
		issuer.provider.config().clone(),
		oauth,
		issuer.state.clone(),
		Arc::new(HostAccounts(issuer.active.clone())),
		issuer.signer.clone(),
	)
	.unwrap();
	let pending = provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();
	let (first, second) = tokio::time::timeout(Duration::from_secs(10), async {
		tokio::join!(
			provider.complete_authorization(&pending.id, "session", approve()),
			provider.complete_authorization(&pending.id, "session", approve()),
		)
	})
	.await
	.expect("host lookups must not hold pending-state locks");
	let results = [first, second];
	assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
	assert_eq!(
		results
			.iter()
			.filter(|r| **r == Err(OidcError::InvalidGrant))
			.count(),
		1
	);
	let code = code_from(results.into_iter().find_map(Result::ok).as_ref().unwrap());
	assert!(
		issuer
			.provider
			.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.is_ok()
	);
}

#[cfg(feature = "database")]
mod postgres {
	use super::*;
	use crate::oauth2_server::PostgresOAuthStore;
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use reinhardt_query::prelude::{
		Alias, Expr, ExprTrait, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder,
	};
	use sqlx::PgPool;
	use testcontainers::{ContainerAsync, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	struct PgIssuer {
		_container: ContainerAsync<Postgres>,
		oauth: Arc<OAuthServer>,
		oauth_store: PostgresOAuthStore,
		state: PostgresOidcStore,
		provider: OidcProvider,
		signer: Arc<RsaPemKeyRing>,
		secret: String,
	}

	async fn pg_issuer() -> PgIssuer {
		let template = issuer().await;
		let container = Postgres::default()
			.start()
			.await
			.expect("PostgreSQL container");
		let port = container.get_host_port_ipv4(5432).await.unwrap();
		let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
		let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
		DatabaseMigrationExecutor::new(connection)
			.apply_migrations(&[
				PostgresOAuthStore::migration(),
				PostgresOidcStore::migration(),
			])
			.await
			.unwrap();
		let oauth_store = PostgresOAuthStore::new(PgPool::connect(&url).await.unwrap());
		oauth_store
			.put_resource(
				template
					.oauth_store
					.resource("oidc-userinfo")
					.await
					.unwrap()
					.unwrap(),
			)
			.await
			.unwrap();
		oauth_store
			.put_client(template.oauth_store.client("rp-a").await.unwrap().unwrap())
			.await
			.unwrap();
		let oauth = Arc::new(
			OAuthServer::for_development(
				template.oauth.config().clone(),
				Arc::new(oauth_store.clone()),
				Arc::new(SimpleUserRepository),
				Arc::new(AllowAll),
			)
			.unwrap(),
		);
		let state = PostgresOidcStore::new(PgPool::connect(&url).await.unwrap());
		let provider = OidcProvider::for_development(
			template.provider.config().clone(),
			oauth.clone(),
			Arc::new(state.clone()),
			Arc::new(HostAccounts(template.active)),
			template.signer.clone(),
		)
		.unwrap();
		provider.rotate_signing_key("key-a").await.unwrap();
		PgIssuer {
			_container: container,
			oauth,
			oauth_store,
			state,
			provider,
			signer: template.signer,
			secret: template.secret,
		}
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_initial_registrations_have_one_winner() {
		let issuer = pg_issuer().await;
		let other = issuer.oauth_store.clone();
		let mut client = issuer.oauth_store.client("rp-a").await.unwrap().unwrap();
		client.client_id = "new-rp".into();
		let mut competing_client = client.clone();
		competing_client.secret_hash = Some("competing".into());
		let (first, second) = tokio::join!(
			issuer.oauth_store.insert_client_if_absent(client.clone()),
			other.insert_client_if_absent(competing_client.clone()),
		);
		assert_eq!(
			[first.unwrap(), second.unwrap()]
				.iter()
				.filter(|won| **won)
				.count(),
			1
		);
		let saved = issuer.oauth_store.client("new-rp").await.unwrap().unwrap();
		assert!(saved == client || saved == competing_client);

		let mut resource = issuer
			.oauth_store
			.resource("oidc-userinfo")
			.await
			.unwrap()
			.unwrap();
		resource.resource_id = "new-resource".into();
		resource.audience = "https://auth.example/new-resource".into();
		let mut competing_resource = resource.clone();
		competing_resource.secret_hash = "competing".into();
		let (first, second) = tokio::join!(
			issuer
				.oauth_store
				.insert_resource_if_absent(resource.clone()),
			other.insert_resource_if_absent(competing_resource.clone()),
		);
		assert_eq!(
			[first.unwrap(), second.unwrap()]
				.iter()
				.filter(|won| **won)
				.count(),
			1
		);
		let saved = issuer
			.oauth_store
			.resource("new-resource")
			.await
			.unwrap()
			.unwrap();
		assert!(saved == resource || saved == competing_resource);
	}

	async fn authorize(provider: &OidcProvider) -> String {
		let pending = provider
			.begin_authorization(request(), "session")
			.await
			.unwrap();
		code_from(
			&provider
				.complete_authorization(&pending.id, "session", approve())
				.await
				.unwrap(),
		)
	}

	async fn count(pool: &PgPool, table: &str) -> i64 {
		let (sql, _) = Query::select()
			.expr(Expr::cust("COUNT(*)"))
			.from(Alias::new(table))
			.build(PostgresQueryBuilder);
		let (count,): (i64,) = sqlx::query_as(&sql).fetch_one(pool).await.unwrap();
		count
	}

	#[rstest]
	#[tokio::test]
	async fn expiry_maintenance_preserves_replay_revocation_and_eventual_cleanup() {
		// Arrange: the access token outlives the authorization code.
		let issuer = pg_issuer().await;
		let code = authorize(&issuer.provider).await;
		let token = issuer
			.provider
			.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.unwrap();
		let digest = hex::encode(Sha256::digest(code.as_bytes()));
		let context = issuer.state.code(&digest).await.unwrap().unwrap();
		let token_digest = hex::encode(Sha256::digest(token.access_token.as_bytes()));
		let token_record = issuer
			.oauth_store
			.token(&token_digest)
			.await
			.unwrap()
			.unwrap();
		assert!(token_record.expires_at > context.expires_at);

		// Act: run the host's two maintenance passes at the code expiry deadline.
		assert_eq!(
			issuer
				.oauth_store
				.purge_expired(context.expires_at)
				.await
				.unwrap(),
			0
		);
		assert_eq!(
			issuer
				.state
				.purge_expired(context.expires_at)
				.await
				.unwrap(),
			0
		);

		// Assert: replay still reaches OAuth's atomic linked-token revocation.
		assert!(issuer.state.code(&digest).await.unwrap().is_some());
		assert_eq!(
			issuer
				.provider
				.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		assert!(
			issuer
				.oauth_store
				.token(&token_digest)
				.await
				.unwrap()
				.unwrap()
				.revoked
		);
		assert!(
			issuer
				.provider
				.userinfo(&token.access_token)
				.await
				.unwrap()
				.is_none()
		);

		// Cleanup: the context is removable once the linked token and OAuth code are gone.
		assert_eq!(
			issuer
				.oauth_store
				.purge_expired(token_record.expires_at)
				.await
				.unwrap(),
			2
		);
		assert_eq!(
			issuer
				.state
				.purge_expired(token_record.expires_at)
				.await
				.unwrap(),
			1
		);
		assert!(issuer.state.code(&digest).await.unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn security_revocation_invalidates_oidc_codes_but_preserves_subject() {
		// Arrange: an issued UserInfo token and an unused authorization code.
		let issuer = pg_issuer().await;
		let used = authorize(&issuer.provider).await;
		let unused = authorize(&issuer.provider).await;
		let subject = issuer.state.subject("user-a").await.unwrap().unwrap();
		let token = issuer
			.provider
			.exchange_code(&used, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.unwrap();

		// Act.
		assert_eq!(issuer.oauth.revoke_user("user-a").await.unwrap(), 1);

		// Assert: the security event invalidates grants without retiring identity.
		assert_eq!(
			issuer.state.subject("user-a").await.unwrap().as_deref(),
			Some(subject.as_str())
		);
		assert!(
			issuer
				.provider
				.userinfo(&token.access_token)
				.await
				.unwrap()
				.is_none()
		);
		assert_eq!(
			issuer
				.provider
				.exchange_code(&unused, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		let fresh = authorize(&issuer.provider).await;
		assert!(
			issuer
				.provider
				.exchange_code(&fresh, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.is_ok()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn independent_review_postgres_completion_race() {
		let issuer = pg_issuer().await;
		let oauth = Arc::new(
			OAuthServer::for_development(
				issuer.oauth.config().clone(),
				Arc::new(issuer.oauth_store.clone()),
				Arc::new(CompletionBarrierUsers(Arc::new(tokio::sync::Barrier::new(
					2,
				)))),
				Arc::new(AllowAll),
			)
			.unwrap(),
		);
		let first = OidcProvider::for_development(
			issuer.provider.config().clone(),
			oauth.clone(),
			Arc::new(issuer.state.clone()),
			Arc::new(HostAccounts(Arc::new(AtomicBool::new(true)))),
			issuer.signer.clone(),
		)
		.unwrap();
		let second = OidcProvider::for_development(
			issuer.provider.config().clone(),
			oauth,
			Arc::new(issuer.state.clone()),
			Arc::new(HostAccounts(Arc::new(AtomicBool::new(true)))),
			issuer.signer.clone(),
		)
		.unwrap();
		let pending = first
			.begin_authorization(request(), "session")
			.await
			.unwrap();
		let (first, second) = tokio::time::timeout(Duration::from_secs(10), async {
			tokio::join!(
				first.complete_authorization(&pending.id, "session", approve()),
				second.complete_authorization(&pending.id, "session", approve()),
			)
		})
		.await
		.expect("host lookups must not hold database pending-state locks");
		let results = [first, second];
		assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
		assert_eq!(
			results
				.iter()
				.filter(|r| **r == Err(OidcError::InvalidGrant))
				.count(),
			1
		);
		let pool = issuer.oauth_store.pool();
		assert_eq!(count(pool, "oauth_server_codes").await, 1);
		assert_eq!(count(pool, "oidc_op_codes").await, 1);
		assert_eq!(count(pool, "oauth_server_pending").await, 0);
		assert_eq!(count(pool, "oidc_op_pending").await, 0);
		let code = code_from(results.into_iter().find_map(Result::ok).as_ref().unwrap());
		assert!(
			issuer
				.provider
				.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.is_ok()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn independent_review_postgres_secret_rotation_race() {
		let issuer = pg_issuer().await;
		let second = OAuthServer::for_development(
			issuer.oauth.config().clone(),
			Arc::new(issuer.oauth_store.clone()),
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap();
		let overlap = Duration::from_secs(60);
		let (first, second) = tokio::join!(
			issuer
				.oauth
				.rotate_client_secret_with_overlap("rp-a", overlap),
			second.rotate_client_secret_with_overlap("rp-a", overlap),
		);
		let secrets: Vec<_> = [first, second].into_iter().filter_map(Result::ok).collect();
		assert!(!secrets.is_empty());
		for secret in secrets {
			let code = authorize(&issuer.provider).await;
			assert!(
				issuer
					.provider
					.exchange_code(&code, "rp-a", &secret, REDIRECT, VERIFIER)
					.await
					.is_ok()
			);
		}
	}

	#[rstest]
	#[tokio::test]
	async fn independent_review_reverse_maintenance_preserves_replay() {
		let issuer = pg_issuer().await;
		let code = authorize(&issuer.provider).await;
		let token = issuer
			.provider
			.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.unwrap();
		let digest = hex::encode(Sha256::digest(code.as_bytes()));
		let context = issuer.state.code(&digest).await.unwrap().unwrap();
		assert_eq!(
			issuer
				.state
				.purge_expired(context.expires_at)
				.await
				.unwrap(),
			0
		);
		assert_eq!(
			issuer
				.oauth_store
				.purge_expired(context.expires_at)
				.await
				.unwrap(),
			0
		);
		assert!(issuer.state.code(&digest).await.unwrap().is_some());
		assert_eq!(
			issuer
				.provider
				.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		assert!(
			issuer
				.provider
				.userinfo(&token.access_token)
				.await
				.unwrap()
				.is_none()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn late_review_postgres_resource_cas_rejects_concurrent_and_stale_updates() {
		let issuer = pg_issuer().await;
		let store = &issuer.oauth_store;
		issuer
			.oauth
			.register_resource("cas-resource", "https://cas.example")
			.await
			.unwrap();
		let expected = store.resource("cas-resource").await.unwrap().unwrap();
		let mut first = expected.clone();
		first.secret_hash = "first".into();
		let mut second = expected.clone();
		second.secret_hash = "second".into();
		let (left, right) = tokio::join!(
			store.compare_and_swap_resource(&expected, first),
			store.compare_and_swap_resource(&expected, second)
		);
		assert_eq!(
			[left.unwrap(), right.unwrap()]
				.into_iter()
				.filter(|won| *won)
				.count(),
			1
		);
		let mut disabled = store.resource("cas-resource").await.unwrap().unwrap();
		disabled.enabled = false;
		store.put_resource(disabled.clone()).await.unwrap();
		assert!(
			!store
				.compare_and_swap_resource(&expected, expected.clone())
				.await
				.unwrap()
		);
		assert_eq!(
			store.resource("cas-resource").await.unwrap(),
			Some(disabled.clone())
		);
		let mut changed_identity = disabled.clone();
		changed_identity.audience = "https://other.example".into();
		assert!(
			store
				.compare_and_swap_resource(&disabled, changed_identity)
				.await
				.is_err()
		);
	}

	struct CloseOnceUsers {
		pool: PgPool,
		close: AtomicBool,
	}
	#[async_trait]
	impl crate::UserRepository for CloseOnceUsers {
		async fn get_user_by_id(
			&self,
			id: &str,
		) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
			if self.close.swap(false, Ordering::SeqCst) {
				self.pool.close().await;
			}
			crate::UserRepository::get_user_by_id(&SimpleUserRepository, id).await
		}
	}

	#[rstest]
	#[tokio::test]
	async fn oidc_storage_failure_does_not_orphan_oauth_code_or_burn_pending() {
		// Arrange: lose the OIDC connection after validation, before code storage.
		let issuer = pg_issuer().await;
		let oauth = Arc::new(
			OAuthServer::for_development(
				issuer.oauth.config().clone(),
				Arc::new(issuer.oauth_store.clone()),
				Arc::new(CloseOnceUsers {
					pool: issuer.state.pool().clone(),
					close: AtomicBool::new(true),
				}),
				Arc::new(AllowAll),
			)
			.unwrap(),
		);
		let provider = OidcProvider::for_development(
			issuer.provider.config().clone(),
			oauth.clone(),
			Arc::new(issuer.state.clone()),
			Arc::new(HostAccounts(Arc::new(AtomicBool::new(true)))),
			issuer.signer.clone(),
		)
		.unwrap();
		let pending = provider
			.begin_authorization(request(), "session")
			.await
			.unwrap();

		// Act.
		assert_eq!(
			provider
				.complete_authorization(&pending.id, "session", approve())
				.await
				.unwrap_err(),
			OidcError::ServerError
		);

		// Assert: both continuations remain, and no undisclosed OAuth code exists.
		let pool = issuer.oauth_store.pool();
		assert_eq!(count(pool, "oauth_server_codes").await, 0);
		assert_eq!(count(pool, "oauth_server_pending").await, 1);
		assert_eq!(count(pool, "oidc_op_pending").await, 1);
		let recovered = OidcProvider::for_development(
			issuer.provider.config().clone(),
			oauth,
			Arc::new(PostgresOidcStore::new(pool.clone())),
			Arc::new(HostAccounts(Arc::new(AtomicBool::new(true)))),
			issuer.signer,
		)
		.unwrap();
		assert!(
			!code_from(
				&recovered
					.complete_authorization(&pending.id, "session", approve())
					.await
					.unwrap()
			)
			.is_empty()
		);
		assert_eq!(count(pool, "oauth_server_codes").await, 1);
		assert_eq!(count(pool, "oauth_server_pending").await, 0);
		assert_eq!(count(pool, "oidc_op_pending").await, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn context_insert_failure_rolls_back_oauth_code_and_both_pending_rows() {
		// Arrange: force the last insert in the combined transaction to collide.
		let issuer = pg_issuer().await;
		let pending = issuer
			.provider
			.begin_authorization(request(), "session")
			.await
			.unwrap();
		let prepared = issuer
			.oauth
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
		let code = prepared.code.as_ref().unwrap();
		let context = context_for(&pending.request, code);
		issuer.state.put_code(context.clone()).await.unwrap();
		let commit = AuthorizationCommit {
			pending: &prepared.pending,
			code: Some(code),
			now: chrono::Utc::now().timestamp(),
		};

		// Act: OAuth's insert and pending consumption must roll back with OIDC's failure.
		assert!(
			issuer
				.state
				.complete_authorization(
					&issuer.oauth_store,
					commit,
					&pending.request,
					Some(&context)
				)
				.await
				.is_err()
		);

		// Assert: no orphan OAuth code and neither pending row has been consumed.
		assert!(
			issuer
				.oauth_store
				.code(&code.digest)
				.await
				.unwrap()
				.is_none()
		);
		assert!(
			issuer
				.oauth_store
				.pending(&pending.id)
				.await
				.unwrap()
				.is_some()
		);
		assert!(issuer.state.pending(&pending.id).await.unwrap().is_some());
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oidc_op_codes"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(code.digest.as_str()))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(&code.digest)
			.execute(issuer.state.pool())
			.await
			.unwrap();
		assert!(
			issuer
				.state
				.complete_authorization(
					&issuer.oauth_store,
					commit,
					&pending.request,
					Some(&context)
				)
				.await
				.unwrap()
		);
		assert_eq!(count(issuer.state.pool(), "oauth_server_codes").await, 1);
		assert_eq!(count(issuer.state.pool(), "oidc_op_codes").await, 1);
		assert_eq!(count(issuer.state.pool(), "oauth_server_pending").await, 0);
		assert_eq!(count(issuer.state.pool(), "oidc_op_pending").await, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn concurrent_postgres_approvals_commit_exactly_one_code_and_context() {
		// Arrange.
		let issuer = pg_issuer().await;
		let pending = issuer
			.provider
			.begin_authorization(request(), "session")
			.await
			.unwrap();

		// Act.
		let (left, right) = tokio::join!(
			issuer
				.provider
				.complete_authorization(&pending.id, "session", approve()),
			issuer
				.provider
				.complete_authorization(&pending.id, "session", approve()),
		);

		// Assert: a single winning transaction owns both code records.
		let (Ok(location), Err(error)) = (if left.is_ok() {
			(left, right)
		} else {
			(right, left)
		}) else {
			panic!("exactly one approval must succeed");
		};
		assert_eq!(error, OidcError::InvalidGrant);
		assert_eq!(count(issuer.state.pool(), "oauth_server_codes").await, 1);
		assert_eq!(count(issuer.state.pool(), "oidc_op_codes").await, 1);
		assert!(
			issuer
				.provider
				.exchange_code(
					&code_from(&location),
					"rp-a",
					&issuer.secret,
					REDIRECT,
					VERIFIER
				)
				.await
				.is_ok()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_client_compare_and_swap_has_one_winner_across_handles() {
		// Arrange: two independent handles read the same client revision.
		let issuer = pg_issuer().await;
		let other = PostgresOAuthStore::new(issuer.state.pool().clone());
		let expected = issuer.oauth_store.client("rp-a").await.unwrap().unwrap();
		let mut left = expected.clone();
		left.secret_hash = Some("left-hash".into());
		let mut right = expected.clone();
		right.secret_hash = Some("right-hash".into());

		// Act.
		let (a, b) = tokio::join!(
			issuer
				.oauth_store
				.compare_and_swap_client(&expected, left.clone()),
			other.compare_and_swap_client(&expected, right.clone()),
		);
		let (a, b) = (a.unwrap(), b.unwrap());

		// Assert: no stale writer reports success or overwrites the winning revision.
		assert_ne!(a, b);
		assert_eq!(
			issuer.oauth_store.client("rp-a").await.unwrap(),
			Some(if a { left } else { right })
		);
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_disable_does_not_revive_old_oidc_authorizations() {
		// Arrange: OIDC continuations and tokens share the OAuth security boundary.
		let issuer = pg_issuer().await;
		let pending = issuer
			.provider
			.begin_authorization(request(), "session")
			.await
			.unwrap();
		let unused = authorize(&issuer.provider).await;
		let used = authorize(&issuer.provider).await;
		let token = issuer
			.provider
			.exchange_code(&used, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.unwrap();
		let registration = issuer.oauth_store.client("rp-a").await.unwrap().unwrap();

		// Act: explicit client re-registration must not undo a prior security boundary.
		assert_eq!(issuer.oauth.disable_client("rp-a").await.unwrap(), 1);
		let secret = issuer
			.oauth
			.register_client(registration)
			.await
			.unwrap()
			.unwrap();

		// Assert: both pending paths and old grants are permanently invalid.
		assert_eq!(
			issuer
				.provider
				.complete_authorization(&pending.id, "session", approve())
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		assert_eq!(
			issuer
				.provider
				.exchange_code(&unused, "rp-a", &secret, REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		assert!(
			issuer
				.provider
				.userinfo(&token.access_token)
				.await
				.unwrap()
				.is_none()
		);
		let fresh = authorize(&issuer.provider).await;
		assert!(
			issuer
				.provider
				.exchange_code(&fresh, "rp-a", &secret, REDIRECT, VERIFIER)
				.await
				.is_ok()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_oidc_replay_revokes_while_resource_is_disabled() {
		// Arrange: issue a UserInfo token, then disable its resource registration.
		let issuer = pg_issuer().await;
		let code = authorize(&issuer.provider).await;
		let token = issuer
			.provider
			.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
			.await
			.unwrap();
		let mut resource = issuer
			.oauth_store
			.resource("oidc-userinfo")
			.await
			.unwrap()
			.unwrap();
		resource.enabled = false;
		issuer
			.oauth_store
			.put_resource(resource.clone())
			.await
			.unwrap();

		// Act: replay with valid credentials and bindings must record revocation first.
		assert_eq!(
			issuer
				.provider
				.exchange_code(&code, "rp-a", &issuer.secret, REDIRECT, VERIFIER)
				.await
				.unwrap_err(),
			OidcError::InvalidGrant
		);
		resource.enabled = true;
		issuer.oauth_store.put_resource(resource).await.unwrap();

		// Assert: resource recovery cannot resurrect the linked token.
		assert!(
			issuer
				.provider
				.userinfo(&token.access_token)
				.await
				.unwrap()
				.is_none()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_plain_oauth_code_insert_failure_preserves_pending_for_retry() {
		use crate::oauth2_server::AuthorizationRequest;
		// Arrange: an ordinary OAuth pending request and a colliding code digest.
		let issuer = pg_issuer().await;
		let mut registration = issuer.oauth_store.client("rp-a").await.unwrap().unwrap();
		registration.oidc_enabled = false;
		registration.scopes = vec!["read".into()];
		registration.default_scopes = vec!["read".into()];
		issuer.oauth.register_client(registration).await.unwrap();
		let pending = issuer
			.oauth
			.begin_authorization(
				AuthorizationRequest {
					client_id: "rp-a".into(),
					redirect_uri: REDIRECT.into(),
					response_type: "code".into(),
					code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes())),
					code_challenge_method: "S256".into(),
					scope: Some("read".into()),
					resource: Some(USERINFO.into()),
					state: None,
				},
				"session",
			)
			.await
			.unwrap();
		let stored = issuer
			.oauth_store
			.pending(&pending.id)
			.await
			.unwrap()
			.unwrap();
		let code = StoredCode {
			digest: "ordinary-oauth-collision".into(),
			client_id: "rp-a".into(),
			redirect_uri: REDIRECT.into(),
			challenge: stored.request.code_challenge.clone(),
			user_id: "user-a".into(),
			scopes: vec!["read".into()],
			audience: USERINFO.into(),
			oidc: false,
			expires_at: chrono::Utc::now().timestamp() + 60,
			redeemed: false,
			replayed: false,
		};
		issuer.oauth_store.put_code(code.clone()).await.unwrap();
		let commit = AuthorizationCommit {
			pending: &stored,
			code: Some(&code),
			now: chrono::Utc::now().timestamp(),
		};

		// Act: the insert fails after acquiring the pending row lock.
		assert!(issuer.oauth_store.complete_pending(commit).await.is_err());

		// Assert and retry: no partial deletion escaped the failed transaction.
		assert_eq!(
			issuer.oauth_store.pending(&pending.id).await.unwrap(),
			Some(stored.clone())
		);
		assert!(
			issuer
				.oauth
				.complete_authorization(
					&pending.id,
					"session",
					AuthorizationDecision::Approve {
						user_id: "user-a".into(),
						scopes: vec!["read".into()],
					}
				)
				.await
				.is_ok()
		);
	}
}

#[rstest]
#[case::duplicate_id(true, "client_id=rp-a", StatusCode::BAD_REQUEST, "invalid_request")]
#[case::duplicate_secret(
	true,
	"client_secret=secret",
	StatusCode::BAD_REQUEST,
	"invalid_request"
)]
#[case::empty_duplicate_id(true, "client_id=", StatusCode::BAD_REQUEST, "invalid_request")]
#[case::body_only_id(false, "client_id=rp-a", StatusCode::UNAUTHORIZED, "invalid_client")]
#[case::body_only_secret(
	false,
	"client_secret=secret",
	StatusCode::UNAUTHORIZED,
	"invalid_client"
)]
#[tokio::test]
async fn late_review_oidc_distinguishes_duplicate_and_missing_client_auth(
	#[case] with_header: bool,
	#[case] body: &str,
	#[case] status: StatusCode,
	#[case] error: &str,
) {
	let test = issuer().await;
	let mut request = Request::builder()
		.method(Method::POST)
		.uri("/oidc/token")
		.header("Content-Type", "application/x-www-form-urlencoded")
		.body(Bytes::from(body.to_owned()));
	if with_header {
		request = request.header(
			"Authorization",
			format!("Basic {}", STANDARD.encode(format!("rp-a:{}", test.secret))),
		);
	}
	let response = OidcHandler::new(test.provider.clone(), OidcEndpoint::Token)
		.handle(request.build().unwrap())
		.await
		.unwrap();
	assert_eq!(response.status, status);
	assert_eq!(
		serde_json::from_slice::<Value>(&response.body).unwrap()["error"],
		error
	);
	assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
}
#[rstest]
#[tokio::test]
async fn late_review_discovery_covers_every_issued_claim() {
	let test = issuer().await;
	let pending = test
		.provider
		.begin_authorization(request(), "session")
		.await
		.unwrap();
	let code = code_from(
		&test
			.provider
			.complete_authorization(&pending.id, "session", approve())
			.await
			.unwrap(),
	);
	let token = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let claims: Value = serde_json::from_slice(
		&URL_SAFE_NO_PAD
			.decode(token.id_token.split('.').nth(1).unwrap())
			.unwrap(),
	)
	.unwrap();
	let metadata = test.provider.discovery();
	let advertised = metadata["claims_supported"].as_array().unwrap();
	for name in claims.as_object().unwrap().keys() {
		assert!(
			advertised.contains(&Value::String(name.clone())),
			"missing claim: {name}"
		);
	}
	assert!(advertised.contains(&Value::String("nonce".into())));
	assert!(!advertised.contains(&Value::String("email".into())));
}
#[test]
fn late_review_auth_full_includes_oidc_op() {
	let manifest = include_str!("../../../Cargo.toml");
	let preset = manifest
		.split("auth-full = [")
		.nth(1)
		.unwrap()
		.split(']')
		.next()
		.unwrap();
	assert!(preset.contains(r#""oidc-op""#));
}
