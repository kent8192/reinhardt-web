//! Regression tests for authorization-server trust boundaries.

use super::protocol::challenge;
use super::*;
use crate::repository::SimpleUserRepository;
use async_trait::async_trait;
use rstest::rstest;
use std::sync::Arc;

struct AllowAll;
#[async_trait]
impl OAuthRateLimiter for AllowAll {
	async fn allow(&self, _: &str) -> bool {
		true
	}
}

fn config() -> OAuthServerConfig {
	OAuthServerConfig::new(
		"https://auth.example",
		"https://auth.example/oauth/authorize",
		"https://auth.example/oauth/token",
		"https://auth.example/oauth/revoke",
		"https://auth.example/oauth/introspect",
	)
	.unwrap()
}
#[rstest]
fn metadata_location_uses_rfc8414_path_insertion() {
	assert_eq!(
		config().metadata_url().unwrap(),
		"https://auth.example/.well-known/oauth-authorization-server"
	);
	let nested = OAuthServerConfig::new(
		"https://auth.example/tenant",
		"https://auth.example/tenant/authorize",
		"https://auth.example/tenant/token",
		"https://auth.example/tenant/revoke",
		"https://auth.example/tenant/introspect",
	)
	.unwrap();
	assert_eq!(
		nested.metadata_url().unwrap(),
		"https://auth.example/.well-known/oauth-authorization-server/tenant"
	);
}
fn server() -> OAuthServer {
	OAuthServer::for_development(
		config(),
		Arc::new(MemoryOAuthStore::new()),
		Arc::new(SimpleUserRepository),
		Arc::new(AllowAll),
	)
	.unwrap()
}
fn client(kind: ClientKind) -> ClientRegistration {
	ClientRegistration {
		client_id: "client-a".into(),
		kind,
		secret_hash: None,
		authorization_code: true,
		client_credentials: kind == ClientKind::Confidential,
		redirect_uris: vec![
			"https://client.example/callback".into(),
			"http://127.0.0.1:3456/callback".into(),
			"http://[::1]:3456/callback".into(),
			"com.example.app:/callback".into(),
		],
		scopes: vec!["read".into(), "write".into()],
		default_scopes: vec!["read".into()],
		audiences: vec!["https://api.example".into()],
		default_audience: Some("https://api.example".into()),
		browser_origins: if kind == ClientKind::Public {
			vec!["https://client.example".into()]
		} else {
			vec![]
		},
		enabled: true,
	}
}
async fn setup(kind: ClientKind) -> (OAuthServer, Option<String>) {
	let server = server();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	let secret = server.register_client(client(kind)).await.unwrap();
	(server, secret)
}
fn request(redirect: &str) -> AuthorizationRequest {
	AuthorizationRequest {
		client_id: "client-a".into(),
		redirect_uri: redirect.into(),
		response_type: "code".into(),
		code_challenge: challenge(
			"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~",
		),
		code_challenge_method: "S256".into(),
		scope: Some("read".into()),
		resource: Some("https://api.example".into()),
		state: Some("state-1".into()),
	}
}
fn verifier() -> &'static str {
	"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~"
}

