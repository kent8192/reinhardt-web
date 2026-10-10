//! PostgreSQL store-contract tests for refresh-token families.
//!
//! Each test starts its own PostgreSQL container and applies the OAuth server
//! migrations through Reinhardt's migration executor, so the tests also cover
//! migration `0003_oauth_server_refresh_tokens` on top of the earlier migrations.

use crate::oauth2_server::{
	ClientKind, ClientRegistration, CodeInspection, CodeRedemption, CodeRedemptionRequest,
	CodeRedemptionWithRefresh, OAuthServerStore, PostgresOAuthStore, RefreshInspection,
	RefreshRotation, RefreshRotationRequest, StoredCode, StoredRefreshToken, StoredToken,
	StoredTokenFamily, TokenPrincipal,
};
use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::DatabaseMigrationExecutor;
use rstest::{fixture, rstest};
use sqlx::PgPool;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

const CLIENT: &str = "client-a";
const OTHER_CLIENT: &str = "client-b";
const USER: &str = "user-a";
const OTHER_USER: &str = "user-b";
const AUDIENCE: &str = "https://api.example";
const REDIRECT: &str = "https://client.example/callback";
const CHALLENGE: &str = "challenge";
/// Time of the initial code redemption in every test.
const ISSUED_AT: i64 = 10;

struct Harness {
	_container: ContainerAsync<Postgres>,
	/// Two independent store instances sharing one database, like two server nodes.
	first: PostgresOAuthStore,
	second: PostgresOAuthStore,
}

async fn start_postgres() -> (ContainerAsync<Postgres>, String) {
	let container = Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	(container, url)
}

#[fixture]
async fn harness() -> Harness {
	let (container, url) = start_postgres().await;
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	executor
		.apply_migrations(&PostgresOAuthStore::migrations())
		.await
		.unwrap();
	Harness {
		_container: container,
		first: PostgresOAuthStore::new(PgPool::connect(&url).await.unwrap()),
		second: PostgresOAuthStore::new(PgPool::connect(&url).await.unwrap()),
	}
}

/// One code redemption that starts a token family.
#[derive(Clone)]
struct Grant {
	code: &'static str,
	family: &'static str,
	refresh: &'static str,
	access: &'static str,
	client: &'static str,
	user: &'static str,
	idle_expires_at: i64,
	absolute_expires_at: i64,
	access_expires_at: i64,
	code_expires_at: i64,
}

impl Default for Grant {
	fn default() -> Self {
		Self {
			code: "code-a",
			family: "family-a",
			refresh: "refresh-0",
			access: "access-0",
			client: CLIENT,
			user: USER,
			idle_expires_at: 1_000,
			absolute_expires_at: 10_000,
			access_expires_at: i64::MAX,
			code_expires_at: i64::MAX,
		}
	}
}

impl Grant {
	fn other() -> Self {
		Self {
			code: "code-b",
			family: "family-b",
			refresh: "refresh-b0",
			access: "access-b0",
			client: OTHER_CLIENT,
			user: OTHER_USER,
			..Self::default()
		}
	}

	fn stored_code(&self) -> StoredCode {
		StoredCode {
			digest: self.code.into(),
			client_id: self.client.into(),
			redirect_uri: REDIRECT.into(),
			challenge: CHALLENGE.into(),
			user_id: self.user.into(),
			scopes: vec!["read".into(), "write".into()],
			audience: AUDIENCE.into(),
			oidc: false,
			expires_at: self.code_expires_at,
			redeemed: false,
			replayed: false,
		}
	}

	fn family(&self) -> StoredTokenFamily {
		StoredTokenFamily {
			family_id: self.family.into(),
			client_id: self.client.into(),
			user_id: self.user.into(),
			scopes: vec!["read".into(), "write".into()],
			audience: AUDIENCE.into(),
			code_digest: self.code.into(),
			created_at: ISSUED_AT,
			absolute_expires_at: self.absolute_expires_at,
			revoked: false,
		}
	}

