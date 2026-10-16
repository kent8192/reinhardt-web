use async_trait::async_trait;
use http::{HeaderMap, StatusCode};
use reinhardt_auth::AuthBackend;
use reinhardt_auth::jwt::JwtAuth;
use reinhardt_http::{AuthState, Handler, MiddlewareChain, Request, Response};
use reinhardt_middleware::CookieSessionAuthMiddleware;
use reinhardt_middleware::session::{AsyncSessionBackend, SessionData};
use reinhardt_test::APIClient;
use reinhardt_test::auth::{
	ForceLoginUser, JwtTestConfig, SecondaryAuth, SessionIdentity, TestAuthError,
	TestSessionAuthBackend,
};
use rstest::{fixture, rstest};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

struct FixtureUser;

impl ForceLoginUser for FixtureUser {
	fn session_user_id(&self) -> String {
		"user-42".into()
	}

	fn session_is_staff(&self) -> bool {
		false
	}
}

struct RecordingBackend {
	saved: Arc<Mutex<Vec<SessionData>>>,
	failure: Option<&'static str>,
}

#[async_trait]
impl AsyncSessionBackend for RecordingBackend {
	async fn load(&self, id: &str) -> reinhardt_http::Result<Option<SessionData>> {
		Ok(self
			.saved
			.lock()
			.unwrap()
			.iter()
			.rev()
			.find(|session| session.id == id && session.expires_at > SystemTime::now())
			.cloned())
	}

	async fn save(&self, session: &SessionData) -> reinhardt_http::Result<()> {
		if let Some(message) = self.failure {
			return Err(reinhardt_http::Error::Internal(message.into()));
		}
		self.saved.lock().unwrap().push(session.clone());
		Ok(())
	}

	async fn destroy(&self, id: &str) -> reinhardt_http::Result<()> {
		self.saved
			.lock()
			.unwrap()
			.retain(|session| session.id != id);
		Ok(())
	}

	async fn touch(&self, _id: &str, _ttl: Duration) -> reinhardt_http::Result<()> {
		Ok(())
	}
}

struct AccountFixture {
	backend: Arc<TestSessionAuthBackend>,
	sessions: Arc<RecordingBackend>,
}

#[fixture]
fn account_backend() -> AccountFixture {
	let sessions = Arc::new(RecordingBackend {
		saved: Arc::new(Mutex::new(Vec::new())),
		failure: None,
	});
	AccountFixture {
		backend: Arc::new(TestSessionAuthBackend::new(sessions.clone())),
		sessions,
	}
}

struct AuthStateHandler;

#[async_trait]
impl Handler for AuthStateHandler {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		let state = request.extensions.get::<AuthState>().unwrap();
		Response::ok().with_json(&json!({
			"user_id": state.user_id(),
			"authenticated": state.is_authenticated(),
			"admin": state.is_admin(),
			"active": state.is_active(),
		}))
	}
}

fn cookie_client(backend: Arc<TestSessionAuthBackend>, resolve_accounts: bool) -> APIClient {
	let middleware = CookieSessionAuthMiddleware::new(backend.clone());
	let middleware = if resolve_accounts {
		middleware.with_auth_backend(backend)
	} else {
		middleware
	};
	let chain =
		MiddlewareChain::new(Arc::new(AuthStateHandler)).with_middleware(Arc::new(middleware));
	APIClient::from_handler(chain)
}

