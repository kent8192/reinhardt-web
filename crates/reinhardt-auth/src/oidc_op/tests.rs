//! Protocol and state integration coverage for the opt-in issuer.

use super::*;
use crate::oauth2_server::OAuthBrowserSession;
use crate::oauth2_server::{
	ClientKind, ClientRegistration, MemoryOAuthStore, OAuthRateLimiter, OAuthServer,
	OAuthServerConfig,
};
use crate::repository::SimpleUserRepository;
use async_trait::async_trait;
use base64::{
	Engine as _,
	engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use bytes::Bytes;
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Request, Response};
use rsa::{
	BigUint, RsaPrivateKey, RsaPublicKey,
	pkcs1v15::{Signature, VerifyingKey},
	pkcs8::{EncodePrivateKey, LineEnding},
	signature::Verifier,
};
use rstest::rstest;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

const ISSUER: &str = "https://auth.example";
const REDIRECT: &str = "https://rp.example/callback";
const USERINFO: &str = "https://auth.example/oidc/userinfo";
const VERIFIER: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";

struct AllowAll;
#[async_trait]
impl OAuthRateLimiter for AllowAll {
	async fn allow(&self, _: &str) -> bool {
		true
	}
}
#[cfg(feature = "database")]
impl crate::oauth2_server::SharedOAuthRateLimiter for AllowAll {}

struct HostAccounts(Arc<AtomicBool>);
#[async_trait]
impl OidcAccountStatus for HostAccounts {
	async fn is_active(&self, _: &str) -> Result<bool, String> {
		Ok(self.0.load(Ordering::SeqCst))
	}
}

struct TestIssuer {
	provider: Arc<OidcProvider>,
	oauth: Arc<OAuthServer>,
	signer: Arc<RsaPemKeyRing>,
	secret: String,
	active: Arc<AtomicBool>,
}

fn add_key(ring: &RsaPemKeyRing, kid: &str) -> PublicRsaJwk {
	let key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
	let pem = key.to_pkcs8_pem(LineEnding::LF).unwrap();
	ring.add_private_key_pem(kid, pem.as_str()).unwrap()
}

async fn issuer() -> TestIssuer {
	let oauth = Arc::new(
		OAuthServer::for_development(
			OAuthServerConfig::new(
				ISSUER,
				"https://auth.example/oauth/authorize",
				"https://auth.example/oauth/token",
				"https://auth.example/oauth/revoke",
				"https://auth.example/oauth/introspect",
			)
			.unwrap(),
			Arc::new(MemoryOAuthStore::new()),
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	oauth
		.register_resource("oidc-userinfo", USERINFO)
		.await
		.unwrap();
	let secret = oauth
		.register_client(ClientRegistration {
			client_id: "rp-a".into(),
			kind: ClientKind::Confidential,
			secret_hash: None,
			previous_secret_hash: None,
			previous_secret_expires_at: None,
			oidc_enabled: true,
			authorization_code: true,
			client_credentials: false,
			redirect_uris: vec![REDIRECT.into()],
			scopes: vec!["openid".into()],
			default_scopes: vec!["openid".into()],
			audiences: vec![USERINFO.into()],
			default_audience: Some(USERINFO.into()),
			browser_origins: vec![],
			enabled: true,
		})
		.await
		.unwrap()
		.unwrap();
	let active = Arc::new(AtomicBool::new(true));
	let signer = Arc::new(RsaPemKeyRing::new());
	add_key(&signer, "key-a");
	let provider = Arc::new(
		OidcProvider::for_development(
			OidcConfig::new(
				ISSUER,
				"https://auth.example/oidc/authorize",
				"https://auth.example/oidc/token",
				USERINFO,
				"https://auth.example/oidc/jwks",
			)
			.unwrap(),
			oauth.clone(),
			Arc::new(MemoryOidcStore::new()),
			Arc::new(HostAccounts(active.clone())),
			signer.clone(),
		)
		.unwrap(),
	);
	provider.rotate_signing_key("key-a").await.unwrap();
	TestIssuer {
		provider,
		oauth,
		signer,
		secret,
		active,
	}
}

fn request() -> OidcAuthorizationRequest {
	OidcAuthorizationRequest {
		client_id: "rp-a".into(),
		redirect_uri: REDIRECT.into(),
		response_type: "code".into(),
		scope: "openid".into(),
		code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes())),
		code_challenge_method: "S256".into(),
		state: Some("rp-state".into()),
		nonce: Some("rp-nonce".into()),
		prompt: None,
		max_age: None,
	}
}