	fn refresh_token(&self) -> StoredRefreshToken {
		StoredRefreshToken {
			digest: self.refresh.into(),
			family_id: self.family.into(),
			parent_digest: None,
			issued_at: ISSUED_AT,
			idle_expires_at: self.idle_expires_at,
			rotated: false,
		}
	}

	fn access_token(&self) -> StoredToken {
		StoredToken {
			digest: self.access.into(),
			client_id: self.client.into(),
			principal: TokenPrincipal::User(self.user.into()),
			scopes: vec!["read".into(), "write".into()],
			audience: AUDIENCE.into(),
			issued_at: ISSUED_AT,
			expires_at: self.access_expires_at,
			revoked: false,
			code_digest: Some(self.code.into()),
			family_id: Some(self.family.into()),
		}
	}

	fn redemption(&self, challenge: &'static str) -> CodeRedemptionRequest<'_> {
		CodeRedemptionRequest {
			digest: self.code,
			client_id: self.client,
			redirect_uri: REDIRECT,
			challenge,
			resource: None,
			expect_oidc: false,
			now: ISSUED_AT,
			token: self.access_token(),
		}
	}

	fn family_redemption(&self) -> CodeRedemptionWithRefresh<'_> {
		CodeRedemptionWithRefresh {
			redemption: self.redemption(CHALLENGE),
			family: self.family(),
			refresh_token: self.refresh_token(),
		}
	}
}

/// Store the grant's code and redeem it into a token family.
async fn start_family(store: &PostgresOAuthStore, grant: &Grant) {
	store.put_code(grant.stored_code()).await.unwrap();
	let outcome = store
		.redeem_code_and_store_tokens(grant.family_redemption())
		.await
		.unwrap();
	assert!(matches!(outcome, CodeRedemption::Valid(_)));
}

/// Rotate `presented` into `next_refresh` and `next_access`, as the default family's client.
async fn rotate(
	store: &PostgresOAuthStore,
	presented: &str,
	next_refresh: &str,
	next_access: &str,
	now: i64,
) -> RefreshRotation {
	rotate_as(store, CLIENT, presented, next_refresh, next_access, now).await
}

async fn rotate_as(
	store: &PostgresOAuthStore,
	client_id: &str,
	presented: &str,
	next_refresh: &str,
	next_access: &str,
	now: i64,
) -> RefreshRotation {
	let grant = Grant::default();
	store
		.rotate_refresh_token(RefreshRotationRequest {
			presented_digest: presented,
			client_id,
			now,
			replacement: StoredRefreshToken {
				digest: next_refresh.into(),
				family_id: grant.family.into(),
				parent_digest: Some(presented.into()),
				issued_at: now,
				idle_expires_at: now + 1_000,
				rotated: false,
			},
			token: StoredToken {
				digest: next_access.into(),
				issued_at: now,
				code_digest: None,
				..grant.access_token()
			},
		})
		.await
		.unwrap()
}

async fn inspect(store: &PostgresOAuthStore, digest: &str, now: i64) -> RefreshInspection {
	store
		.inspect_refresh_token(digest, CLIENT, now)
		.await
		.unwrap()
}

async fn access_revoked(store: &PostgresOAuthStore, digest: &str) -> bool {
	store.token(digest).await.unwrap().unwrap().revoked
}

async fn count(store: &PostgresOAuthStore, table: &str) -> i64 {
	sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
		.fetch_one(store.pool())
		.await
		.unwrap()
}

async fn table_exists(store: &PostgresOAuthStore, table: &str) -> bool {
	let name: (Option<String>,) = sqlx::query_as("SELECT to_regclass($1)::text")
		.bind(table)
		.fetch_one(store.pool())
		.await
		.unwrap();
	name.0.is_some()
}

fn registration(client_id: &str) -> ClientRegistration {
	let mut client = ClientRegistration::new(client_id, ClientKind::Public);
	client.authorization_code = true;
	client.refresh_token = true;
	client.redirect_uris = vec![REDIRECT.into()];
	client.scopes = vec!["read".into(), "write".into()];
	client.default_scopes = vec!["read".into()];
	client.audiences = vec![AUDIENCE.into()];
	client.default_audience = Some(AUDIENCE.into());
	client.browser_origins = vec!["https://client.example".into()];
	client
}