#[rstest]
#[tokio::test]
async fn code_pkce_scope_replay_and_revocation() {
	let (server, _) = setup(ClientKind::Public).await;
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session-a")
		.await
		.unwrap();
	let wrong_session = server
		.complete_authorization(
			&pending.id,
			"session-b",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await;
	assert_eq!(wrong_session.unwrap_err(), OAuthError::InvalidGrant);
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session-a")
		.await
		.unwrap();
	let redirect = server
		.complete_authorization(
			&pending.id,
			"session-a",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let redirect = url::Url::parse(&redirect).unwrap();
	let code = redirect
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	assert_eq!(
		redirect.query_pairs().find(|(k, _)| k == "iss").unwrap().1,
		"https://auth.example"
	);
	let token = server
		.exchange_code(
			&code,
			"client-a",
			None,
			"https://client.example/callback",
			verifier(),
			Some("https://api.example"),
		)
		.await
		.unwrap();
	let info = server
		.token_info(&token.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(info.principal, TokenPrincipal::User("user-a".into()));
	assert!(info.has_scope("read"));
	assert!(!info.has_scope("write"));
	assert_eq!(
		server
			.exchange_code(
				&code,
				"client-a",
				None,
				"https://client.example/callback",
				verifier(),
				None
			)
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
}

#[rstest]
#[tokio::test]
async fn confidential_client_and_audience_isolation() {
	let (server, secret) = setup(ClientKind::Confidential).await;
	let secret = secret.unwrap();
	assert_eq!(
		server
			.client_credentials("client-a", "wrong", None, None)
			.await
			.unwrap_err(),
		OAuthError::InvalidClient
	);
	let token = server
		.client_credentials("client-a", &secret, None, None)
		.await
		.unwrap();
	let info = server
		.token_info(&token.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(info.principal, TokenPrincipal::Client("client-a".into()));
	assert_eq!(info.scopes, vec!["read"]);
	let resource_secret = server.rotate_resource_secret("resource-a").await.unwrap();
	assert!(
		server
			.introspect(&token.access_token, "resource-a", "wrong")
			.await
			.is_err()
	);
	assert!(
		server
			.introspect(&token.access_token, "resource-a", &resource_secret)
			.await
			.unwrap()
			.is_some()
	);
	server
		.revoke(&token.access_token, "client-a", Some(&secret))
		.await
		.unwrap();
	assert!(
		server
			.introspect(&token.access_token, "resource-a", &resource_secret)
			.await
			.unwrap()
			.is_none()
	);
	let new_secret = server.rotate_client_secret("client-a").await.unwrap();
	assert!(
		server
			.client_credentials("client-a", &secret, None, None)
			.await
			.is_err()
	);
	assert!(
		server
			.client_credentials("client-a", &new_secret, None, None)
			.await
			.is_ok()
	);
}

#[rstest]
#[tokio::test]
async fn registration_and_redirect_boundaries() {
	let (server, _) = setup(ClientKind::Public).await;
	assert!(
		server
			.begin_authorization(request("http://127.0.0.1:9876/callback"), "session")
			.await
			.is_ok()
	);
	assert!(
		server
			.begin_authorization(request("com.example.app:/callback"), "session")
			.await
			.is_ok()
	);
	assert!(
		server
			.begin_authorization(request("http://[::1]:9876/callback"), "session")
			.await
			.is_ok()
	);
	assert!(
		server
			.begin_authorization(request("http://127.0.0.1:9876/other"), "session")
			.await
			.is_err()
	);
	assert!(
		server
			.begin_authorization(request("http://localhost:9876/callback"), "session")
			.await
			.is_err()
	);
	assert!(
		server
			.begin_authorization(request("https://evil.example/callback"), "session")
			.await
			.is_err()
	);
	let mut scoped = request("https://client.example/callback");
	scoped.scope = Some("admin".into());
	assert_eq!(
		server
			.begin_authorization(scoped, "session")
			.await
			.unwrap_err(),
		OAuthError::InvalidScope
	);
	let mut targeted = request("https://client.example/callback");
	targeted.resource = Some("https://evil.example".into());
	assert_eq!(
		server
			.begin_authorization(targeted, "session")
			.await
			.unwrap_err(),
		OAuthError::InvalidTarget
	);
}

#[rstest]
#[tokio::test]
async fn wrong_pkce_cannot_issue_a_token() {
	let (server, _) = setup(ClientKind::Public).await;
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session")
		.await
		.unwrap();
	let redirect = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let code = url::Url::parse(&redirect)
		.unwrap()
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	assert_eq!(
		server
			.exchange_code(
				&code,
				"client-a",
				None,
				"https://client.example/callback",
				"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-other",
				None
			)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert_eq!(
		server
			.exchange_code(
				&code,
				"client-a",
				None,
				"https://client.example/callback",
				verifier(),
				None,
			)
			.await
			.unwrap()
			.token_type,
		"Bearer"
	);
}

#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn postgres_single_use_and_cross_instance_replay() {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use sqlx::PgPool;
	use testcontainers::runners::AsyncRunner;
	use testcontainers_modules::postgres::Postgres;

	// Arrange: two independent stores share a migrated PostgreSQL database.
	let container = Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	executor
		.apply_migrations(&[PostgresOAuthStore::migration()])
		.await
		.unwrap();
	let pool = PgPool::connect(&url).await.unwrap();
	let first = PostgresOAuthStore::new(pool.clone());
	let second = PostgresOAuthStore::new(pool);
	first
		.put_resource(ResourceRegistration {
			resource_id: "resource-a".into(),
			audience: "https://api.example".into(),
			secret_hash: "hash".into(),
			enabled: true,
		})
		.await
		.unwrap();
	first.put_client(client(ClientKind::Public)).await.unwrap();
	assert!(second.resource("resource-a").await.unwrap().is_some());
	assert!(
		second
			.resource_for_audience("https://api.example")
			.await
			.unwrap()
			.is_some()
	);
	assert!(second.client("client-a").await.unwrap().is_some());
	assert!(
		second
			.public_client_for_origin("https://client.example")
			.await
			.unwrap()
			.is_some()
	);
	let mut updated = client(ClientKind::Public);
	updated.enabled = false;
	second.put_client(updated).await.unwrap();
	assert!(!first.client("client-a").await.unwrap().unwrap().enabled);
	first.put_client(client(ClientKind::Public)).await.unwrap();
	let code = StoredCode {
		digest: "code-digest".into(),
		client_id: "client-a".into(),
		redirect_uri: "https://client.example/callback".into(),
		challenge: "challenge".into(),
		user_id: "user-a".into(),
		scopes: vec!["read".into()],
		audience: "https://api.example".into(),
		expires_at: i64::MAX,
		redeemed: false,
		replayed: false,
	};
	first.put_code(code).await.unwrap();
	assert!(matches!(
		first
			.redeem_code(
				"code-digest",
				"client-a",
				"https://client.example/callback",
				"wrong",
				None,
				1,
			)
			.await
			.unwrap(),
		CodeRedemption::Invalid
	));
	let pending = PendingRecord {
		request: PendingAuthorization {
			id: "pending".into(),
			client_id: "client-a".into(),
			redirect_uri: "https://client.example/callback".into(),
			scopes: vec!["read".into()],
			audience: "https://api.example".into(),
			code_challenge: "challenge".into(),
			state: None,
		},
		session_digest: "session-digest".into(),
		expires_at: i64::MAX,
	};
	first.put_pending("pending", pending).await.unwrap();

	// Act: concurrent requests from distinct store instances redeem the same code.
	let (a, b) = tokio::join!(
		first.redeem_code(
			"code-digest",
			"client-a",
			"https://client.example/callback",
			"challenge",
			None,
			1
		),
		second.redeem_code(
			"code-digest",
			"client-a",
			"https://client.example/callback",
			"challenge",
			None,
			1
		)
	);
	let outcomes = [a.unwrap(), b.unwrap()];
	let valid = outcomes
		.iter()
		.filter(|o| matches!(o, CodeRedemption::Valid(_)))
		.count();
	let replay = outcomes
		.iter()
		.filter(|o| matches!(o, CodeRedemption::Replay))
		.count();
	let pending = second.take_pending("pending").await.unwrap();
	let consumed = first.take_pending("pending").await.unwrap();
	second
		.put_token(StoredToken {
			digest: "token-digest".into(),
			client_id: "client-a".into(),
			principal: TokenPrincipal::User("user-a".into()),
			scopes: vec!["read".into()],
			audience: "https://api.example".into(),
			issued_at: 1,
			expires_at: i64::MAX,
			revoked: false,
			code_digest: Some("code-digest".into()),
		})
		.await
		.unwrap();

	// Assert: the replay flag prevents a token written after the replay from becoming active.
	assert_eq!(valid, 1);
	assert_eq!(replay, 1);
	assert!(pending.is_some());
	assert!(consumed.is_none());
	assert!(first.token("token-digest").await.unwrap().unwrap().revoked);
	first
		.put_token(StoredToken {
			digest: "user-token".into(),
			client_id: "client-a".into(),
			principal: TokenPrincipal::User("user-a".into()),
			scopes: vec![],
			audience: "https://api.example".into(),
			issued_at: 1,
			expires_at: i64::MAX,
			revoked: false,
			code_digest: None,
		})
		.await
		.unwrap();
	assert_eq!(second.revoke_user("user-a").await.unwrap(), 1);
	assert!(first.token("user-token").await.unwrap().unwrap().revoked);
	first
		.put_token(StoredToken {
			digest: "client-token".into(),
			client_id: "client-a".into(),
			principal: TokenPrincipal::Client("client-a".into()),
			scopes: vec![],
			audience: "https://api.example".into(),
			issued_at: 1,
			expires_at: i64::MAX,
			revoked: false,
			code_digest: None,
		})
		.await
		.unwrap();
	assert_eq!(second.revoke_client("client-a").await.unwrap(), 1);
	assert!(first.token("client-token").await.unwrap().unwrap().revoked);
	first
		.put_pending(
			"expired-pending",
			PendingRecord {
				request: PendingAuthorization {
					id: "expired-pending".into(),
					client_id: "client-a".into(),
					redirect_uri: "https://client.example/callback".into(),
					scopes: vec![],
					audience: "https://api.example".into(),
					code_challenge: "challenge".into(),
					state: None,
				},
				session_digest: "digest".into(),
				expires_at: 2,
			},
		)
		.await
		.unwrap();
	first
		.put_code(StoredCode {
			digest: "expired-code".into(),
			client_id: "client-a".into(),
			redirect_uri: "https://client.example/callback".into(),
			challenge: "challenge".into(),
			user_id: "user-a".into(),
			scopes: vec![],
			audience: "https://api.example".into(),
			expires_at: 2,
			redeemed: false,
			replayed: false,
		})
		.await
		.unwrap();
	first
		.put_token(StoredToken {
			digest: "expired-token".into(),
			client_id: "client-a".into(),
			principal: TokenPrincipal::Client("client-a".into()),
			scopes: vec![],
			audience: "https://api.example".into(),
			issued_at: 1,
			expires_at: 2,
			revoked: false,
			code_digest: None,
		})
		.await
		.unwrap();
	first
		.put_code(StoredCode {
			digest: "retained-code".into(),
			client_id: "client-a".into(),
			redirect_uri: "https://client.example/callback".into(),
			challenge: "challenge".into(),
			user_id: "user-a".into(),
			scopes: vec![],
			audience: "https://api.example".into(),
			expires_at: 2,
			redeemed: true,
			replayed: false,
		})
		.await
		.unwrap();
	first
		.put_token(StoredToken {
			digest: "live-linked-token".into(),
			client_id: "client-a".into(),
			principal: TokenPrincipal::User("user-a".into()),
			scopes: vec![],
			audience: "https://api.example".into(),
			issued_at: 1,
			expires_at: i64::MAX,
			revoked: false,
			code_digest: Some("retained-code".into()),
		})
		.await
		.unwrap();
	assert_eq!(second.purge_expired(2).await.unwrap(), 3);
	assert!(
		first
			.take_pending("expired-pending")
			.await
			.unwrap()
			.is_none()
	);
	assert!(first.token("expired-token").await.unwrap().is_none());
	assert!(first.token("token-digest").await.unwrap().is_some());

	// Roll back through Reinhardt's migration executor, not a test-only SQL path.
	executor
		.rollback_migrations(&[PostgresOAuthStore::migration()])
		.await
		.unwrap();
	let table: (Option<String>,) =
		sqlx::query_as("SELECT to_regclass('oauth_server_tokens')::text")
			.fetch_one(first.pool())
			.await
			.unwrap();
	assert!(table.0.is_none());
}

struct PresentPending;
#[async_trait]
impl OAuthConsentPresenter for PresentPending {
	async fn present(
		&self,
		_: reinhardt_http::Request,
		pending: PendingAuthorization,
	) -> reinhardt_core::exception::Result<reinhardt_http::Response> {
		reinhardt_http::Response::ok().with_json(&pending)
	}
}

#[rstest]
#[tokio::test]
async fn mounted_http_handlers_expose_protocol_and_registered_cors() {
	use bytes::Bytes;
	use hyper::{Method, StatusCode};
	use reinhardt_http::{Handler, Request};
	use reinhardt_urls::routers::ServerRouter;
	let (server, _) = setup(ClientKind::Public).await;
	let server = Arc::new(server);
	let router = ServerRouter::new()
		.handler_arc(
			"/oauth/authorize",
			Arc::new(OAuthHandler::authorization(
				server.clone(),
				Arc::new(PresentPending),
			)),
		)
		.handler_arc(
			"/oauth/token",
			Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Token)),
		)
		.handler_arc(
			"/oauth/revoke",
			Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Revocation)),
		)
		.handler_arc(
			"/.well-known/oauth-authorization-server",
			Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Metadata)),
		);

	let metadata = router
		.handle(
			Request::builder()
				.uri("/.well-known/oauth-authorization-server")
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(metadata.status, StatusCode::OK);
	let body: serde_json::Value = serde_json::from_slice(&metadata.body).unwrap();
	assert_eq!(
		body["grant_types_supported"],
		serde_json::json!(["authorization_code", "client_credentials"])
	);
	assert_eq!(
		body["code_challenge_methods_supported"],
		serde_json::json!(["S256"])
	);
	let preflight = router
		.handle(
			Request::builder()
				.method(Method::OPTIONS)
				.uri("/oauth/token")
				.header("Origin", "https://client.example")
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(preflight.status, StatusCode::NO_CONTENT);
	assert_eq!(
		preflight
			.headers
			.get("access-control-allow-origin")
			.unwrap(),
		"https://client.example"
	);
	let rejected = router
		.handle(
			Request::builder()
				.method(Method::OPTIONS)
				.uri("/oauth/token")
				.header("Origin", "https://evil.example")
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(rejected.status, StatusCode::FORBIDDEN);
	let invalid_code_form = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("grant_type", "authorization_code")
		.append_pair("client_id", "client-a")
		.append_pair("code", "unused")
		.append_pair("redirect_uri", "https://client.example/callback")
		.finish();
	let invalid_code = router
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oauth/token")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.header("Origin", "https://client.example")
				.body(Bytes::from(invalid_code_form))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(invalid_code.status, StatusCode::BAD_REQUEST);
	assert_eq!(
		invalid_code
			.headers
			.get("access-control-allow-origin")
			.unwrap(),
		"https://client.example"
	);
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(&invalid_code.body).unwrap()["error"],
		"invalid_request"
	);
	for (uri, method, allowed) in [
		("/oauth/token", Method::GET, "POST"),
		(
			"/.well-known/oauth-authorization-server",
			Method::POST,
			"GET",
		),
	] {
		let response = router
			.handle(Request::builder().uri(uri).method(method).build().unwrap())
			.await
			.unwrap();
		assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED);
		assert_eq!(response.headers.get("allow").unwrap(), allowed);
	}

	let auth_query = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("response_type", "code")
		.append_pair("client_id", "client-a")
		.append_pair("redirect_uri", "https://client.example/callback")
		.append_pair("code_challenge", &challenge(verifier()))
		.append_pair("code_challenge_method", "S256")
		.finish();
	let auth = Request::builder()
		.uri(format!("/oauth/authorize?{auth_query}"))
		.build()
		.unwrap();
	auth.extensions
		.insert(OAuthBrowserSession("session".into()));
	let pending = router.handle(auth).await.unwrap();
	assert_eq!(pending.status, StatusCode::OK);
	let pending: PendingAuthorization = serde_json::from_slice(&pending.body).unwrap();
	let location = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let code = url::Url::parse(&location)
		.unwrap()
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	let form = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("grant_type", "authorization_code")
		.append_pair("client_id", "client-a")
		.append_pair("code", &code)
		.append_pair("redirect_uri", "https://client.example/callback")
		.append_pair("code_verifier", verifier())
		.finish();
	let token_response = router
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oauth/token")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.header("Origin", "https://client.example")
				.body(Bytes::from(form))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(token_response.status, StatusCode::OK);
	assert_eq!(
		token_response
			.headers
			.get("access-control-allow-origin")
			.unwrap(),
		"https://client.example"
	);
	assert_eq!(
		token_response.headers.get("cache-control").unwrap(),
		"no-store"
	);
	let token: serde_json::Value = serde_json::from_slice(&token_response.body).unwrap();
	assert!(token.get("refresh_token").is_none());
	let access = token["access_token"].as_str().unwrap();
	let revoke = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("client_id", "client-a")
		.append_pair("token", access)
		.finish();
	let revoked = router
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oauth/revoke")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.body(Bytes::from(revoke))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(revoked.status, StatusCode::OK);
	assert!(server.token_info(access).await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn lowercase_basic_scheme_authenticates_confidential_client() {
	use base64::{Engine as _, engine::general_purpose::STANDARD};
	use bytes::Bytes;
	use hyper::{Method, StatusCode};
	use reinhardt_http::{Handler, Request};

	let (server, secret) = setup(ClientKind::Confidential).await;
	let handler = OAuthHandler::new(Arc::new(server), OAuthEndpoint::Token);
	let credentials = STANDARD.encode(format!("client-a:{}", secret.unwrap()));
	let response = handler
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oauth/token")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.header("Authorization", format!("basic {credentials}"))
				.body(Bytes::from_static(b"grant_type=client_credentials"))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(response.status, StatusCode::OK);
}

struct CaptureLimiter(Arc<tokio::sync::Mutex<Vec<String>>>);
#[async_trait]
impl OAuthRateLimiter for CaptureLimiter {
	async fn allow(&self, key: &str) -> bool {
		self.0.lock().await.push(key.to_owned());
		true
	}
}

#[rstest]
#[tokio::test]
async fn rate_limit_uses_client_ip_from_trusted_proxy() {
	use hyper::StatusCode;
	use reinhardt_http::{Handler, Request, TrustedProxies};
	use std::net::{IpAddr, Ipv4Addr, SocketAddr};

	let keys = Arc::new(tokio::sync::Mutex::new(Vec::new()));
	let server = OAuthServer::for_development(
		config(),
		Arc::new(MemoryOAuthStore::new()),
		Arc::new(SimpleUserRepository),
		Arc::new(CaptureLimiter(keys.clone())),
	)
	.unwrap();
	let proxy: IpAddr = Ipv4Addr::new(10, 0, 0, 1).into();
	let request = Request::builder()
		.uri("/.well-known/oauth-authorization-server")
		.remote_addr(SocketAddr::new(proxy, 8080))
		.header("X-Forwarded-For", "203.0.113.42")
		.header("X-Forwarded-Proto", "https")
		.build()
		.unwrap();
	request.set_trusted_proxies(TrustedProxies::new(vec![proxy]));
	assert!(request.is_secure());
	let response = OAuthHandler::new(Arc::new(server), OAuthEndpoint::Metadata)
		.handle(request)
		.await
		.unwrap();
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(keys.lock().await.as_slice(), &["Metadata:203.0.113.42"]);
}

#[derive(Clone)]
struct MutableUsers(std::sync::Arc<std::sync::atomic::AtomicBool>);
struct MutableIdentity {
	active: bool,
}
impl crate::AuthIdentity for MutableIdentity {
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
		self.active
	}
}
#[async_trait]
impl crate::UserRepository for MutableUsers {
	async fn get_user_by_id(
		&self,
		_: &str,
	) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
		Ok(Some(Box::new(MutableIdentity {
			active: self.0.load(std::sync::atomic::Ordering::SeqCst),
		})))
	}
}

