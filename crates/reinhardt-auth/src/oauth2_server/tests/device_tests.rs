//! RFC 8628 Device Authorization Grant coverage.
//!
//! Store-level scenarios run against both `MemoryOAuthStore` and, with the
//! `database` feature, `PostgresOAuthStore` on a TestContainers PostgreSQL.
//! HTTP-level scenarios use the memory store because the wire behaviour is
//! independent of the store.

use super::*;
use bytes::Bytes;
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Request, Response};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

const VERIFICATION_URI: &str = "https://auth.example/device";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const USER_CODE_ALPHABET: &str = "BCDFGHJKLMNPQRSTVWXZ";
/// Address of the Verification Page requester in tests.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
/// A second requester address, isolated from [`CLIENT_IP`]'s rate limit.
const OTHER_CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9));

fn sha(value: &str) -> String {
	hex::encode(Sha256::digest(value.as_bytes()))
}
fn unix_now() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs() as i64
}
fn device_settings() -> DeviceAuthorizationConfig {
	DeviceAuthorizationConfig::new("https://auth.example/oauth/device", VERIFICATION_URI)
}
fn device_config() -> OAuthServerConfig {
	config()
		.with_device_authorization(device_settings())
		.unwrap()
}
fn device_client(client_id: &str, kind: ClientKind) -> ClientRegistration {
	let mut client = ClientRegistration::new(client_id, kind);
	client.device_code = true;
	client.scopes = vec!["read".into(), "write".into()];
	client.default_scopes = vec!["read".into()];
	client.audiences = vec!["https://api.example".into()];
	client.default_audience = Some("https://api.example".into());
	client
}
async fn server_with(
	store: Arc<dyn OAuthServerStore>,
	config: OAuthServerConfig,
	limiter: Arc<dyn OAuthRateLimiter>,
) -> OAuthServer {
	let server =
		OAuthServer::for_development(config, store, Arc::new(SimpleUserRepository), limiter)
			.unwrap();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	server
}
/// A device-enabled server with public client `tv-app` and a second public client `other-tv`.
async fn device_server(store: Arc<dyn OAuthServerStore>) -> OAuthServer {
	let server = server_with(store, device_config(), Arc::new(AllowAll)).await;
	for id in ["tv-app", "other-tv"] {
		server
			.register_client(device_client(id, ClientKind::Public))
			.await
			.unwrap();
	}
	server
}
async fn issue(server: &OAuthServer) -> DeviceAuthorizationResponse {
	server
		.begin_device_authorization("tv-app", None, Some("read write"), None)
		.await
		.unwrap()
}
async fn approve(
	server: &OAuthServer,
	issued: &DeviceAuthorizationResponse,
	session: &str,
	user_id: &str,
	scopes: &[&str],
) {
	let pending = server
		.lookup_device_user_code(&issued.user_code, session, CLIENT_IP)
		.await
		.unwrap();
	server
		.complete_device_authorization(
			&pending.id,
			session,
			AuthorizationDecision::Approve {
				user_id: user_id.into(),
				scopes: scopes.iter().map(|scope| (*scope).to_owned()).collect(),
			},
		)
		.await
		.unwrap();
}
async fn poll(
	server: &OAuthServer,
	issued: &DeviceAuthorizationResponse,
) -> Result<IssuedToken, OAuthError> {
	server
		.exchange_device_code(&issued.device_code, "tv-app", None)
		.await
}
async fn record_of(
	store: &Arc<dyn OAuthServerStore>,
	issued: &DeviceAuthorizationResponse,
) -> StoredDeviceAuthorization {
	store
		.device_authorization(&sha(&issued.device_code))
		.await
		.unwrap()
		.unwrap()
}
fn crafted(
	id: &str,
	device_code: &str,
	user_code: &str,
	expires_at: i64,
) -> StoredDeviceAuthorization {
	StoredDeviceAuthorization {
		id: id.into(),
		device_code_digest: sha(device_code),
		user_code_digest: sha(user_code),
		client_id: "tv-app".into(),
		scopes: vec!["read".into()],
		audience: "https://api.example".into(),
		session_digest: None,
		status: DeviceAuthorizationStatus::Pending,
		user_id: None,
		approved_scopes: Vec::new(),
		token_digest: None,
		interval: 5,
		last_polled_at: None,
		expires_at,
	}
}
fn form_request(uri: &str, body: &str, headers: &[(&'static str, &str)]) -> Request {
	let mut builder = Request::builder()
		.method(Method::POST)
		.uri(uri)
		.header("Content-Type", "application/x-www-form-urlencoded");
	for (name, value) in headers {
		builder = builder.header(*name, (*value).to_owned());
	}
	builder.body(Bytes::from(body.to_owned())).build().unwrap()
}
async fn call(server: &Arc<OAuthServer>, endpoint: OAuthEndpoint, request: Request) -> Response {
	OAuthHandler::new(server.clone(), endpoint)
		.handle(request)
		.await
		.unwrap()
}
fn json_of(response: &Response) -> serde_json::Value {
	serde_json::from_slice(&response.body).unwrap()
}

/// Generate a memory and a PostgreSQL test for a store-level scenario.
macro_rules! on_each_store {
	($name:ident) => {
		mod $name {
			use super::*;
			#[rstest]
			#[tokio::test]
			async fn memory() {
				super::$name(Arc::new(MemoryOAuthStore::new())).await;
			}
			#[cfg(feature = "database")]
			#[rstest]
			#[tokio::test]
			async fn postgres() {
				let (_container, store) = postgres_store().await;
				super::$name(store).await;
			}
		}
	};
}

#[cfg(feature = "database")]
async fn postgres_store() -> (
	testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
	Arc<PostgresOAuthStore>,
) {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use testcontainers::runners::AsyncRunner;

	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	DatabaseMigrationExecutor::new(connection)
		.apply_migrations(&PostgresOAuthStore::migrations())
		.await
		.unwrap();
	let pool = sqlx::PgPool::connect(&url).await.unwrap();
	(container, Arc::new(PostgresOAuthStore::new(pool)))
}

// ---------------------------------------------------------------------------
// Device authorization endpoint
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn endpoint_returns_rfc8628_response_without_cors() {
	// Arrange
	let store: Arc<dyn OAuthServerStore> = Arc::new(MemoryOAuthStore::new());
	let server = Arc::new(device_server(store.clone()).await);
	let request = form_request(
		"/oauth/device",
		"client_id=tv-app&scope=read",
		&[("Origin", "https://client.example")],
	);

	// Act
	let response = call(&server, OAuthEndpoint::DeviceAuthorization, request).await;

	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
	assert!(!response.headers.contains_key("access-control-allow-origin"));
	let body = json_of(&response);
	let device_code = body["device_code"].as_str().unwrap();
	let user_code = body["user_code"].as_str().unwrap();
	// NOTE: codes are random; only their shape can be asserted.
	assert_eq!(device_code.len(), 43);
	assert_eq!(user_code.len(), 9);
	assert_eq!(user_code.as_bytes()[4], b'-');
	assert!(
		user_code
			.chars()
			.filter(|c| *c != '-')
			.all(|c| USER_CODE_ALPHABET.contains(c))
	);
	assert_eq!(body["verification_uri"], VERIFICATION_URI);
	assert_eq!(
		body["verification_uri_complete"],
		format!("{VERIFICATION_URI}?user_code={user_code}")
	);
	assert_eq!(body["expires_in"], 600);
	assert_eq!(body["interval"], 5);
	// Only digests are stored.
	let stored = store
		.device_authorization(&sha(device_code))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(stored.user_code_digest, sha(&user_code.replace('-', "")));
	assert_eq!(stored.client_id, "tv-app");
	assert_eq!(stored.scopes, vec!["read"]);
	assert_eq!(stored.audience, "https://api.example");
	assert_eq!(stored.status, DeviceAuthorizationStatus::Pending);
}

#[rstest]
#[tokio::test]
async fn endpoint_has_no_preflight_and_accepts_only_post() {
	// Arrange
	let server = Arc::new(device_server(Arc::new(MemoryOAuthStore::new())).await);
	let preflight = Request::builder()
		.method(Method::OPTIONS)
		.uri("/oauth/device")
		.header("Origin", "https://client.example")
		.build()
		.unwrap();

	// Act
	let options = call(&server, OAuthEndpoint::DeviceAuthorization, preflight).await;
	let get = call(
		&server,
		OAuthEndpoint::DeviceAuthorization,
		Request::builder().uri("/oauth/device").build().unwrap(),
	)
	.await;

	// Assert
	assert_eq!(options.status, StatusCode::METHOD_NOT_ALLOWED);
	assert!(!options.headers.contains_key("access-control-allow-origin"));
	assert_eq!(get.status, StatusCode::METHOD_NOT_ALLOWED);
	assert_eq!(get.headers.get("allow").unwrap(), "POST");
}

#[rstest]
#[tokio::test]
async fn disabled_grant_is_rejected_everywhere() {
	// Arrange: the server configuration has no device_authorization.
	let server = Arc::new(server());
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();

	// Act
	let endpoint = call(
		&server,
		OAuthEndpoint::DeviceAuthorization,
		form_request("/oauth/device", "client_id=tv-app", &[]),
	)
	.await;
	let token = call(
		&server,
		OAuthEndpoint::Token,
		form_request(
			"/oauth/token",
			&format!("grant_type={DEVICE_GRANT}&client_id=tv-app&device_code=x"),
			&[],
		),
	)
	.await;
	let registration = server
		.register_client(device_client("tv-app", ClientKind::Public))
		.await;

	// Assert
	assert_eq!(endpoint.status, StatusCode::BAD_REQUEST);
	assert_eq!(endpoint.headers.get("cache-control").unwrap(), "no-store");
	assert_eq!(json_of(&endpoint)["error"], "invalid_request");
	assert_eq!(token.status, StatusCode::BAD_REQUEST);
	assert_eq!(json_of(&token)["error"], "unsupported_grant_type");
	assert_eq!(registration.unwrap_err(), OAuthError::InvalidRequest);
}

#[rstest]
#[tokio::test]
async fn client_without_device_flag_is_unauthorized() {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;
	server
		.register_client(client(ClientKind::Public))
		.await
		.unwrap();

	// Act
	let begin = server
		.begin_device_authorization("client-a", None, None, None)
		.await;
	let exchange = server
		.exchange_device_code("any-device-code", "client-a", None)
		.await;

	// Assert
	assert_eq!(begin.unwrap_err(), OAuthError::UnauthorizedClient);
	assert_eq!(exchange.unwrap_err(), OAuthError::UnauthorizedClient);
}

#[rstest]
#[tokio::test]
async fn openid_scope_is_rejected() {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;
	let mut openid_client = device_client("oidc-tv", ClientKind::Public);
	openid_client.scopes.push("openid".into());
	server.register_client(openid_client).await.unwrap();

	// Act
	let result = server
		.begin_device_authorization("oidc-tv", None, Some("openid"), None)
		.await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidScope);
}

#[rstest]
#[tokio::test]
async fn confidential_client_authenticates_with_basic_credentials() {
	use base64::{Engine as _, engine::general_purpose::STANDARD};

	// Arrange
	let server = Arc::new(device_server(Arc::new(MemoryOAuthStore::new())).await);
	let secret = server
		.register_client(device_client("kiosk", ClientKind::Confidential))
		.await
		.unwrap()
		.unwrap();
	let valid = format!("Basic {}", STANDARD.encode(format!("kiosk:{secret}")));
	let invalid = format!("Basic {}", STANDARD.encode("kiosk:wrong"));

	// Act
	let accepted = call(
		&server,
		OAuthEndpoint::DeviceAuthorization,
		form_request("/oauth/device", "", &[("Authorization", valid.as_str())]),
	)
	.await;
	let rejected = call(
		&server,
		OAuthEndpoint::DeviceAuthorization,
		form_request("/oauth/device", "", &[("Authorization", invalid.as_str())]),
	)
	.await;
	let missing_secret = call(
		&server,
		OAuthEndpoint::DeviceAuthorization,
		form_request("/oauth/device", "client_id=kiosk", &[]),
	)
	.await;

	// Assert
	assert_eq!(accepted.status, StatusCode::OK);
	assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
	assert_eq!(json_of(&rejected)["error"], "invalid_client");
	assert_eq!(missing_secret.status, StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn user_codes_use_the_restricted_alphabet_and_normalize() {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;
	let mut issued = Vec::new();
	for _ in 0..40 {
		issued.push(issue(&server).await);
	}

	// Act
	let lookups = [
		issued[0].user_code.to_lowercase(),
		issued[1].user_code.replace('-', ""),
		format!("  {} ", issued[2].user_code.replace('-', " ")),
	];
	for input in &lookups {
		let resolved = server
			.lookup_device_user_code(input, "session-a", CLIENT_IP)
			.await;
		assert!(resolved.is_ok(), "{input:?}");
	}

	// Assert
	for response in &issued {
		assert_eq!(response.user_code.len(), 9);
		assert!(
			response
				.user_code
				.chars()
				.filter(|c| *c != '-')
				.all(|c| USER_CODE_ALPHABET.contains(c))
		);
	}
	let codes: std::collections::HashSet<_> = issued.iter().map(|r| &r.user_code).collect();
	assert_eq!(codes.len(), issued.len());
}

#[rstest]
#[case::too_short("BCDF-GHJ")]
#[case::too_long("BCDF-GHJKL")]
#[case::outside_alphabet("BCDF-GHJA")]
#[case::digit("BCDF-GHJ1")]
#[case::empty("")]
#[case::unknown_but_well_formed("BCDF-GHJK")]
#[tokio::test]
async fn malformed_or_unknown_user_code_is_invalid_grant(#[case] input: &str) {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;

	// Act
	let result = server
		.lookup_device_user_code(input, "session-a", CLIENT_IP)
		.await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidGrant);
}

// ---------------------------------------------------------------------------
// Verification Page API
// ---------------------------------------------------------------------------

async fn lookup_binds_one_browser_session(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = issue(&server).await;

	// Act
	let first = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await
		.unwrap();
	let repeat = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await
		.unwrap();
	let other = server
		.lookup_device_user_code(&issued.user_code, "session-b", CLIENT_IP)
		.await;
	let hijack = server
		.complete_device_authorization(&first.id, "session-b", AuthorizationDecision::Deny)
		.await;

	// Assert
	assert_eq!(first, repeat);
	assert_eq!(first.client_id, "tv-app");
	assert_eq!(first.scopes, vec!["read", "write"]);
	assert_eq!(first.audience, "https://api.example");
	assert_ne!(first.id, issued.user_code);
	assert_ne!(first.id, issued.device_code);
	assert_eq!(other.unwrap_err(), OAuthError::InvalidGrant);
	assert_eq!(hijack.unwrap_err(), OAuthError::InvalidGrant);
	let record = record_of(&store, &issued).await;
	assert_eq!(record.status, DeviceAuthorizationStatus::Pending);
	assert_eq!(record.session_digest, Some(sha("session-a")));
}
on_each_store!(lookup_binds_one_browser_session);

async fn complete_requires_a_bound_undecided_record(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = issue(&server).await;
	let unbound = record_of(&store, &issued).await;

	// Act
	let before_lookup = server
		.complete_device_authorization(&unbound.id, "session-a", AuthorizationDecision::Deny)
		.await;
	approve(&server, &issued, "session-a", "user-a", &["read"]).await;
	let after_decision = server
		.complete_device_authorization(&unbound.id, "session-a", AuthorizationDecision::Deny)
		.await;
	let relookup = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await;

	// Assert
	assert_eq!(before_lookup.unwrap_err(), OAuthError::InvalidGrant);
	assert_eq!(after_decision.unwrap_err(), OAuthError::InvalidGrant);
	assert_eq!(relookup.unwrap_err(), OAuthError::InvalidGrant);
	let record = record_of(&store, &issued).await;
	assert_eq!(record.status, DeviceAuthorizationStatus::Approved);
	assert_eq!(record.user_id.as_deref(), Some("user-a"));
	assert_eq!(record.approved_scopes, vec!["read"]);
}
on_each_store!(complete_requires_a_bound_undecided_record);

async fn approval_with_extra_scope_leaves_the_record_untouched(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = server
		.begin_device_authorization("tv-app", None, Some("read"), None)
		.await
		.unwrap();
	let pending = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await
		.unwrap();

	// Act
	let excessive = server
		.complete_device_authorization(
			&pending.id,
			"session-a",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into(), "write".into()],
			},
		)
		.await;
	let record = record_of(&store, &issued).await;
	let retry = server
		.complete_device_authorization(
			&pending.id,
			"session-a",
			AuthorizationDecision::Approve {
				user_id: "user-a".into(),
				scopes: vec!["read".into()],
			},
		)
		.await;

	// Assert
	assert_eq!(excessive.unwrap_err(), OAuthError::InvalidScope);
	assert_eq!(record.status, DeviceAuthorizationStatus::Pending);
	assert!(retry.is_ok());
}
on_each_store!(approval_with_extra_scope_leaves_the_record_untouched);