fn approve() -> OidcAuthorizationDecision {
	OidcAuthorizationDecision::Approve {
		user_id: "user-a".into(),
		auth_time: chrono::Utc::now().timestamp(),
		consented: true,
		consent_prompted: true,
		account_selected: true,
		reauthenticated: true,
	}
}

fn code_from(location: &str) -> String {
	url::Url::parse(location)
		.unwrap()
		.query_pairs()
		.find(|(key, _)| key == "code")
		.unwrap()
		.1
		.into_owned()
}

async fn authorize(test: &TestIssuer) -> String {
	let pending = test
		.provider
		.begin_authorization(request(), "browser-a")
		.await
		.unwrap();
	let redirect = test
		.provider
		.complete_authorization(&pending.id, "browser-a", approve())
		.await
		.unwrap();
	assert!(redirect.contains("state=rp-state"));
	assert!(redirect.contains("iss=https%3A%2F%2Fauth.example"));
	code_from(&redirect)
}

fn verify_id_token(token: &str, public: &PublicRsaJwk) -> Value {
	let parts: Vec<_> = token.split('.').collect();
	assert_eq!(parts.len(), 3);
	let header: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
	assert_eq!(header["alg"], "RS256");
	assert_eq!(header["kid"], public.kid);
	let key = RsaPublicKey::new(
		BigUint::from_bytes_be(&URL_SAFE_NO_PAD.decode(&public.n).unwrap()),
		BigUint::from_bytes_be(&URL_SAFE_NO_PAD.decode(&public.e).unwrap()),
	)
	.unwrap();
	let signature =
		Signature::try_from(URL_SAFE_NO_PAD.decode(parts[2]).unwrap().as_slice()).unwrap();
	VerifyingKey::<Sha256>::new(key)
		.verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
		.unwrap();
	serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap()
}