#[rstest]
#[tokio::test]
async fn migration_0003_applies_on_top_of_earlier_migrations_and_keeps_existing_tokens() {
	// Arrange: a database already migrated to 0002 with a token written before 0003.
	let (_container, url) = start_postgres().await;
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	let initial = executor
		.apply_migrations(&PostgresOAuthStore::migrations()[..2])
		.await
		.unwrap();
	let store = PostgresOAuthStore::new(PgPool::connect(&url).await.unwrap());
	let mut legacy = serde_json::to_value(Grant::default().access_token()).unwrap();
	legacy["code_digest"] = serde_json::Value::Null;
	legacy.as_object_mut().unwrap().remove("family_id");
	sqlx::query(
		"INSERT INTO oauth_server_tokens (digest, payload, client_id, user_id, expires_at) \
		 VALUES ('legacy-token', $1, $2, $3, 9223372036854775807)",
	)
	.bind(legacy)
	.bind(CLIENT)
	.bind(USER)
	.execute(store.pool())
	.await
	.unwrap();

	// Act: apply the full migration list, then apply it again.
	let upgraded = executor
		.apply_migrations(&PostgresOAuthStore::migrations())
		.await
		.unwrap();
	let repeated = executor
		.apply_migrations(&PostgresOAuthStore::migrations())
		.await
		.unwrap();

	// Assert: 0003 is the only new migration, creates its tables and column, and keeps old rows.
	assert_eq!(initial.applied.len(), 2);
	assert_eq!(upgraded.applied.len(), 1);
	assert!(repeated.applied.is_empty());
	assert!(table_exists(&store, "oauth_server_token_families").await);
	assert!(table_exists(&store, "oauth_server_refresh_tokens").await);
	let column: (Option<String>,) = sqlx::query_as(
		"SELECT column_name::text FROM information_schema.columns \
		 WHERE table_name = 'oauth_server_tokens' AND column_name = 'family_id'",
	)
	.fetch_one(store.pool())
	.await
	.unwrap();
	assert_eq!(column.0.as_deref(), Some("family_id"));
	let legacy = store.token("legacy-token").await.unwrap().unwrap();
	assert_eq!(legacy.family_id, None);
	assert!(!legacy.revoked);
	assert_eq!(store.revoke_user(USER).await.unwrap(), 1);
	assert!(access_revoked(&store, "legacy-token").await);
}

#[rstest]
#[tokio::test]
async fn redemption_creates_family_refresh_token_and_linked_access_token(
	#[future] harness: Harness,
) {
	// Arrange
	let harness = harness.await;
	let grant = Grant::default();

	// Act
	start_family(&harness.first, &grant).await;

	// Assert: every node sees the family, its first refresh token, and the linked access token.
	let RefreshInspection::Active(token, family) = inspect(&harness.second, "refresh-0", 20).await
	else {
		panic!("expected an active refresh token");
	};
	assert_eq!(token, grant.refresh_token());
	assert_eq!(family, grant.family());
	let access = harness.second.token("access-0").await.unwrap().unwrap();
	assert_eq!(access.family_id.as_deref(), Some("family-a"));
	assert!(!access.revoked);
}