struct NoUsers;
#[async_trait]
impl crate::UserRepository for NoUsers {
	async fn get_user_by_id(
		&self,
		_: &str,
	) -> Result<Option<Box<dyn crate::AuthIdentity>>, String> {
		Ok(None)
	}
}

#[rstest]
#[tokio::test]
async fn approval_by_an_unusable_user_is_recorded_as_denied() {
	// Arrange
	let store: Arc<dyn OAuthServerStore> = Arc::new(MemoryOAuthStore::new());
	let server = OAuthServer::for_development(
		device_config(),
		store.clone(),
		Arc::new(NoUsers),
		Arc::new(AllowAll),
	)
	.unwrap();
	server
		.register_resource("resource-a", "https://api.example")
		.await
		.unwrap();
	server
		.register_client(device_client("tv-app", ClientKind::Public))
		.await
		.unwrap();
	let issued = issue(&server).await;
	let pending = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await
		.unwrap();

	// Act
	let completed = server
		.complete_device_authorization(
			&pending.id,
			"session-a",
			AuthorizationDecision::Approve {
				user_id: "ghost".into(),
				scopes: vec!["read".into()],
			},
		)
		.await;
	let polled = poll(&server, &issued).await;

	// Assert
	assert!(completed.is_ok());
	assert_eq!(polled.unwrap_err(), OAuthError::AccessDenied);
	assert_eq!(
		record_of(&store, &issued).await.status,
		DeviceAuthorizationStatus::Denied
	);
}

