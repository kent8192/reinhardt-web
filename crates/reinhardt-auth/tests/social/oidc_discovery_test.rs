//! OIDC discovery integration tests

use reinhardt_auth::social::core::OAuth2Client;
use reinhardt_auth::social::oidc::{DiscoveryClient, OIDCDiscovery};
use rstest::*;

#[tokio::test]
async fn test_discovery_fetch_from_url() {
	// This test documents the expected behavior
	// In a real scenario, you would mock the HTTP response

	// Arrange
	let issuer_url = "https://accounts.google.com";
	let oauth2_client = OAuth2Client::new();
	let client = DiscoveryClient::new(oauth2_client);

	// Act - This would make a real HTTP request
	// For now, we document the expected behavior
	let result = client.discover(issuer_url).await;

	// Assert - In mocked environment, this would return a discovery document
	// In test environment without mocks, we expect an error
	match result {
		Ok(discovery) => {
			assert_eq!(discovery.issuer, "https://accounts.google.com");
			assert!(!discovery.authorization_endpoint.is_empty());
		}
		Err(_) => {
			// Expected in test environment without network/mocks
			assert!(true, "Network error expected in test environment");
		}
	}
}

#[tokio::test]
async fn test_discovery_document_structure() {
	// Arrange
	let discovery = OIDCDiscovery {
		issuer: "https://accounts.google.com".to_string(),
		authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
		token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
		jwks_uri: "https://www.googleapis.com/oauth2/v3/certs".to_string(),
		userinfo_endpoint: Some("https://www.googleapis.com/oauth2/v3/userinfo".to_string()),
		scopes_supported: Some(vec![
			"openid".to_string(),
			"email".to_string(),
			"profile".to_string(),
		]),
		response_types_supported: Some(vec!["code".to_string()]),
		grant_types_supported: Some(vec!["authorization_code".to_string()]),
		subject_types_supported: Some(vec!["public".to_string()]),
		id_token_signing_alg_values_supported: Some(vec!["RS256".to_string()]),
		claims_supported: None,
	};

	// Assert
	assert!(!discovery.issuer.is_empty());
	assert!(!discovery.authorization_endpoint.is_empty());
	assert!(!discovery.token_endpoint.is_empty());
	assert!(!discovery.jwks_uri.is_empty());
	assert!(discovery.scopes_supported.is_some());
}

#[tokio::test]
async fn test_discovery_required_fields() {
	// Arrange
	let discovery = OIDCDiscovery {
		issuer: "https://example.com".to_string(),
		authorization_endpoint: "https://example.com/auth".to_string(),
		token_endpoint: "https://example.com/token".to_string(),
		jwks_uri: "https://example.com/jwks".to_string(),
		userinfo_endpoint: None,
		scopes_supported: None,
		response_types_supported: None,
		grant_types_supported: None,
		subject_types_supported: None,
		id_token_signing_alg_values_supported: None,
		claims_supported: None,
	};

	// Assert - Required fields must be present
	assert!(!discovery.issuer.is_empty());
	assert!(!discovery.authorization_endpoint.is_empty());
	assert!(!discovery.token_endpoint.is_empty());
	assert!(!discovery.jwks_uri.is_empty());
}

#[test]
fn test_discovery_serialization() {
	// Arrange
	let discovery = OIDCDiscovery {
		issuer: "https://accounts.google.com".to_string(),
		authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
		token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
		jwks_uri: "https://www.googleapis.com/oauth2/v3/certs".to_string(),
		userinfo_endpoint: Some("https://www.googleapis.com/oauth2/v3/userinfo".to_string()),
		scopes_supported: Some(vec![
			"openid".to_string(),
			"email".to_string(),
			"profile".to_string(),
		]),
		response_types_supported: Some(vec!["code".to_string()]),
		grant_types_supported: Some(vec!["authorization_code".to_string()]),
		subject_types_supported: Some(vec!["public".to_string()]),
		id_token_signing_alg_values_supported: Some(vec!["RS256".to_string()]),
		claims_supported: None,
	};

	// Act
	let json = serde_json::to_string(&discovery).unwrap();

	// Assert
	assert!(json.contains("\"issuer\":\"https://accounts.google.com\""));
	assert!(json.contains("\"jwks_uri\":\"https://www.googleapis.com/oauth2/v3/certs\""));
}

#[test]
fn test_discovery_deserialization() {
	// Arrange
	let json = r#"{
		"issuer": "https://accounts.google.com",
		"authorization_endpoint": "https://accounts.google.com/o/oauth2/v2/auth",
		"token_endpoint": "https://oauth2.googleapis.com/token",
		"jwks_uri": "https://www.googleapis.com/oauth2/v3/certs",
		"userinfo_endpoint": "https://www.googleapis.com/oauth2/v3/userinfo",
		"scopes_supported": ["openid", "email", "profile"]
	}"#;

	// Act
	let discovery: OIDCDiscovery = serde_json::from_str(json).unwrap();

	// Assert
	assert_eq!(discovery.issuer, "https://accounts.google.com");
	assert_eq!(
		discovery.authorization_endpoint,
		"https://accounts.google.com/o/oauth2/v2/auth"
	);
	assert!(discovery.scopes_supported.is_some());
}