#[rstest]
#[tokio::test]
async fn failed_redemptions_write_no_family(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	let grant = Grant::default();
	let mut mismatched_scopes = grant.family_redemption();
	mismatched_scopes.family.scopes = vec!["admin".into()];
	let mut wrong_challenge = grant.family_redemption();
	wrong_challenge.redemption.challenge = "wrong";
	harness.first.put_code(grant.stored_code()).await.unwrap();

	// Act
	let challenge = harness
		.first
		.redeem_code_and_store_tokens(wrong_challenge)
		.await
		.unwrap();
	let scopes = harness
		.first
		.redeem_code_and_store_tokens(mismatched_scopes)
		.await
		.unwrap();

	// Assert: nothing was written, and the untouched code can still be redeemed.
	assert!(matches!(challenge, CodeRedemption::Invalid));
	assert!(matches!(scopes, CodeRedemption::Invalid));
	assert_eq!(
		count(&harness.first, "oauth_server_token_families").await,
		0
	);
	assert_eq!(
		count(&harness.first, "oauth_server_refresh_tokens").await,
		0
	);
	assert!(harness.first.token("access-0").await.unwrap().is_none());
	assert!(matches!(
		harness
			.second
			.redeem_code_and_store_tokens(grant.family_redemption())
			.await
			.unwrap(),
		CodeRedemption::Valid(_)
	));
}