struct VerificationLimit {
	max: usize,
	seen: std::sync::Mutex<HashMap<String, usize>>,
}
#[async_trait]
impl OAuthRateLimiter for VerificationLimit {
	async fn allow(&self, key: &str) -> bool {
		if !key.starts_with("DeviceVerification:") {
			return true;
		}
		let mut seen = self.seen.lock().unwrap();
		let count = seen.entry(key.to_owned()).or_default();
		*count += 1;
		*count <= self.max
	}
}

/// Identity of the `n`th guessing request: browser session and requester address.
type Requester = fn(usize) -> (String, IpAddr);

#[rstest]
#[case::same_session_and_address(|_| ("attacker".to_owned(), CLIENT_IP))]
#[case::fresh_session_per_guess(|n| (format!("attacker-{n}"), CLIENT_IP))]
#[case::fresh_address_per_guess(
	|n| ("attacker".to_owned(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, n as u8)))
)]
#[tokio::test]
async fn user_code_lookups_are_rate_limited_per_session_and_address(#[case] requester: Requester) {
	// Arrange: three lookups per rate-limit key are allowed.
	let limiter = Arc::new(VerificationLimit {
		max: 3,
		seen: Default::default(),
	});
	let server = server_with(Arc::new(MemoryOAuthStore::new()), device_config(), limiter).await;
	server
		.register_client(device_client("tv-app", ClientKind::Public))
		.await
		.unwrap();
	let issued = issue(&server).await;

	// Act
	let mut guesses = Vec::new();
	for n in 0..3 {
		let (session, ip) = requester(n);
		guesses.push(
			server
				.lookup_device_user_code("BCDF-GHJK", &session, ip)
				.await,
		);
	}
	let (session, ip) = requester(3);
	let throttled = server
		.lookup_device_user_code(&issued.user_code, &session, ip)
		.await;
	let empty_session = server
		.lookup_device_user_code(&issued.user_code, "", ip)
		.await;
	let victim = server
		.lookup_device_user_code(&issued.user_code, "victim", OTHER_CLIENT_IP)
		.await;

	// Assert
	assert!(
		guesses
			.iter()
			.all(|guess| *guess.as_ref().unwrap_err() == OAuthError::InvalidGrant)
	);
	assert_eq!(throttled.unwrap_err(), OAuthError::SlowDown);
	assert_eq!(empty_session.unwrap_err(), OAuthError::InvalidRequest);
	assert!(victim.is_ok());
	assert_eq!(OAuthError::SlowDown.as_str(), "slow_down");
}

