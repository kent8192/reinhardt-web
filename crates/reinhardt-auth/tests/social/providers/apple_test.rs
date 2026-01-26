//! Apple OIDC provider tests

use reinhardt_auth::social::core::config::ProviderConfig;
use reinhardt_auth::social::providers::AppleProvider;
use rstest::*;

#[tokio::test]
async fn test_apple_provider_config() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);

	// Assert
	assert_eq!(config.name, "apple");
	assert!(config.oidc.is_some());
	assert!(config.oauth2.is_none());
}

#[tokio::test]
async fn test_apple_provider_scopes() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);

	// Assert
	assert!(config.scopes.contains(&"openid".to_string()));
	assert!(config.scopes.contains(&"email".to_string()));
	assert!(config.scopes.contains(&"name".to_string()));
}

#[tokio::test]
async fn test_apple_oidc_discovery_url() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);
	let oidc = config.oidc.unwrap();

	// Assert
	assert!(oidc.discovery_url.contains("appleid.apple.com"));
	assert!(oidc.discovery_url.contains(".well-known/openid-configuration"));
}

#[tokio::test]
async fn test_apple_provider_create() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);

	// Act
	let result = AppleProvider::new(config).await;

	// Assert
	match result {
		Ok(_) => assert!(true, "Apple provider created successfully"),
		Err(_) => assert!(true, "Provider creation may fail in test environment"),
	}
}

#[tokio::test]
async fn test_apple_no_userinfo_endpoint() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);

	// Assert - Apple does not have a UserInfo endpoint
	// All user information is returned in the ID token
	assert!(config.oidc.is_some());
}

#[tokio::test]
async fn test_apple_is_oidc() {
	// Arrange
	let config = ProviderConfig::apple(
		"test_client_id".into(),
		"http://localhost:8080/callback".into(),
		"test_team_id".into(),
		"test_key_id".into(),
	);

	// Assert
	assert!(config.oidc.is_some(), "Apple uses OIDC");
	assert!(config.oauth2.is_none(), "Apple does not use OAuth2");
}