#[rstest]
#[tokio::test]
async fn inconsistent_family_redemptions_are_rejected(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	let grant = Grant::default();
	harness.first.put_code(grant.stored_code()).await.unwrap();
	let mut unlinked_token = grant.family_redemption();
	unlinked_token.redemption.token.family_id = Some("other-family".into());
	let mut rotated_refresh = grant.family_redemption();
	rotated_refresh.refresh_token.rotated = true;
	let mut wrong_code = grant.family_redemption();
	wrong_code.family.code_digest = "other-code".into();

	// Act
	let results = [
		harness
			.first
			.redeem_code_and_store_tokens(unlinked_token)
			.await,
		harness
			.first
			.redeem_code_and_store_tokens(rotated_refresh)
			.await,
		harness.first.redeem_code_and_store_tokens(wrong_code).await,
		harness
			.first
			.redeem_code_and_store_token(grant.redemption(CHALLENGE))
			.await,
	];

	// Assert: the three malformed family requests and the family-linked plain request fail.
	assert!(results.iter().all(Result::is_err));
	assert_eq!(
		count(&harness.first, "oauth_server_token_families").await,
		0
	);
	assert!(harness.first.token("access-0").await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn rotation_replaces_the_token_and_marks_the_parent_rotated(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;

	// Act
	let rotation = rotate(&harness.second, "refresh-0", "refresh-1", "access-1", 20).await;

	// Assert: the replacement continues the chain and the new access token is linked to the family.
	assert!(matches!(rotation, RefreshRotation::Rotated));
	let RefreshInspection::Active(token, family) = inspect(&harness.first, "refresh-1", 30).await
	else {
		panic!("expected the replacement to be active");
	};
	assert_eq!(token.parent_digest.as_deref(), Some("refresh-0"));
	assert_eq!(token.idle_expires_at, 1_020);
	assert!(!token.rotated);
	assert_eq!(family, Grant::default().family());
	let access = harness.first.token("access-1").await.unwrap().unwrap();
	assert_eq!(access.family_id.as_deref(), Some("family-a"));
	assert_eq!(access.code_digest, None);
	assert!(!access.revoked);
}

#[rstest]
#[tokio::test]
async fn reusing_a_rotated_token_revokes_the_family_and_its_access_tokens(
	#[future] harness: Harness,
) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	assert!(matches!(
		rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await,
		RefreshRotation::Rotated
	));

	// Act
	let reuse = inspect(&harness.second, "refresh-0", 30).await;

	// Assert: both access tokens and the replacement refresh token are dead.
	assert!(matches!(
		reuse,
		RefreshInspection::Reused { ref family_id } if family_id == "family-a"
	));
	assert!(access_revoked(&harness.first, "access-0").await);
	assert!(access_revoked(&harness.first, "access-1").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-1", 30).await,
		RefreshInspection::Invalid
	));
	assert!(matches!(
		rotate(&harness.first, "refresh-1", "refresh-2", "access-2", 30).await,
		RefreshRotation::Invalid
	));
	assert!(harness.first.token("access-2").await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn rotating_a_rotated_token_reports_reuse_and_revokes_the_family(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;

	// Act: a second rotation of the already rotated token.
	let rotation = rotate(&harness.second, "refresh-0", "refresh-x", "access-x", 30).await;

	// Assert: nothing new is issued and the family is revoked.
	assert!(matches!(
		rotation,
		RefreshRotation::Reused { ref family_id } if family_id == "family-a"
	));
	assert!(harness.first.token("access-x").await.unwrap().is_none());
	assert!(access_revoked(&harness.first, "access-0").await);
	assert!(access_revoked(&harness.first, "access-1").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-1", 30).await,
		RefreshInspection::Invalid
	));
}

#[rstest]
#[tokio::test]
async fn concurrent_rotations_have_exactly_one_winner_and_revoke_the_family(
	#[future] harness: Harness,
) {
	// Arrange: eight rotations of one token race across two store instances.
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	let mut racers = tokio::task::JoinSet::new();
	for index in 0..8 {
		let store = if index % 2 == 0 {
			harness.first.clone()
		} else {
			harness.second.clone()
		};
		racers.spawn(async move {
			rotate(
				&store,
				"refresh-0",
				&format!("refresh-r{index}"),
				&format!("access-r{index}"),
				20,
			)
			.await
		});
	}

	// Act
	let mut outcomes = Vec::new();
	while let Some(outcome) = racers.join_next().await {
		outcomes.push(outcome.unwrap());
	}

	// Assert: rotations serialize on the token row. One succeeds, the next one sees the
	// rotated token and revokes the family, and every later one sees a revoked family.
	let count = |expected: fn(&RefreshRotation) -> bool| {
		outcomes.iter().filter(|outcome| expected(outcome)).count()
	};
	let rotated = count(|outcome| matches!(outcome, RefreshRotation::Rotated));
	let reused = count(|outcome| matches!(outcome, RefreshRotation::Reused { .. }));
	let invalid = count(|outcome| matches!(outcome, RefreshRotation::Invalid));
	assert_eq!((rotated, reused, invalid), (1, 1, 6));
	assert!(access_revoked(&harness.first, "access-0").await);
	let winner_access: Vec<String> =
		sqlx::query_scalar("SELECT digest FROM oauth_server_tokens WHERE digest LIKE 'access-r%'")
			.fetch_all(harness.first.pool())
			.await
			.unwrap();
	assert_eq!(winner_access.len(), 1);
	assert!(access_revoked(&harness.first, &winner_access[0]).await);
	let winner_refresh: Vec<String> = sqlx::query_scalar(
		"SELECT digest FROM oauth_server_refresh_tokens WHERE digest LIKE 'refresh-r%'",
	)
	.fetch_all(harness.first.pool())
	.await
	.unwrap();
	assert_eq!(winner_refresh.len(), 1);
	assert!(matches!(
		inspect(&harness.first, &winner_refresh[0], 30).await,
		RefreshInspection::Invalid
	));
}

#[rstest]
#[tokio::test]
async fn another_clients_token_is_invalid_and_leaves_the_family_alive(#[future] harness: Harness) {
	// Arrange: a rotated token and a current token of the same family.
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;

	// Act: another client presents both the rotated and the current token.
	let inspections = [
		harness
			.first
			.inspect_refresh_token("refresh-0", OTHER_CLIENT, 30)
			.await
			.unwrap(),
		harness
			.first
			.inspect_refresh_token("refresh-1", OTHER_CLIENT, 30)
			.await
			.unwrap(),
	];
	let rotation = rotate_as(
		&harness.first,
		OTHER_CLIENT,
		"refresh-1",
		"refresh-2",
		"access-2",
		30,
	)
	.await;
	let revoked = harness
		.first
		.revoke_refresh_family("refresh-1", OTHER_CLIENT)
		.await
		.unwrap();

	// Assert: nothing happened to the owner's family.
	assert!(
		inspections
			.iter()
			.all(|inspection| matches!(inspection, RefreshInspection::Invalid))
	);
	assert!(matches!(rotation, RefreshRotation::Invalid));
	assert!(!revoked);
	assert!(!access_revoked(&harness.first, "access-1").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-1", 30).await,
		RefreshInspection::Active(..)
	));
	assert!(harness.first.token("access-2").await.unwrap().is_none());
}