// ---------------------------------------------------------------------------
// Token endpoint polling
// ---------------------------------------------------------------------------

async fn polling_throttles_and_grows_the_interval(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = issue(&server).await;

	// Act
	let first = poll(&server, &issued).await;
	let second = poll(&server, &issued).await;
	let after_second = record_of(&store, &issued).await.interval;
	let third = poll(&server, &issued).await;
	let after_third = record_of(&store, &issued).await.interval;

	// Assert
	assert_eq!(first.unwrap_err(), OAuthError::AuthorizationPending);
	assert_eq!(second.unwrap_err(), OAuthError::SlowDown);
	assert_eq!(after_second, 10);
	assert_eq!(third.unwrap_err(), OAuthError::SlowDown);
	assert_eq!(after_third, 15);
	assert!(record_of(&store, &issued).await.last_polled_at.is_some());
}
on_each_store!(polling_throttles_and_grows_the_interval);

async fn approval_after_pending_yields_a_token(store: Arc<dyn OAuthServerStore>) {
	// Arrange: a one-second interval lets the test wait out the throttle.
	let mut settings = device_settings();
	settings.poll_interval = Duration::from_secs(1);
	let config = config().with_device_authorization(settings).unwrap();
	let server = server_with(store.clone(), config, Arc::new(AllowAll)).await;
	server
		.register_client(device_client("tv-app", ClientKind::Public))
		.await
		.unwrap();
	let issued = issue(&server).await;
	let pending = poll(&server, &issued).await;
	approve(&server, &issued, "session-a", "user-a", &["read"]).await;
	tokio::time::sleep(Duration::from_millis(1100)).await;

	// Act
	let token = poll(&server, &issued).await.unwrap();

	// Assert
	assert_eq!(pending.unwrap_err(), OAuthError::AuthorizationPending);
	assert_eq!(token.token_type, "Bearer");
	assert_eq!(token.scope, "read");
	assert_eq!(token.expires_in, 3600);
	let info = server
		.token_info(&token.access_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(info.client_id, "tv-app");
	assert_eq!(info.principal, TokenPrincipal::User("user-a".into()));
	assert_eq!(info.scopes, vec!["read"]);
	assert_eq!(info.audience, "https://api.example");
	let record = record_of(&store, &issued).await;
	assert_eq!(record.status, DeviceAuthorizationStatus::Redeemed);
	assert_eq!(record.token_digest, Some(sha(&token.access_token)));
}
on_each_store!(approval_after_pending_yields_a_token);

async fn denied_authorization_reports_access_denied(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store).await;
	let issued = issue(&server).await;
	let pending = server
		.lookup_device_user_code(&issued.user_code, "session-a", CLIENT_IP)
		.await
		.unwrap();
	server
		.complete_device_authorization(&pending.id, "session-a", AuthorizationDecision::Deny)
		.await
		.unwrap();

	// Act
	let result = poll(&server, &issued).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::AccessDenied);
}
on_each_store!(denied_authorization_reports_access_denied);