#[rstest]
#[tokio::test]
async fn code_flow_issues_verifiable_id_token_and_matching_userinfo_subject() {
	let test = issuer().await;
	let code = authorize(&test).await;
	let response = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let jwks = test.provider.jwks().await.unwrap();
	let key = test.signer.public_key("key-a").await.unwrap().unwrap();
	let claims = verify_id_token(&response.id_token, &key);
	let info = test
		.provider
		.userinfo(&response.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claims["iss"], ISSUER);
	assert_eq!(claims["aud"], "rp-a");
	assert_eq!(claims["nonce"], "rp-nonce");
	assert_eq!(claims["sub"], info["sub"]);
	assert_ne!(claims["sub"], "user-a");
	assert_eq!(response.expires_in, 600);
	assert_eq!(jwks["keys"][0]["kid"], "key-a");
	assert!(response.id_token.split('.').count() == 3);
	assert_eq!(
		test.provider
			.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
	assert!(
		test.provider
			.userinfo(&response.access_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[rstest]
#[tokio::test]
async fn invalid_client_redirect_pkce_and_prompt_are_rejected() {
	let test = issuer().await;
	let mut input = request();
	input.redirect_uri = "https://evil.example/callback".into();
	assert!(matches!(
		test.provider.begin_authorization(input, "browser-a").await,
		Err(OidcError::InvalidRequest)
	));
	let mut input = request();
	input.client_id = "missing".into();
	assert!(matches!(
		test.provider.begin_authorization(input, "browser-a").await,
		Err(OidcError::InvalidClient)
	));
	let mut input = request();
	input.prompt = Some("none login".into());
	assert!(matches!(
		test.provider.begin_authorization(input, "browser-a").await,
		Err(OidcError::InvalidRequest)
	));
	let code = authorize(&test).await;
	assert_eq!(
		test.provider
			.exchange_code(&code, "rp-a", &test.secret, REDIRECT, "wrong-verifier")
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
	assert!(
		test.provider
			.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
			.await
			.is_ok()
	);
}

#[rstest]
#[tokio::test]
async fn issuer_configuration_rejects_insecure_or_inconsistent_urls() {
	assert!(matches!(
		OidcConfig::new(
			"http://auth.example",
			"http://auth.example/oidc/authorize",
			"http://auth.example/oidc/token",
			"http://auth.example/oidc/userinfo",
			"http://auth.example/oidc/jwks",
		),
		Err(OidcError::InvalidRequest)
	));
	assert!(matches!(
		OidcConfig::new(
			"https://auth.example/tenant",
			"https://auth.example/oidc/authorize",
			"https://auth.example/oidc/token",
			"https://auth.example/oidc/userinfo",
			"https://auth.example/oidc/jwks",
		),
		Err(OidcError::InvalidRequest)
	));
	assert!(matches!(
		OidcConfig::new(
			ISSUER,
			"https://other.example/oidc/authorize",
			"https://auth.example/oidc/token",
			USERINFO,
			"https://auth.example/oidc/jwks",
		),
		Err(OidcError::InvalidRequest)
	));
	assert!(
		OidcConfig::for_loopback_development(
			"http://127.0.0.1:8000",
			"http://127.0.0.1:8000/oidc/authorize",
			"http://127.0.0.1:8000/oidc/token",
			"http://127.0.0.1:8000/oidc/userinfo",
			"http://127.0.0.1:8000/oidc/jwks",
		)
		.is_ok()
	);
	assert!(matches!(
		OidcConfig::for_loopback_development(
			"http://auth.example",
			"http://auth.example/oidc/authorize",
			"http://auth.example/oidc/token",
			"http://auth.example/oidc/userinfo",
			"http://auth.example/oidc/jwks",
		),
		Err(OidcError::InvalidRequest)
	));
	let test = issuer().await;
	assert!(
		test.provider
			.authorization_error_redirect(
				"rp-a",
				"http://127.0.0.1:3000/callback",
				None,
				OidcError::InvalidRequest,
			)
			.await
			.is_none()
	);
}

#[rstest]
#[tokio::test]
async fn account_disable_and_retirement_revoke_userinfo_without_reusing_subject() {
	let test = issuer().await;
	let code = authorize(&test).await;
	let first = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let first_subject = test
		.provider
		.userinfo(&first.access_token)
		.await
		.unwrap()
		.unwrap()["sub"]
		.clone();
	test.active.store(false, Ordering::SeqCst);
	assert!(
		test.provider
			.userinfo(&first.access_token)
			.await
			.unwrap()
			.is_none()
	);
	let code = authorize_disabled(&test).await;
	assert_eq!(code, OidcError::AccessDenied);
	test.provider.retire_user("user-a").await.unwrap();
	test.active.store(true, Ordering::SeqCst);
	let second_code = authorize(&test).await;
	let second = test
		.provider
		.exchange_code(&second_code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let second_subject = test
		.provider
		.userinfo(&second.access_token)
		.await
		.unwrap()
		.unwrap()["sub"]
		.clone();
	assert_ne!(first_subject, second_subject);
}

async fn authorize_disabled(test: &TestIssuer) -> OidcError {
	let pending = test
		.provider
		.begin_authorization(request(), "browser-a")
		.await
		.unwrap();
	let location = test
		.provider
		.complete_authorization(&pending.id, "browser-a", approve())
		.await
		.unwrap();
	assert!(location.contains("error=access_denied"));
	OidcError::AccessDenied
}

#[rstest]
#[tokio::test]
async fn emergency_key_removal_changes_jwks_but_cached_public_key_still_verifies() {
	let test = issuer().await;
	let code = authorize(&test).await;
	let response = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let cached = test.signer.public_key("key-a").await.unwrap().unwrap();
	add_key(&test.signer, "key-b");
	test.provider.rotate_signing_key("key-b").await.unwrap();
	assert_eq!(
		test.provider.jwks().await.unwrap()["keys"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	test.provider.compromise_signing_key("key-a").await.unwrap();
	let keys = test.provider.jwks().await.unwrap();
	assert_eq!(keys["keys"].as_array().unwrap().len(), 1);
	assert_eq!(keys["keys"][0]["kid"], "key-b");
	assert_eq!(verify_id_token(&response.id_token, &cached)["aud"], "rp-a");
}

struct ApproveInteraction(Arc<OidcProvider>);
#[async_trait]
impl OidcInteraction for ApproveInteraction {
	async fn present(
		&self,
		request: Request,
		pending: OidcPendingHandle,
	) -> reinhardt_core::exception::Result<Response> {
		let session = request.extensions.get::<OAuthBrowserSession>().unwrap();
		let location = self
			.0
			.complete_authorization(&pending.id, &session.0, approve())
			.await
			.unwrap();
		Ok(Response::new(StatusCode::FOUND).with_header("Location", &location))
	}
}

#[rstest]
#[tokio::test]
async fn mounted_handlers_support_mock_rp_login_and_userinfo() {
	let test = issuer().await;
	let discovery = OidcHandler::new(test.provider.clone(), OidcEndpoint::Discovery)
		.handle(
			Request::builder()
				.uri("/.well-known/openid-configuration")
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(discovery.status, StatusCode::OK);
	let metadata: Value = serde_json::from_slice(&discovery.body).unwrap();
	assert_eq!(
		metadata["response_types_supported"],
		serde_json::json!(["code"])
	);
	assert_eq!(metadata["scopes_supported"], serde_json::json!(["openid"]));
	assert_eq!(metadata["request_uri_parameter_supported"], false);

	let query = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("response_type", "code")
		.append_pair("scope", "openid")
		.append_pair("client_id", "rp-a")
		.append_pair("redirect_uri", REDIRECT)
		.append_pair(
			"code_challenge",
			&URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes())),
		)
		.append_pair("code_challenge_method", "S256")
		.append_pair("state", "rp-state")
		.append_pair("nonce", "rp-nonce")
		.finish();
	let authorization = Request::builder()
		.uri(format!("/oidc/authorize?{query}"))
		.build()
		.unwrap();
	authorization
		.extensions
		.insert(OAuthBrowserSession("browser-a".into()));
	let approval = OidcHandler::authorization(
		test.provider.clone(),
		Arc::new(ApproveInteraction(test.provider.clone())),
	)
	.handle(authorization)
	.await
	.unwrap();
	assert_eq!(approval.status, StatusCode::FOUND);
	let location = approval.headers.get("location").unwrap().to_str().unwrap();
	let code = code_from(location);

	let form = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("grant_type", "authorization_code")
		.append_pair("code", &code)
		.append_pair("redirect_uri", REDIRECT)
		.append_pair("code_verifier", VERIFIER)
		.finish();
	let basic = STANDARD.encode(format!("rp-a:{}", test.secret));
	let issued = OidcHandler::new(test.provider.clone(), OidcEndpoint::Token)
		.handle(
			Request::builder()
				.method(Method::POST)
				.uri("/oidc/token")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.header("Authorization", format!("Basic {basic}"))
				.body(Bytes::from(form))
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(issued.status, StatusCode::OK);
	assert_eq!(issued.headers.get("cache-control").unwrap(), "no-store");
	let tokens: Value = serde_json::from_slice(&issued.body).unwrap();
	assert!(tokens.get("refresh_token").is_none());
	let claims = verify_id_token(
		tokens["id_token"].as_str().unwrap(),
		&test.signer.public_key("key-a").await.unwrap().unwrap(),
	);
	let info = OidcHandler::new(test.provider.clone(), OidcEndpoint::UserInfo)
		.handle(
			Request::builder()
				.uri("/oidc/userinfo")
				.header(
					"Authorization",
					format!("Bearer {}", tokens["access_token"].as_str().unwrap()),
				)
				.build()
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(info.status, StatusCode::OK);
	let userinfo: Value = serde_json::from_slice(&info.body).unwrap();
	assert_eq!(userinfo["sub"], claims["sub"]);
}

#[rstest]
#[tokio::test]
async fn independent_relying_party_parses_metadata_and_verifies_id_token() {
	use openidconnect::core::{
		CoreIdToken, CoreIdTokenVerifier, CoreJsonWebKeySet, CoreProviderMetadata,
	};
	use openidconnect::{ClientId, ClientSecret, IssuerUrl, Nonce};
	use std::str::FromStr;

	let test = issuer().await;
	let metadata: CoreProviderMetadata = serde_json::from_value(test.provider.discovery()).unwrap();
	assert_eq!(metadata.issuer().as_str(), ISSUER);
	let jwks: CoreJsonWebKeySet =
		serde_json::from_value(test.provider.jwks().await.unwrap()).unwrap();
	let code = authorize(&test).await;
	let response = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let token = CoreIdToken::from_str(&response.id_token).unwrap();
	let verifier = CoreIdTokenVerifier::new_confidential_client(
		ClientId::new("rp-a".into()),
		ClientSecret::new(test.secret.clone()),
		IssuerUrl::new(ISSUER.into()).unwrap(),
		jwks,
	);
	let claims = token
		.claims(&verifier, &Nonce::new("rp-nonce".into()))
		.unwrap();
	assert!(
		token
			.claims(&verifier, &Nonce::new("wrong-nonce".into()))
			.is_err()
	);
	let userinfo = test
		.provider
		.userinfo(&response.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claims.subject().as_str(), userinfo["sub"].as_str().unwrap());
}

#[rstest]
#[tokio::test]
async fn oauth_token_endpoint_cannot_redeem_oidc_code() {
	let test = issuer().await;
	let code = authorize(&test).await;
	assert_eq!(
		test.oauth
			.exchange_code(
				&code,
				"rp-a",
				Some(&test.secret),
				REDIRECT,
				VERIFIER,
				Some(USERINFO)
			)
			.await
			.unwrap_err(),
		crate::oauth2_server::OAuthError::InvalidGrant
	);
	assert_eq!(
		test.provider
			.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
}

#[rstest]
#[tokio::test]
async fn oauth_completion_cannot_convert_oidc_pending_into_oauth_code() {
	let test = issuer().await;
	let pending = test
		.provider
		.begin_authorization(request(), "browser-a")
		.await
		.unwrap();
	assert_eq!(
		test.oauth
			.complete_authorization(
				&pending.id,
				"browser-a",
				crate::oauth2_server::AuthorizationDecision::Approve {
					user_id: "user-a".into(),
					scopes: vec!["openid".into()],
				},
			)
			.await
			.unwrap_err(),
		crate::oauth2_server::OAuthError::InvalidGrant
	);
	assert_eq!(
		test.provider
			.complete_authorization(&pending.id, "browser-a", approve())
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
}

#[rstest]
#[tokio::test]
async fn prompt_and_max_age_require_host_evidence() {
	let test = issuer().await;
	let cases = [
		(
			"login",
			None,
			true,
			true,
			true,
			false,
			-120,
			OidcError::LoginRequired,
		),
		(
			"none",
			Some(0),
			true,
			true,
			true,
			false,
			-120,
			OidcError::LoginRequired,
		),
		(
			"none",
			Some(0),
			true,
			true,
			true,
			false,
			0,
			OidcError::LoginRequired,
		),
		(
			"none",
			None,
			false,
			true,
			true,
			true,
			0,
			OidcError::ConsentRequired,
		),
		(
			"consent",
			None,
			true,
			false,
			true,
			true,
			0,
			OidcError::ConsentRequired,
		),
		(
			"select_account",
			None,
			true,
			true,
			false,
			true,
			0,
			OidcError::AccountSelectionRequired,
		),
	];
	for (prompt, max_age, consented, consent_prompted, selected, reauthenticated, age, error) in
		cases
	{
		let mut input = request();
		input.prompt = Some(prompt.into());
		input.max_age = max_age;
		let pending = test
			.provider
			.begin_authorization(input, "browser-a")
			.await
			.unwrap();
		let decision = OidcAuthorizationDecision::Approve {
			user_id: "user-a".into(),
			auth_time: chrono::Utc::now().timestamp() + age,
			consented,
			consent_prompted,
			account_selected: selected,
			reauthenticated,
		};
		let redirect = test
			.provider
			.complete_authorization(&pending.id, "browser-a", decision)
			.await
			.unwrap();
		assert!(redirect.contains(&format!("error={}", error.as_str())));
		assert!(redirect.contains("state=rp-state"));
	}
	let mut input = request();
	input.max_age = Some(i64::MAX as u64);
	let pending = test
		.provider
		.begin_authorization(input, "browser-a")
		.await
		.unwrap();
	let redirect = test
		.provider
		.complete_authorization(&pending.id, "browser-a", approve())
		.await
		.unwrap();
	assert!(redirect.contains("code="));
}

#[rstest]
#[tokio::test]
async fn client_rotation_overlap_and_disable_revoke_userinfo() {
	let test = issuer().await;
	let code = authorize(&test).await;
	assert_eq!(
		test.oauth
			.rotate_client_secret_with_overlap("rp-a", std::time::Duration::from_secs(86_401))
			.await
			.unwrap_err(),
		crate::oauth2_server::OAuthError::InvalidRequest
	);
	let rotated = test
		.oauth
		.rotate_client_secret_with_overlap("rp-a", std::time::Duration::from_secs(60))
		.await
		.unwrap();
	let token = test
		.provider
		.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	assert!(
		test.provider
			.userinfo(&token.access_token)
			.await
			.unwrap()
			.is_some()
	);
	test.oauth
		.revoke_previous_client_secret("rp-a")
		.await
		.unwrap();
	let code = authorize(&test).await;
	assert_eq!(
		test.provider
			.exchange_code(&code, "rp-a", &test.secret, REDIRECT, VERIFIER)
			.await
			.unwrap_err(),
		OidcError::InvalidClient
	);
	let fresh = test
		.provider
		.exchange_code(&code, "rp-a", &rotated, REDIRECT, VERIFIER)
		.await
		.unwrap();
	assert!(
		test.provider
			.userinfo(&fresh.access_token)
			.await
			.unwrap()
			.is_some()
	);
	test.oauth.disable_client("rp-a").await.unwrap();
	assert!(
		test.provider
			.userinfo(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		test.provider
			.userinfo(&fresh.access_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn postgres_state_is_shared_and_single_use_across_instances() {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use sqlx::PgPool;
	use testcontainers::runners::AsyncRunner;
	use testcontainers_modules::postgres::Postgres;

	// Arrange: migrate one database and open independent store handles.
	let container = Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	executor
		.apply_migrations(&[PostgresOidcStore::migration()])
		.await
		.unwrap();
	let first = PostgresOidcStore::new(PgPool::connect(&url).await.unwrap());
	let second = PostgresOidcStore::new(PgPool::connect(&url).await.unwrap());

	// Act: concurrent nodes claim a subject and consume the same continuation.
	let (left, right) = tokio::join!(
		first.subject_or_insert("user-a", "opaque-a"),
		second.subject_or_insert("user-a", "opaque-b")
	);
	let subject = left.unwrap();
	let right = right.unwrap();
	assert_eq!(subject, right);
	assert_eq!(
		second.subject("user-a").await.unwrap().as_deref(),
		Some(subject.as_str())
	);
	let pending = OidcPending {
		requested_at: 1,
		client_id: "rp-a".into(),
		redirect_uri: REDIRECT.into(),
		state: Some("state".into()),
		nonce: Some("nonce".into()),
		prompts: vec!["none".into()],
		max_age: Some(60),
		expires_at: i64::MAX,
	};
	first.put_pending("pending-a", pending).await.unwrap();
	let (left, right) = tokio::join!(
		first.take_pending("pending-a"),
		second.take_pending("pending-a")
	);
	assert_eq!(
		usize::from(left.unwrap().is_some()) + usize::from(right.unwrap().is_some()),
		1
	);
	first
		.put_code(OidcCodeContext {
			digest: "digest-a".into(),
			user_id: "user-a".into(),
			client_id: "rp-a".into(),
			nonce: None,
			auth_time: 1,
			expires_at: i64::MAX,
		})
		.await
		.unwrap();
	assert_eq!(
		second.code("digest-a").await.unwrap().unwrap().user_id,
		"user-a"
	);

	let signer = RsaPemKeyRing::new();
	let key_a = add_key(&signer, "key-a");
	let key_b = add_key(&signer, "key-b");
	first.rotate_key(key_a, 100, 960).await.unwrap();
	assert_eq!(
		second.active_key().await.unwrap().unwrap().public.kid,
		"key-a"
	);
	second.rotate_key(key_b, 200, 960).await.unwrap();
	assert_eq!(
		first.active_key().await.unwrap().unwrap().public.kid,
		"key-b"
	);
	assert_eq!(first.public_keys(201).await.unwrap().len(), 2);
	first.compromise_key("key-a").await.unwrap();
	assert_eq!(second.public_keys(201).await.unwrap().len(), 1);
	assert_eq!(
		second.public_keys(201).await.unwrap()[0].public.kid,
		"key-b"
	);

	// Assert: retirement preserves the reservation and migrations roll back.
	second.retire_subject("user-a").await.unwrap();
	assert!(first.subject("user-a").await.unwrap().is_none());
	assert!(first.subject_or_insert("user-b", &subject).await.is_err());
	executor
		.rollback_migrations(&[PostgresOidcStore::migration()])
		.await
		.unwrap();
	let table: (Option<String>,) =
		sqlx::query_as("SELECT to_regclass('oidc_op_subject_reservations')::text")
			.fetch_one(first.pool())
			.await
			.unwrap();
	assert!(table.0.is_none());
}

#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn production_nodes_complete_one_cross_instance_login() {
	use crate::oauth2_server::PostgresOAuthStore;
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use sqlx::PgPool;
	use testcontainers::runners::AsyncRunner;
	use testcontainers_modules::postgres::Postgres;

	let container = Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	executor
		.apply_migrations(&[
			PostgresOAuthStore::migration(),
			PostgresOidcStore::migration(),
		])
		.await
		.unwrap();
	let pool_a = PgPool::connect(&url).await.unwrap();
	let pool_b = PgPool::connect(&url).await.unwrap();
	let oauth_config = || {
		OAuthServerConfig::new(
			ISSUER,
			"https://auth.example/oauth/authorize",
			"https://auth.example/oauth/token",
			"https://auth.example/oauth/revoke",
			"https://auth.example/oauth/introspect",
		)
		.unwrap()
	};
	let oauth_a = Arc::new(
		OAuthServer::for_production(
			oauth_config(),
			PostgresOAuthStore::new(pool_a.clone()),
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	let oauth_b = Arc::new(
		OAuthServer::for_production(
			oauth_config(),
			PostgresOAuthStore::new(pool_b.clone()),
			Arc::new(SimpleUserRepository),
			Arc::new(AllowAll),
		)
		.unwrap(),
	);
	oauth_a
		.register_resource("oidc-userinfo", USERINFO)
		.await
		.unwrap();
	let secret = oauth_a
		.register_client(ClientRegistration {
			client_id: "rp-a".into(),
			kind: ClientKind::Confidential,
			secret_hash: None,
			previous_secret_hash: None,
			previous_secret_expires_at: None,
			oidc_enabled: true,
			authorization_code: true,
			client_credentials: false,
			redirect_uris: vec![REDIRECT.into()],
			scopes: vec!["openid".into()],
			default_scopes: vec!["openid".into()],
			audiences: vec![USERINFO.into()],
			default_audience: Some(USERINFO.into()),
			browser_origins: vec![],
			enabled: true,
		})
		.await
		.unwrap()
		.unwrap();
	let signer = Arc::new(RsaPemKeyRing::new());
	let public = add_key(&signer, "key-a");
	let state_a = PostgresOidcStore::new(pool_a);
	let state_b = PostgresOidcStore::new(pool_b);
	state_a
		.rotate_key(public, chrono::Utc::now().timestamp(), 960)
		.await
		.unwrap();
	let config = || {
		OidcConfig::new(
			ISSUER,
			"https://auth.example/oidc/authorize",
			"https://auth.example/oidc/token",
			USERINFO,
			"https://auth.example/oidc/jwks",
		)
		.unwrap()
	};
	let accounts = Arc::new(HostAccounts(Arc::new(AtomicBool::new(true))));
	let first =
		OidcProvider::for_production(config(), oauth_a, state_a, accounts.clone(), signer.clone())
			.await
			.unwrap();
	let second = OidcProvider::for_production(config(), oauth_b, state_b, accounts, signer.clone())
		.await
		.unwrap();
	let pending = first
		.begin_authorization(request(), "browser-a")
		.await
		.unwrap();
	let redirect = second
		.complete_authorization(&pending.id, "browser-a", approve())
		.await
		.unwrap();
	let code = code_from(&redirect);
	let token = first
		.exchange_code(&code, "rp-a", &secret, REDIRECT, VERIFIER)
		.await
		.unwrap();
	let claims = verify_id_token(
		&token.id_token,
		&signer.public_key("key-a").await.unwrap().unwrap(),
	);
	let userinfo = second.userinfo(&token.access_token).await.unwrap().unwrap();
	assert_eq!(claims["sub"], userinfo["sub"]);
	assert_eq!(
		first
			.exchange_code(&code, "rp-a", &secret, REDIRECT, VERIFIER)
			.await
			.unwrap_err(),
		OidcError::InvalidGrant
	);
	executor
		.rollback_migrations(&[
			PostgresOidcStore::migration(),
			PostgresOAuthStore::migration(),
		])
		.await
		.unwrap();
}