#[rstest]
#[case::before_idle_expiry(100, 1_000, 99, true)]
#[case::at_idle_expiry(100, 1_000, 100, false)]
#[case::after_idle_expiry(100, 1_000, 500, false)]
#[case::before_absolute_expiry(1_000, 500, 499, true)]
#[case::at_absolute_expiry(1_000, 500, 500, false)]
#[case::after_absolute_expiry(1_000, 500, 800, false)]
#[tokio::test]
async fn refresh_tokens_expire_by_idle_and_absolute_deadline(
	#[future] harness: Harness,
	#[case] idle_expires_at: i64,
	#[case] absolute_expires_at: i64,
	#[case] now: i64,
	#[case] live: bool,
) {
	// Arrange
	let harness = harness.await;
	start_family(
		&harness.first,
		&Grant {
			idle_expires_at,
			absolute_expires_at,
			..Grant::default()
		},
	)
	.await;

	// Act
	let inspection = inspect(&harness.second, "refresh-0", now).await;
	let rotation = rotate(&harness.second, "refresh-0", "refresh-1", "access-1", now).await;

	// Assert: expiry is inclusive, and an expired rotation writes nothing and revokes nothing.
	assert_eq!(matches!(inspection, RefreshInspection::Active(..)), live);
	assert_eq!(matches!(rotation, RefreshRotation::Rotated), live);
	assert_eq!(
		harness.first.token("access-1").await.unwrap().is_some(),
		live
	);
	assert!(!access_revoked(&harness.first, "access-0").await);
	if !live {
		assert!(matches!(inspection, RefreshInspection::Invalid));
		assert!(matches!(rotation, RefreshRotation::Invalid));
		assert_eq!(
			count(&harness.first, "oauth_server_refresh_tokens").await,
			1
		);
	}
}

#[rstest]
#[tokio::test]
async fn inconsistent_rotations_are_rejected_without_writes(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	let grant = Grant::default();
	start_family(&harness.first, &grant).await;
	let request =
		|replacement_parent: &'static str, token_family: &'static str| RefreshRotationRequest {
			presented_digest: "refresh-0",
			client_id: CLIENT,
			now: 20,
			replacement: StoredRefreshToken {
				digest: "refresh-1".into(),
				family_id: "family-a".into(),
				parent_digest: Some(replacement_parent.into()),
				issued_at: 20,
				idle_expires_at: 1_020,
				rotated: false,
			},
			token: StoredToken {
				digest: "access-1".into(),
				code_digest: None,
				family_id: Some(token_family.into()),
				..grant.access_token()
			},
		};

	// Act
	let wrong_parent = harness
		.first
		.rotate_refresh_token(request("refresh-9", "family-a"))
		.await;
	let wrong_family = harness
		.first
		.rotate_refresh_token(request("refresh-0", "family-z"))
		.await;

	// Assert: the presented token was not consumed.
	assert!(wrong_parent.is_err());
	assert!(wrong_family.is_err());
	assert!(harness.first.token("access-1").await.unwrap().is_none());
	assert!(matches!(
		rotate(&harness.second, "refresh-0", "refresh-1", "access-1", 20).await,
		RefreshRotation::Rotated
	));
}

#[rstest]
#[case::current_token(false)]
#[case::rotated_token(true)]
#[tokio::test]
async fn revoking_a_refresh_token_revokes_the_whole_family(
	#[future] harness: Harness,
	#[case] present_rotated: bool,
) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;
	let presented = if present_rotated {
		"refresh-0"
	} else {
		"refresh-1"
	};

	// Act
	let revoked = harness
		.second
		.revoke_refresh_family(presented, CLIENT)
		.await
		.unwrap();

	// Assert: the family, both access tokens, and the other refresh token are dead.
	assert!(revoked);
	assert!(access_revoked(&harness.first, "access-0").await);
	assert!(access_revoked(&harness.first, "access-1").await);
	for digest in ["refresh-0", "refresh-1"] {
		assert!(matches!(
			inspect(&harness.first, digest, 30).await,
			RefreshInspection::Invalid
		));
	}
}