async fn expired_authorization_reports_expired_token(store: Arc<dyn OAuthServerStore>) {
	// Arrange: expiry wins over the pending state.
	let server = device_server(store.clone()).await;
	let now = unix_now();
	store
		.insert_device_authorization(
			crafted("expired", "expired-code", "BCDFGHJK", now - 10),
			now - 20,
		)
		.await
		.unwrap();

	// Act
	let result = server
		.exchange_device_code("expired-code", "tv-app", None)
		.await;
	let lookup = server
		.lookup_device_user_code("BCDF-GHJK", "session-a", CLIENT_IP)
		.await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::ExpiredToken);
	assert_eq!(lookup.unwrap_err(), OAuthError::InvalidGrant);
}
on_each_store!(expired_authorization_reports_expired_token);

async fn expiry_precedes_slow_down_for_rapid_polls(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let now = unix_now();
	store
		.insert_device_authorization(
			crafted("expired", "expired-code", "BCDFGHJK", now - 10),
			now - 20,
		)
		.await
		.unwrap();

	// Act: the second poll arrives well inside the five-second interval.
	let first = server
		.exchange_device_code("expired-code", "tv-app", None)
		.await;
	let second = server
		.exchange_device_code("expired-code", "tv-app", None)
		.await;

	// Assert
	assert_eq!(first.unwrap_err(), OAuthError::ExpiredToken);
	assert_eq!(second.unwrap_err(), OAuthError::ExpiredToken);
	let record = store
		.device_authorization(&sha("expired-code"))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(record.interval, 5);
	assert_eq!(record.last_polled_at, None);
}
on_each_store!(expiry_precedes_slow_down_for_rapid_polls);

async fn unknown_and_foreign_device_codes_are_invalid_grants(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = issue(&server).await;

	// Act
	let unknown = server
		.exchange_device_code("never-issued", "tv-app", None)
		.await;
	let foreign = server
		.exchange_device_code(&issued.device_code, "other-tv", None)
		.await;

	// Assert
	assert_eq!(unknown.unwrap_err(), OAuthError::InvalidGrant);
	assert_eq!(foreign.unwrap_err(), OAuthError::InvalidGrant);
	// The owner's polling state is not touched by the other client.
	assert_eq!(record_of(&store, &issued).await.last_polled_at, None);
}
on_each_store!(unknown_and_foreign_device_codes_are_invalid_grants);

async fn replayed_device_code_revokes_its_token(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let issued = issue(&server).await;
	approve(&server, &issued, "session-a", "user-a", &["read"]).await;
	let token = poll(&server, &issued).await.unwrap();
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_some()
	);

	// Act
	let replay = poll(&server, &issued).await;

	// Assert
	assert_eq!(replay.unwrap_err(), OAuthError::InvalidGrant);
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(
		record_of(&store, &issued).await.status,
		DeviceAuthorizationStatus::Redeemed
	);
}
on_each_store!(replayed_device_code_revokes_its_token);

#[rstest]
#[tokio::test]
async fn token_endpoint_reports_polling_errors_and_requires_device_code() {
	// Arrange
	let server = Arc::new(device_server(Arc::new(MemoryOAuthStore::new())).await);
	let issued = issue(&server).await;
	let grant = |device_code: &str| {
		form_request(
			"/oauth/token",
			&format!("grant_type={DEVICE_GRANT}&client_id=tv-app&device_code={device_code}"),
			&[],
		)
	};

	// Act
	let pending = call(&server, OAuthEndpoint::Token, grant(&issued.device_code)).await;
	let throttled = call(&server, OAuthEndpoint::Token, grant(&issued.device_code)).await;
	let missing = call(
		&server,
		OAuthEndpoint::Token,
		form_request(
			"/oauth/token",
			&format!("grant_type={DEVICE_GRANT}&client_id=tv-app"),
			&[],
		),
	)
	.await;

	// Assert
	assert_eq!(pending.status, StatusCode::BAD_REQUEST);
	assert_eq!(json_of(&pending)["error"], "authorization_pending");
	assert_eq!(throttled.status, StatusCode::BAD_REQUEST);
	assert_eq!(json_of(&throttled)["error"], "slow_down");
	assert_eq!(missing.status, StatusCode::BAD_REQUEST);
	assert_eq!(json_of(&missing)["error"], "invalid_request");
}