#[rstest]
#[tokio::test]
async fn inactive_user_and_expired_token_are_rejected() {
	use std::sync::atomic::{AtomicBool, Ordering};
	let active = Arc::new(AtomicBool::new(true));
	let mut cfg = config();
	cfg.token_ttl = std::time::Duration::from_secs(1);
	let server = OAuthServer::for_development(
		cfg,
		Arc::new(MemoryOAuthStore::new()),
		Arc::new(MutableUsers(active.clone())),
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
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session")
		.await
		.unwrap();
	let location = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let code = url::Url::parse(&location)
		.unwrap()
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	let token = server
		.exchange_code(
			&code,
			"client-a",
			None,
			"https://client.example/callback",
			verifier(),
			None,
		)
		.await
		.unwrap();
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_some()
	);
	active.store(false, Ordering::SeqCst);
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	active.store(true, Ordering::SeqCst);
	tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[rstest]
#[tokio::test]
async fn revocation_ownership_and_resource_audience_are_enforced() {
	let (server, _) = setup(ClientKind::Public).await;
	let resource_a_secret = server.rotate_resource_secret("resource-a").await.unwrap();
	let resource_b_secret = server
		.register_resource("resource-b", "https://other-api.example")
		.await
		.unwrap();
	let mut other_client = client(ClientKind::Public);
	other_client.client_id = "client-b".into();
	server.register_client(other_client).await.unwrap();
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session")
		.await
		.unwrap();
	let location = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let code = url::Url::parse(&location)
		.unwrap()
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	let token = server
		.exchange_code(
			&code,
			"client-a",
			None,
			"https://client.example/callback",
			verifier(),
			None,
		)
		.await
		.unwrap();
	server
		.revoke(&token.access_token, "client-b", None)
		.await
		.unwrap();
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_some()
	);
	assert!(
		server
			.introspect(&token.access_token, "resource-b", &resource_b_secret)
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		server
			.introspect(&token.access_token, "resource-a", &resource_a_secret)
			.await
			.unwrap()
			.is_some()
	);
	assert_eq!(server.revoke_user("user-a").await.unwrap(), 1);
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[rstest]
#[tokio::test]
async fn pending_and_code_expiry_prevent_issuance() {
	let mut cfg = config();
	cfg.pending_ttl = std::time::Duration::from_secs(1);
	cfg.code_ttl = std::time::Duration::from_secs(1);
	let server = OAuthServer::for_development(
		cfg,
		Arc::new(MemoryOAuthStore::new()),
		Arc::new(SimpleUserRepository),
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
	let expired = server
		.begin_authorization(request("https://client.example/callback"), "session")
		.await
		.unwrap();
	tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
	assert_eq!(
		server
			.complete_authorization(
				&expired.id,
				"session",
				AuthorizationDecision::Approve {
					user_id: "user-a".into(),
					scopes: vec!["read".into()]
				}
			)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
	let pending = server
		.begin_authorization(request("https://client.example/callback"), "session")
		.await
		.unwrap();
	let location = server
		.complete_authorization(
			&pending.id,
			"session",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await
		.unwrap();
	let code = url::Url::parse(&location)
		.unwrap()
		.query_pairs()
		.find(|(k, _)| k == "code")
		.unwrap()
		.1
		.into_owned();
	tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
	assert_eq!(
		server
			.exchange_code(
				&code,
				"client-a",
				None,
				"https://client.example/callback",
				verifier(),
				None
			)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
}