#[rstest]
#[tokio::test]
async fn revoking_an_unknown_refresh_token_finds_no_family(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;

	// Act
	let revoked = harness
		.first
		.revoke_refresh_family("access-0", CLIENT)
		.await
		.unwrap();

	// Assert: an access token digest is not a refresh token, and the family survives.
	assert!(!revoked);
	assert!(!access_revoked(&harness.first, "access-0").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-0", 30).await,
		RefreshInspection::Active(..)
	));
}

#[rstest]
#[case::inspection(0)]
#[case::plain_redemption(1)]
#[case::family_redemption(2)]
#[tokio::test]
async fn code_replay_revokes_the_family_and_its_access_tokens(
	#[future] harness: Harness,
	#[case] replay_path: u8,
) {
	// Arrange: a family that already issued a second access token.
	let harness = harness.await;
	let grant = Grant::default();
	start_family(&harness.first, &grant).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;

	// Act: the redeemed code is presented again through each replay path.
	match replay_path {
		0 => {
			let inspected = harness
				.second
				.inspect_code_for_exchange(CodeInspection {
					digest: grant.code,
					client_id: CLIENT,
					redirect_uri: REDIRECT,
					challenge: CHALLENGE,
					resource: None,
					expect_oidc: false,
					now: 30,
				})
				.await
				.unwrap();
			assert!(inspected.is_none());
		}
		1 => {
			let outcome = harness
				.second
				.redeem_code_and_store_token(CodeRedemptionRequest {
					token: StoredToken {
						digest: "replay-access".into(),
						family_id: None,
						..grant.access_token()
					},
					now: 30,
					..grant.redemption(CHALLENGE)
				})
				.await
				.unwrap();
			assert!(matches!(outcome, CodeRedemption::Replay));
		}
		_ => {
			let replay = Grant {
				family: "family-replay",
				refresh: "refresh-replay",
				access: "access-replay",
				..grant.clone()
			};
			let outcome = harness
				.second
				.redeem_code_and_store_tokens(replay.family_redemption())
				.await
				.unwrap();
			assert!(matches!(outcome, CodeRedemption::Replay));
			assert!(matches!(
				inspect(&harness.first, "refresh-replay", 30).await,
				RefreshInspection::Invalid
			));
		}
	}

	// Assert: the original family is revoked, including the access token without a code link.
	assert!(access_revoked(&harness.first, "access-0").await);
	assert!(access_revoked(&harness.first, "access-1").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-1", 30).await,
		RefreshInspection::Invalid
	));
	assert!(matches!(
		rotate(&harness.first, "refresh-1", "refresh-2", "access-2", 30).await,
		RefreshRotation::Invalid
	));
}

#[derive(Clone, Copy, Debug)]
enum Cutoff {
	RevokeUser,
	RetireUser,
	RevokeClient,
	DisableClient,
}

