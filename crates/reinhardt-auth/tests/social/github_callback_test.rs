//! `SocialAuthBackend` callback tests against a mocked GitHub API.

use reinhardt_auth::social::core::config::{OAuth2Config, ProviderConfig};
use reinhardt_auth::social::providers::GitHubProvider;
use reinhardt_auth::social::{SocialAuthBackend, SocialAuthError};
use rstest::*;
use serde_json::{Value, json};
use std::sync::Arc;
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A backend with a `GitHubProvider` whose token endpoint succeeds, plus the
/// state of a started authorization flow.
struct GitHubEnv {
	server: MockServer,
	backend: SocialAuthBackend,
	state: String,
}

#[fixture]
async fn github_env() -> GitHubEnv {
	let server = MockServer::start().await;
	Mock::given(method("POST"))
		.and(path("/login/oauth/access_token"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"access_token": "ghu_user_token",
			"token_type": "bearer",
			"expires_in": 28800,
			"refresh_token": "ghr_refresh_token"
		})))
		.mount(&server)
		.await;

	let config = ProviderConfig {
		scopes: Vec::new(),
		oauth2: Some(OAuth2Config {
			authorization_endpoint: format!("{}/login/oauth/authorize", server.uri()),
			token_endpoint: format!("{}/login/oauth/access_token", server.uri()),
			userinfo_endpoint: Some(format!("{}/user", server.uri())),
		}),
		..ProviderConfig::github(
			"client_id".to_string(),
			"client_secret".to_string(),
			"http://127.0.0.1/callback".to_string(),
		)
	};
	let mut backend = SocialAuthBackend::new();
	backend.register_provider(Arc::new(GitHubProvider::new(config).await.unwrap()));
	let authorization = backend.begin_auth("github", None, None).await.unwrap();

	GitHubEnv {
		server,
		backend,
		state: authorization.state,
	}
}

#[rstest]
#[tokio::test]
async fn callback_exposes_github_login_alongside_name(#[future] github_env: GitHubEnv) {
	// Arrange
	let env = github_env.await;
	Mock::given(method("GET"))
		.and(path("/user"))
		.and(bearer_token("ghu_user_token"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"id": 1,
			"login": "octocat",
			"name": "The Octocat",
			"email": null,
			"avatar_url": "https://avatars.githubusercontent.com/u/1"
		})))
		.expect(1)
		.mount(&env.server)
		.await;

	// Act
	let callback = env
		.backend
		.handle_callback("github", "code", &env.state)
		.await
		.unwrap();

	// Assert
	assert_eq!(callback.claims.sub, "1");
	assert_eq!(callback.claims.name.as_deref(), Some("The Octocat"));
	assert_eq!(
		callback.claims.additional_claims.get("login"),
		Some(&Value::String("octocat".to_string()))
	);
	assert_eq!(callback.token_response.access_token, "ghu_user_token");
}

#[rstest]
#[tokio::test]
async fn callback_fails_when_github_user_lookup_is_unauthorized(#[future] github_env: GitHubEnv) {
	// Arrange
	let env = github_env.await;
	Mock::given(method("GET"))
		.and(path("/user"))
		.respond_with(ResponseTemplate::new(401).set_body_string("Bad credentials"))
		.expect(1)
		.mount(&env.server)
		.await;

	// Act
	let result = env
		.backend
		.handle_callback("github", "code", &env.state)
		.await;

	// Assert
	assert_eq!(
		result.err(),
		Some(SocialAuthError::UserInfoError(
			"GitHub UserInfo request failed (401 Unauthorized): Bad credentials".to_string()
		))
	);
}