#[rstest]
#[case(false, false, false)]
#[case(true, false, true)]
#[case(false, true, true)]
#[case(true, true, true)]
#[tokio::test]
async fn session_overrides_reach_live_cookie_authentication(
	account_backend: AccountFixture,
	#[case] staff: bool,
	#[case] superuser: bool,
	#[case] expected_admin: bool,
) {
	// Arrange: the public adapter provides session storage and current-account lookup.
	let client = cookie_client(account_backend.backend.clone(), true);

	// Act: override privileges through the existing testkit builder.
	client
		.auth()
		.session(&FixtureUser, account_backend.backend.clone())
		.with_staff(staff)
		.with_superuser(superuser)
		.apply()
		.await
		.unwrap();
	let response = client.get("/auth-state").await.unwrap();

	// Assert: the real cookie middleware resolves the trusted current profile.
	assert_eq!(
		response.json_value().unwrap(),
		json!({
			"user_id": "user-42", "authenticated": true,
			"admin": expected_admin, "active": true,
		})
	);
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn cookie_authentication_ignores_direct_session_flag_tampering(
	account_backend: AccountFixture,
	#[case] initial_admin: bool,
) {
	// Arrange: initialize a trusted current account and a valid cookie.
	let client = cookie_client(account_backend.backend.clone(), true);
	client
		.auth()
		.session(&FixtureUser, account_backend.backend.clone())
		.with_staff(initial_admin)
		.apply()
		.await
		.unwrap();

	// Act: mutate the stored metadata without updating the account registry.
	{
		let mut sessions = account_backend.sessions.saved.lock().unwrap();
		let session = sessions.last_mut().unwrap();
		session
			.data
			.insert("is_staff".into(), json!(!initial_admin));
		session
			.data
			.insert("is_superuser".into(), json!(!initial_admin));
	}
	let response = client.get("/auth-state").await.unwrap();

	// Assert: cached flags cannot grant or revoke the current account's privilege.
	assert_eq!(
		response.json_value().unwrap()["admin"],
		json!(initial_admin)
	);
}

#[rstest]
#[tokio::test]
async fn cookie_authentication_rechecks_fixture_account_privileges(
	account_backend: AccountFixture,
) {
	// Arrange: an initially privileged fixture account.
	let client = cookie_client(account_backend.backend.clone(), true);
	client
		.auth()
		.session(&FixtureUser, account_backend.backend.clone())
		.with_superuser(true)
		.apply()
		.await
		.unwrap();
	assert_eq!(
		client
			.get("/auth-state")
			.await
			.unwrap()
			.json_value()
			.unwrap()["admin"],
		json!(true)
	);

	// Act: revoke the current privilege while keeping the existing session/cookie.
	let replacement = SessionIdentity {
		user_id: "user-42".into(),
		is_staff: false,
		is_superuser: false,
	}
	.to_session_data("replacement-session", Duration::from_secs(60));
	account_backend.backend.save(&replacement).await.unwrap();
	// The original session still contains the old cached privilege.
	assert_eq!(
		account_backend.sessions.saved.lock().unwrap()[0].data["is_superuser"],
		json!(true)
	);
	let response = client.get("/auth-state").await.unwrap();

	// Assert: the next request observes current account state.
	assert_eq!(response.json_value().unwrap()["admin"], json!(false));
}

#[rstest]
#[tokio::test]
async fn session_overrides_require_a_live_account_backend(account_backend: AccountFixture) {
	// Arrange: intentionally omit the separate account resolver.
	let client = cookie_client(account_backend.backend.clone(), false);

	// Act: save privileged metadata using the existing builder.
	client
		.auth()
		.session(&FixtureUser, account_backend.backend)
		.with_staff(true)
		.with_superuser(true)
		.apply()
		.await
		.unwrap();
	let response = client.get("/auth-state").await.unwrap();

	// Assert: session metadata alone cannot authenticate or grant admin access.
	assert_eq!(
		response.json_value().unwrap()["authenticated"],
		json!(false)
	);
	assert_eq!(response.json_value().unwrap()["admin"], json!(false));
}

#[rstest]
#[tokio::test]
async fn failed_fixture_session_save_publishes_no_current_account() {
	// Arrange: an unavailable session store with an empty account registry.
	let sessions = Arc::new(RecordingBackend {
		saved: Arc::new(Mutex::new(Vec::new())),
		failure: Some("backend unavailable"),
	});
	let backend = Arc::new(TestSessionAuthBackend::new(sessions.clone()));
	let client = cookie_client(backend.clone(), true);

	// Act: the trusted fixture write fails before publishing identity.
	let error = client
		.auth()
		.session(&FixtureUser, backend.clone())
		.with_staff(true)
		.apply()
		.await
		.unwrap_err();

	// Assert: neither session persistence nor account resolution succeeds.
	assert!(matches!(error, TestAuthError::SessionError(_)));
	assert!(sessions.saved.lock().unwrap().is_empty());
	assert!(backend.get_user("user-42").await.unwrap().is_none());
}

struct HeaderSecondary;

#[async_trait]
impl SecondaryAuth for HeaderSecondary {
	async fn apply_to_client(
		&self,
		client: &APIClient,
		primary: &SessionIdentity,
	) -> Result<(), TestAuthError> {
		client
			.set_header("X-Secondary-User", &primary.user_id)
			.await
			.map_err(|error| TestAuthError::ClientError(error.to_string()))
	}
}

struct RecordingHandler {
	captured: Arc<Mutex<Vec<HeaderMap>>>,
}

#[async_trait]
impl Handler for RecordingHandler {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		self.captured.lock().unwrap().push(request.headers.clone());
		Ok(Response::no_content())
	}
}

