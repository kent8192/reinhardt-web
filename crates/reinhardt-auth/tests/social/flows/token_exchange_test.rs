//! Token exchange flow tests

use helpers::mock_server::MockOAuth2Server;
use reinhardt_auth::social::core::config::ProviderConfig;
use reinhardt_auth::social::core::{OAuth2Client, SocialAuthError};
use reinhardt_auth::social::flow::TokenExchangeFlow;
use rstest::rstest;

#[path = "../../helpers.rs"]
mod helpers;

#[rstest]
#[tokio::test]
async fn test_token_exchange_with_mock_server() {
	// Arrange
	let server = MockOAuth2Server::new().await;
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = TokenExchangeFlow::new(OAuth2Client::new(), config);

	// Act
	let result = flow
		.exchange(&server.token_url(), "test_authorization_code", None)
		.await;

	// Assert
	assert!(
		result.is_ok(),
		"Token exchange should succeed with mock server"
	);
	let response = result.unwrap();
	assert_eq!(response.access_token, "test_access_token");
	assert_eq!(response.token_type, "Bearer");
}

#[rstest]
#[tokio::test]
async fn test_token_exchange_with_pkce() {
	// Arrange
	let server = MockOAuth2Server::new().await;
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = TokenExchangeFlow::new(OAuth2Client::new(), config);
	let (verifier, _challenge) = reinhardt_auth::social::flow::PkceFlow::generate();

	// Act
	let result = flow
		.exchange(
			&server.token_url(),
			"test_authorization_code",
			Some(&verifier),
		)
		.await;

	// Assert
	assert!(result.is_ok(), "Token exchange with PKCE should succeed");
	let response = result.unwrap();
	assert!(!response.access_token.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_token_exchange_server_error() {
	// Arrange
	let mut server = MockOAuth2Server::new().await;
	server.set_error_mode(helpers::mock_server::ErrorMode::ServerError);
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = TokenExchangeFlow::new(OAuth2Client::new(), config);

	// Act
	let result = flow.exchange(&server.token_url(), "test_code", None).await;

	// Assert
	assert!(
		matches!(result, Err(SocialAuthError::TokenExchangeError(message)) if message.contains("500")),
		"Token exchange should report the mock server error"
	);
}

#[rstest]
#[tokio::test]
async fn test_token_exchange_invalid_response() {
	// Arrange
	let mut server = MockOAuth2Server::new().await;
	server.set_error_mode(helpers::mock_server::ErrorMode::InvalidResponse);
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = TokenExchangeFlow::new(OAuth2Client::new(), config);

	// Act
	let result = flow.exchange(&server.token_url(), "test_code", None).await;

	// Assert
	assert!(
		matches!(result, Err(SocialAuthError::TokenExchangeError(_))),
		"Token exchange should fail while parsing the mock response"
	);
}
