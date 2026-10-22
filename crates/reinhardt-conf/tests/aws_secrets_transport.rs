//! AWS custom endpoints use the same HTTP transport as the default SDK client.

#![cfg(feature = "aws-secrets")]

use reinhardt_conf::settings::secrets::SecretProvider;
use reinhardt_conf::settings::secrets::providers::aws::AwsSecretsProvider;
use rstest::{fixture, rstest};
use serde_json::json;
use wiremock::matchers::{body_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[fixture]
async fn secrets_endpoint() -> MockServer {
	MockServer::start().await
}

#[rstest]
#[tokio::test]
async fn custom_http_endpoint_returns_the_requested_secret(#[future] secrets_endpoint: MockServer) {
	// Arrange
	let server = secrets_endpoint.await;
	Mock::given(method("POST"))
		.and(header("x-amz-target", "secretsmanager.GetSecretValue"))
		.and(body_json(json!({"SecretId": "app/database/password"})))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"SecretString": "{\"password\":\"database-password\"}"
		})))
		.expect(1)
		.mount(&server)
		.await;
	let provider = AwsSecretsProvider::with_endpoint(server.uri(), Some("app/".into()))
		.await
		.expect("construct the custom endpoint provider");

	// Act
	let secret = provider
		.get_secret("database/password")
		.await
		.expect("read the secret over the SDK's HTTP transport");

	// Assert
	assert_eq!(secret.expose_secret(), "database-password");
}