#[rstest]
#[case::revoke_user(Cutoff::RevokeUser)]
#[case::retire_user(Cutoff::RetireUser)]
#[case::revoke_client(Cutoff::RevokeClient)]
#[case::disable_client(Cutoff::DisableClient)]
#[tokio::test]
async fn administrative_cutoffs_revoke_families_but_count_access_tokens(
	#[future] harness: Harness,
	#[case] cutoff: Cutoff,
) {
	// Arrange: two access tokens in one family, and an unrelated family of another client and user.
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;
	start_family(&harness.first, &Grant::other()).await;
	harness
		.first
		.put_client(registration(CLIENT))
		.await
		.unwrap();

	// Act
	let counted = match cutoff {
		Cutoff::RevokeUser => Some(harness.second.revoke_user(USER).await.unwrap()),
		Cutoff::RetireUser => {
			harness.second.retire_user(USER).await.unwrap();
			None
		}
		Cutoff::RevokeClient => Some(harness.second.revoke_client(CLIENT).await.unwrap()),
		Cutoff::DisableClient => harness.second.disable_client(CLIENT).await.unwrap(),
	};

	// Assert: the refresh chain is dead, and only access tokens are counted.
	if !matches!(cutoff, Cutoff::RetireUser) {
		assert_eq!(counted, Some(2), "{cutoff:?} counts access tokens only");
	}
	assert!(access_revoked(&harness.first, "access-0").await);
	assert!(access_revoked(&harness.first, "access-1").await);
	assert!(matches!(
		inspect(&harness.first, "refresh-1", 30).await,
		RefreshInspection::Invalid
	));
	assert!(matches!(
		rotate(&harness.first, "refresh-1", "refresh-2", "access-2", 30).await,
		RefreshRotation::Invalid
	));
	assert!(harness.first.token("access-2").await.unwrap().is_none());
	assert!(!access_revoked(&harness.first, "access-b0").await);
	assert!(matches!(
		harness
			.first
			.inspect_refresh_token("refresh-b0", OTHER_CLIENT, 30)
			.await
			.unwrap(),
		RefreshInspection::Active(..)
	));
}

#[rstest]
#[tokio::test]
async fn purge_removes_expired_families_and_keeps_codes_of_live_ones(#[future] harness: Harness) {
	// Arrange: family A is past its absolute deadline, family B is live; neither has a live
	// access token, so only family B keeps its (expired) code.
	let harness = harness.await;
	let expired = Grant {
		idle_expires_at: 100,
		absolute_expires_at: 100,
		access_expires_at: 100,
		code_expires_at: 50,
		..Grant::default()
	};
	let live = Grant {
		access_expires_at: 150,
		code_expires_at: 50,
		..Grant::other()
	};
	start_family(&harness.first, &expired).await;
	start_family(&harness.first, &live).await;

	// Act
	let deleted = harness.second.purge_expired(200).await.unwrap();

	// Assert: two access tokens, A's refresh token, A's family, and A's code were deleted.
	assert_eq!(deleted, 5);
	assert_eq!(
		count(&harness.first, "oauth_server_token_families").await,
		1
	);
	assert_eq!(
		count(&harness.first, "oauth_server_refresh_tokens").await,
		1
	);
	assert!(harness.first.code("code-a").await.unwrap().is_none());
	assert!(harness.first.code("code-b").await.unwrap().is_some());
	assert!(matches!(
		inspect(&harness.first, "refresh-0", 200).await,
		RefreshInspection::Invalid
	));
	assert!(matches!(
		harness
			.first
			.inspect_refresh_token("refresh-b0", OTHER_CLIENT, 200)
			.await
			.unwrap(),
		RefreshInspection::Active(..)
	));
}

#[rstest]
#[tokio::test]
async fn purge_keeps_rotated_tokens_until_the_family_expires(#[future] harness: Harness) {
	// Arrange
	let harness = harness.await;
	start_family(&harness.first, &Grant::default()).await;
	rotate(&harness.first, "refresh-0", "refresh-1", "access-1", 20).await;

	// Act
	let before_deadline = harness.second.purge_expired(5_000).await.unwrap();
	let reuse = inspect(&harness.first, "refresh-0", 5_000).await;
	let after_deadline = harness.second.purge_expired(10_000).await.unwrap();

	// Assert: reuse is still detected before the deadline; afterwards the family is gone.
	assert_eq!(before_deadline, 0);
	assert!(matches!(reuse, RefreshInspection::Reused { .. }));
	assert_eq!(after_deadline, 3);
	assert_eq!(
		count(&harness.first, "oauth_server_refresh_tokens").await,
		0
	);
	assert_eq!(
		count(&harness.first, "oauth_server_token_families").await,
		0
	);
	assert!(matches!(
		inspect(&harness.first, "refresh-0", 10_000).await,
		RefreshInspection::Invalid
	));
}
