//! Token refresh flow tests

use helpers::mock_server::MockOAuth2Server;
use reinhardt_auth::social::core::config::ProviderConfig;
use reinhardt_auth::social::core::{OAuth2Client, SocialAuthError};
use reinhardt_auth::social::flow::RefreshFlow;
use rstest::rstest;

#[path = "../../helpers.rs"]
mod helpers;

#[rstest]
#[tokio::test]
async fn test_token_refresh_with_mock_server() {
	// Arrange
	let server = MockOAuth2Server::new().await;
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = RefreshFlow::new(OAuth2Client::new(), config);

	// Act
	let result = flow
		.refresh(&server.token_url(), "test_refresh_token")
		.await;

	// Assert
	assert!(
		result.is_ok(),
		"Token refresh should succeed with mock server"
	);
	let response = result.unwrap();
	assert!(!response.access_token.is_empty());
	assert_eq!(response.token_type, "Bearer");
}

#[rstest]
#[tokio::test]
async fn test_token_refresh_server_error() {
	// Arrange
	let mut server = MockOAuth2Server::new().await;
	server.set_error_mode(helpers::mock_server::ErrorMode::ServerError);
	let mut config = ProviderConfig::github(
		"test_client_id".into(),
		"test_client_secret".into(),
		"http://localhost:8080/callback".into(),
	);
	config.oauth2.as_mut().unwrap().token_endpoint = server.token_url();
	let flow = RefreshFlow::new(OAuth2Client::new(), config);

	// Act
	let result = flow
		.refresh(&server.token_url(), "test_refresh_token")
		.await;

	// Assert
	assert!(
		matches!(result, Err(SocialAuthError::TokenRefreshError(message)) if message.contains("500")),
		"Token refresh should report the mock server error"
	);
}