#[rstest]
#[tokio::test]
async fn token_endpoint_issues_the_standard_token_response() {
	// Arrange
	let server = Arc::new(device_server(Arc::new(MemoryOAuthStore::new())).await);
	let issued = issue(&server).await;
	approve(&server, &issued, "session-a", "user-a", &["read", "write"]).await;

	// Act
	let response = call(
		&server,
		OAuthEndpoint::Token,
		form_request(
			"/oauth/token",
			&format!(
				"grant_type={DEVICE_GRANT}&client_id=tv-app&device_code={}",
				issued.device_code
			),
			&[],
		),
	)
	.await;

	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
	let body = json_of(&response);
	assert_eq!(body["token_type"], "Bearer");
	assert_eq!(body["scope"], "read write");
	assert_eq!(body["expires_in"], 3600);
	assert!(body["access_token"].as_str().is_some_and(|t| t.len() == 43));
}

// ---------------------------------------------------------------------------
// Invalidation and revocation
// ---------------------------------------------------------------------------

async fn user_security_event_invalidates_approved_authorizations(
	store: Arc<dyn OAuthServerStore>,
	retire: bool,
) {
	// Arrange: approved by user-a, approved by user-b, undecided, and already redeemed.
	let server = device_server(store.clone()).await;
	let approved_a = issue(&server).await;
	approve(&server, &approved_a, "session-a", "user-a", &["read"]).await;
	let approved_b = issue(&server).await;
	approve(&server, &approved_b, "session-b", "user-b", &["read"]).await;
	let undecided = issue(&server).await;
	let redeemed = issue(&server).await;
	approve(&server, &redeemed, "session-c", "user-a", &["read"]).await;
	let device_token = poll(&server, &redeemed).await.unwrap();

	// Act
	let revoked = if retire {
		store.retire_user("user-a").await.unwrap();
		None
	} else {
		Some(server.revoke_user("user-a").await.unwrap())
	};

	// Assert
	assert_eq!(
		poll(&server, &approved_a).await.unwrap_err(),
		OAuthError::InvalidGrant
	);
	assert_eq!(
		record_of(&store, &approved_a).await.status,
		DeviceAuthorizationStatus::Invalidated
	);
	assert!(poll(&server, &approved_b).await.is_ok());
	assert_eq!(
		poll(&server, &undecided).await.unwrap_err(),
		OAuthError::AuthorizationPending
	);
	assert_eq!(
		record_of(&store, &redeemed).await.status,
		DeviceAuthorizationStatus::Redeemed
	);
	// The token issued from a device authorization is a normal user token.
	assert!(
		server
			.token_info(&device_token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	if let Some(revoked) = revoked {
		assert_eq!(revoked, 1);
	}
}
async fn revoke_user_invalidates_approved_authorizations(store: Arc<dyn OAuthServerStore>) {
	user_security_event_invalidates_approved_authorizations(store, false).await;
}
async fn retire_user_invalidates_approved_authorizations(store: Arc<dyn OAuthServerStore>) {
	user_security_event_invalidates_approved_authorizations(store, true).await;
}
on_each_store!(revoke_user_invalidates_approved_authorizations);
on_each_store!(retire_user_invalidates_approved_authorizations);

async fn disable_client_invalidates_every_authorization(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store.clone()).await;
	let approved = issue(&server).await;
	approve(&server, &approved, "session-a", "user-a", &["read"]).await;
	let undecided = issue(&server).await;
	let redeemed = issue(&server).await;
	approve(&server, &redeemed, "session-b", "user-a", &["read"]).await;
	let device_token = poll(&server, &redeemed).await.unwrap();
	let unrelated = server
		.begin_device_authorization("other-tv", None, None, None)
		.await
		.unwrap();

	// Act
	let revoked = server.disable_client("tv-app").await.unwrap();

	// Assert
	assert_eq!(revoked, 1);
	for issued in [&approved, &undecided] {
		assert_eq!(
			record_of(&store, issued).await.status,
			DeviceAuthorizationStatus::Invalidated
		);
		assert!(matches!(
			store
				.poll_device_authorization(&sha(&issued.device_code), "tv-app", unix_now())
				.await
				.unwrap(),
			DevicePoll::Invalid
		));
	}
	assert_eq!(
		record_of(&store, &redeemed).await.status,
		DeviceAuthorizationStatus::Redeemed
	);
	assert!(
		server
			.token_info(&device_token.access_token)
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(
		record_of(&store, &unrelated).await.status,
		DeviceAuthorizationStatus::Pending
	);
	assert_eq!(
		server
			.lookup_device_user_code(&undecided.user_code, "session-c", CLIENT_IP)
			.await
			.unwrap_err(),
		OAuthError::InvalidGrant
	);
}
on_each_store!(disable_client_invalidates_every_authorization);

async fn revoke_client_revokes_device_issued_tokens(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let server = device_server(store).await;
	let issued = issue(&server).await;
	approve(&server, &issued, "session-a", "user-a", &["read"]).await;
	let token = poll(&server, &issued).await.unwrap();

	// Act
	let revoked = server.revoke_client("tv-app").await.unwrap();

	// Assert
	assert_eq!(revoked, 1);
	assert!(
		server
			.token_info(&token.access_token)
			.await
			.unwrap()
			.is_none()
	);
}
on_each_store!(revoke_client_revokes_device_issued_tokens);

async fn user_code_collisions_are_rejected_until_expiry(store: Arc<dyn OAuthServerStore>) {
	// Arrange
	let now = unix_now();
	let live = crafted("id-1", "device-1", "BCDFGHJK", now + 600);
	let colliding = crafted("id-2", "device-2", "BCDFGHJK", now + 600);
	let reused_id = crafted("id-1", "device-3", "LMNPQRST", now + 600);
	let reused_device = crafted("id-4", "device-1", "VWXZBCDF", now + 600);

	// Act
	let first = store.insert_device_authorization(live, now).await.unwrap();
	let second = store
		.insert_device_authorization(colliding.clone(), now)
		.await
		.unwrap();
	let third = store
		.insert_device_authorization(reused_id, now)
		.await
		.unwrap();
	let fourth = store
		.insert_device_authorization(reused_device, now)
		.await
		.unwrap();
	let after_expiry = store
		.insert_device_authorization(colliding, now + 601)
		.await
		.unwrap();

	// Assert
	assert!(first);
	assert!(!second);
	assert!(!third);
	assert!(!fourth);
	assert!(after_expiry);
	let kept = store
		.device_authorization_by_id("id-2")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(kept.device_code_digest, sha("device-2"));
	// The expired record that held the User Code is gone.
	assert!(
		store
			.device_authorization_by_id("id-1")
			.await
			.unwrap()
			.is_none()
	);
}
on_each_store!(user_code_collisions_are_rejected_until_expiry);

// ---------------------------------------------------------------------------
// Metadata, configuration and registration
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn metadata_advertises_the_grant_only_when_enabled() {
	// Arrange
	let enabled = Arc::new(device_server(Arc::new(MemoryOAuthStore::new())).await);
	let disabled = Arc::new(server());
	let get = || Request::builder().uri("/metadata").build().unwrap();

	// Act
	let with = json_of(&call(&enabled, OAuthEndpoint::Metadata, get()).await);
	let without = json_of(&call(&disabled, OAuthEndpoint::Metadata, get()).await);

	// Assert
	assert_eq!(
		with["device_authorization_endpoint"],
		"https://auth.example/oauth/device"
	);
	assert_eq!(
		with["grant_types_supported"],
		serde_json::json!(["authorization_code", "client_credentials", DEVICE_GRANT])
	);
	assert!(without.get("device_authorization_endpoint").is_none());
	assert_eq!(
		without["grant_types_supported"],
		serde_json::json!(["authorization_code", "client_credentials"])
	);
}

fn with_settings(change: impl FnOnce(&mut DeviceAuthorizationConfig)) -> Result<(), OAuthError> {
	let mut settings = device_settings();
	change(&mut settings);
	config().with_device_authorization(settings).map(|_| ())
}

#[rstest]
#[case::ttl_zero(0, 5, Err(OAuthError::InvalidRequest))]
#[case::ttl_minimum(1, 5, Ok(()))]
#[case::ttl_maximum(1800, 5, Ok(()))]
#[case::ttl_too_long(1801, 5, Err(OAuthError::InvalidRequest))]
#[case::interval_zero(600, 0, Err(OAuthError::InvalidRequest))]
#[case::interval_minimum(600, 1, Ok(()))]
#[case::interval_maximum(600, 60, Ok(()))]
#[case::interval_too_long(600, 61, Err(OAuthError::InvalidRequest))]
fn device_lifetimes_are_bounded(
	#[case] ttl: u64,
	#[case] interval: u64,
	#[case] expected: Result<(), OAuthError>,
) {
	// Act
	let result = with_settings(|settings| {
		settings.code_ttl = Duration::from_secs(ttl);
		settings.poll_interval = Duration::from_secs(interval);
	});

	// Assert
	assert_eq!(result, expected);
}

#[rstest]
#[case::foreign_origin_endpoint("https://evil.example/oauth/device", VERIFICATION_URI)]
#[case::endpoint_query("https://auth.example/oauth/device?x=1", VERIFICATION_URI)]
#[case::endpoint_userinfo("https://user@auth.example/oauth/device", VERIFICATION_URI)]
#[case::endpoint_relative("/oauth/device", VERIFICATION_URI)]
#[case::foreign_origin_verification(
	"https://auth.example/oauth/device",
	"https://evil.example/device"
)]
#[case::verification_fragment("https://auth.example/oauth/device", "https://auth.example/device#x")]
#[case::verification_query("https://auth.example/oauth/device", "https://auth.example/device?x=1")]
#[case::verification_http("https://auth.example/oauth/device", "http://auth.example/device")]
fn device_urls_follow_the_endpoint_rules(#[case] endpoint: &str, #[case] verification: &str) {
	// Act
	let result =
		config().with_device_authorization(DeviceAuthorizationConfig::new(endpoint, verification));

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidRequest);
}