fn recording_client(captured: Arc<Mutex<Vec<HeaderMap>>>) -> APIClient {
	APIClient::from_handler(RecordingHandler { captured })
}

#[rstest]
#[tokio::test]
async fn api_client_auth_builders_apply_session_jwt_and_secondary_auth() {
	// Arrange
	let saved = Arc::new(Mutex::new(Vec::new()));
	let session_headers = Arc::new(Mutex::new(Vec::new()));
	let jwt_headers = Arc::new(Mutex::new(Vec::new()));
	let session_client = recording_client(Arc::clone(&session_headers));
	let jwt_client = recording_client(Arc::clone(&jwt_headers));
	let backend = Arc::new(RecordingBackend {
		saved: Arc::clone(&saved),
		failure: None,
	});
	let config = JwtTestConfig {
		secret: "coverage-test-secret".into(),
		expiry: Duration::from_secs(120),
	};
	let user = FixtureUser;

	// Act
	session_client
		.auth()
		.session(&user, backend)
		.with_staff(true)
		.with_superuser(true)
		.with_ttl(Duration::from_secs(90))
		.with_secondary(HeaderSecondary)
		.apply()
		.await
		.unwrap();
	let session_response = session_client.get("/session-check").await.unwrap();
	jwt_client
		.auth()
		.jwt(&user, config.clone())
		.with_secondary(HeaderSecondary)
		.apply()
		.await
		.unwrap();
	let jwt_response = jwt_client.get("/jwt-check").await.unwrap();
	let failed = APIClient::new()
		.auth()
		.session(
			&user,
			Arc::new(RecordingBackend {
				saved: Arc::new(Mutex::new(Vec::new())),
				failure: Some("backend unavailable"),
			}),
		)
		.apply()
		.await
		.unwrap_err();

	// Assert
	let saved_session = saved.lock().unwrap().pop().unwrap();
	assert_eq!(saved_session.data.get("user_id"), Some(&json!("user-42")));
	assert_eq!(saved_session.data.get("is_staff"), Some(&json!(true)));
	assert_eq!(saved_session.data.get("is_superuser"), Some(&json!(true)));
	assert_eq!(
		saved_session
			.expires_at
			.duration_since(saved_session.created_at)
			.unwrap(),
		Duration::from_secs(90)
	);
	assert_eq!(session_response.status(), StatusCode::NO_CONTENT);
	let session_request = session_headers.lock().unwrap().pop().unwrap();
	assert_eq!(
		session_request.get("cookie").unwrap().to_str().unwrap(),
		format!("sessionid={}", saved_session.id)
	);
	assert_eq!(
		session_request
			.get("x-secondary-user")
			.unwrap()
			.to_str()
			.unwrap(),
		"user-42"
	);
	assert_eq!(jwt_response.status(), StatusCode::NO_CONTENT);
	let jwt_request = jwt_headers.lock().unwrap().pop().unwrap();
	let authorization = jwt_request.get("authorization").unwrap().to_str().unwrap();
	let token = authorization.strip_prefix("Bearer ").unwrap();
	let claims = JwtAuth::new(config.secret.as_bytes())
		.decode(token)
		.unwrap();
	assert_eq!(claims.sub, "user-42");
	assert_eq!(claims.username, "user-42");
	assert!(!claims.is_staff);
	assert!(!claims.is_superuser);
	assert_eq!(claims.exp - claims.iat, 120);
	assert_eq!(
		jwt_request
			.get("x-secondary-user")
			.unwrap()
			.to_str()
			.unwrap(),
		"user-42"
	);
	assert_eq!(
		failed.to_string(),
		"session backend error: Internal server error: backend unavailable"
	);
}