#[rstest]
fn defaults_leave_the_grant_disabled_and_use_documented_lifetimes() {
	// Act
	let plain = config();
	let settings = device_settings();

	// Assert
	assert!(plain.device_authorization.is_none());
	assert_eq!(settings.code_ttl, Duration::from_secs(600));
	assert_eq!(settings.poll_interval, Duration::from_secs(5));
}

#[rstest]
#[case::public_device_only(ClientKind::Public, Ok(()))]
#[case::confidential_device_only(ClientKind::Confidential, Ok(()))]
#[tokio::test]
async fn device_only_clients_need_no_redirect_uri(
	#[case] kind: ClientKind,
	#[case] expected: Result<(), OAuthError>,
) {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;

	// Act
	let result = server
		.register_client(device_client("device-only", kind))
		.await
		.map(|_| ());

	// Assert
	assert_eq!(result, expected);
}

#[rstest]
#[tokio::test]
async fn a_client_needs_at_least_one_grant() {
	// Arrange
	let server = device_server(Arc::new(MemoryOAuthStore::new())).await;
	let mut grantless = device_client("grantless", ClientKind::Public);
	grantless.device_code = false;

	// Act
	let result = server.register_client(grantless).await;

	// Assert
	assert_eq!(result.unwrap_err(), OAuthError::InvalidRequest);
}

#[rstest]
fn new_registration_has_everything_disabled_or_empty() {
	// Act
	let registration = ClientRegistration::new("tv", ClientKind::Confidential);

	// Assert
	assert_eq!(registration.client_id, "tv");
	assert_eq!(registration.kind, ClientKind::Confidential);
	assert!(registration.enabled);
	assert!(!registration.oidc_enabled);
	assert!(
		!registration.authorization_code
			&& !registration.client_credentials
			&& !registration.device_code
	);
	assert!(
		registration.redirect_uris.is_empty()
			&& registration.scopes.is_empty()
			&& registration.default_scopes.is_empty()
			&& registration.audiences.is_empty()
			&& registration.browser_origins.is_empty()
	);
	assert!(
		registration.secret_hash.is_none()
			&& registration.previous_secret_hash.is_none()
			&& registration.previous_secret_expires_at.is_none()
			&& registration.default_audience.is_none()
	);
}

#[rstest]
fn persisted_registration_without_device_code_still_deserializes() {
	// Arrange: a registration written before the device_code field existed.
	let original = client(ClientKind::Public);
	let mut json = serde_json::to_value(&original).unwrap();
	json.as_object_mut().unwrap().remove("device_code").unwrap();

	// Act
	let restored: ClientRegistration = serde_json::from_value(json).unwrap();

	// Assert
	assert!(!restored.device_code);
	assert_eq!(restored, original);
}

// ---------------------------------------------------------------------------
// PostgreSQL schema
// ---------------------------------------------------------------------------

#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn migrations_apply_and_roll_back_every_step() {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::DatabaseMigrationExecutor;
	use testcontainers::runners::AsyncRunner;

	// Arrange
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.expect("PostgreSQL container");
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let mut executor =
		DatabaseMigrationExecutor::new(DatabaseConnection::connect_postgres(&url).await.unwrap());
	let pool = sqlx::PgPool::connect(&url).await.unwrap();
	let migrations = PostgresOAuthStore::migrations();
	let exists = |table: &'static str| {
		let pool = pool.clone();
		async move {
			let row: (Option<String>,) = sqlx::query_as("SELECT to_regclass($1)::text")
				.bind(table)
				.fetch_one(&pool)
				.await
				.unwrap();
			row.0.is_some()
		}
	};

	// Act
	let applied = executor.apply_migrations(&migrations).await.unwrap();
	let repeated = executor.apply_migrations(&migrations).await.unwrap();
	let all_tables = (
		exists("oauth_server_tokens").await,
		exists("oauth_server_device_authorizations").await,
		exists("oauth_server_token_families").await,
		exists("oauth_server_refresh_tokens").await,
	);
	let rolled_back_third = executor
		.rollback_migrations(&migrations[2..])
		.await
		.unwrap();
	let after_third = (
		exists("oauth_server_tokens").await,
		exists("oauth_server_device_authorizations").await,
		exists("oauth_server_token_families").await,
		exists("oauth_server_refresh_tokens").await,
	);
	let rolled_back_second = executor
		.rollback_migrations(&migrations[1..2])
		.await
		.unwrap();
	let after_second = (
		exists("oauth_server_tokens").await,
		exists("oauth_server_device_authorizations").await,
	);
	let rolled_back_first = executor
		.rollback_migrations(&migrations[..1])
		.await
		.unwrap();
	let after_first = exists("oauth_server_tokens").await;

	// Assert
	assert_eq!(migrations.len(), 3);
	assert_eq!(migrations[0].name, "0001_oauth_server");
	assert!(migrations[0].dependencies.is_empty());
	assert_eq!(
		migrations[1].name,
		"0002_oauth_server_device_authorizations"
	);
	assert_eq!(
		migrations[1].dependencies,
		vec![("oauth_server".to_owned(), "0001_oauth_server".to_owned())]
	);
	assert_eq!(migrations[2].name, "0003_oauth_server_refresh_tokens");
	assert_eq!(
		migrations[2].dependencies,
		vec![(
			"oauth_server".to_owned(),
			"0002_oauth_server_device_authorizations".to_owned()
		)]
	);
	assert_eq!(applied.applied.len(), 3);
	assert!(repeated.applied.is_empty());
	assert_eq!(all_tables, (true, true, true, true));
	assert_eq!(rolled_back_third.applied.len(), 1);
	assert_eq!(after_third, (true, true, false, false));
	assert_eq!(rolled_back_second.applied.len(), 1);
	assert_eq!(after_second, (true, false));
	assert_eq!(rolled_back_first.applied.len(), 1);
	assert!(!after_first);
}

#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn purge_removes_only_expired_device_authorizations() {
	// Arrange
	let (_container, store) = postgres_store().await;
	store
		.insert_device_authorization(crafted("old", "old-code", "BCDFGHJK", 100), 0)
		.await
		.unwrap();
	store
		.insert_device_authorization(crafted("new", "new-code", "LMNPQRST", 5000), 0)
		.await
		.unwrap();

	// Act
	let deleted = store.purge_expired(1000).await.unwrap();

	// Assert
	assert_eq!(deleted, 1);
	assert!(
		store
			.device_authorization_by_id("old")
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		store
			.device_authorization_by_id("new")
			.await
			.unwrap()
			.is_some()
	);
}
