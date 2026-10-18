//! Redis channel layer integration tests against a real Redis server.
//!
//! The server rejects unauthenticated clients and the connection URL carries no
//! credentials, so these tests only pass when the dedicated `username` and
//! `password` fields of `RedisConfig` govern the effective connection.

#![cfg(feature = "redis-channel")]
#![allow(deprecated)] // `RedisConfig` is deprecated in favor of `RedisChannelSettings`.

use reinhardt_websockets::Message;
use reinhardt_websockets::channels::{ChannelError, ChannelLayer, ChannelMessage};
use reinhardt_websockets::redis_channel::{RedisChannelLayer, RedisConfig};
use rstest::{fixture, rstest};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

const DEFAULT_USER_PASSWORD: &str = "default-user-password";
const ACL_USERNAME: &str = "channel_user";
const ACL_PASSWORD: &str = "channel-user-password";

/// A Redis server that requires authentication, plus a credential-free URL to it.
///
/// Dropping the fixture stops and removes the container.
struct AuthenticatedRedis {
	_container: ContainerAsync<GenericImage>,
	url: String,
}

#[fixture]
async fn authenticated_redis() -> AuthenticatedRedis {
	let acl_password = format!(">{ACL_PASSWORD}");
	let container = GenericImage::new("redis", "7-alpine")
		.with_exposed_port(6379.tcp())
		.with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
		.with_cmd([
			"redis-server",
			"--requirepass",
			DEFAULT_USER_PASSWORD,
			"--user",
			ACL_USERNAME,
			"on",
			acl_password.as_str(),
			"~*",
			"&*",
			"+@all",
		])
		.start()
		.await
		.expect("failed to start Redis container");
	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("failed to resolve Redis port");

	AuthenticatedRedis {
		_container: container,
		url: format!("redis://127.0.0.1:{port}"),
	}
}

/// Sends one message through `layer` and returns the message received back.
async fn round_trip(layer: &RedisChannelLayer) -> Option<ChannelMessage> {
	let message = ChannelMessage::new("user_1".to_string(), Message::text("hello".to_string()));
	layer
		.send("ghsa_channel", message)
		.await
		.expect("send through authenticated connection");
	layer
		.receive("ghsa_channel")
		.await
		.expect("receive through authenticated connection")
}

#[rstest]
#[tokio::test]
async fn test_password_field_authenticates_connection(
	#[future] authenticated_redis: AuthenticatedRedis,
) {
	// Arrange
	let redis = authenticated_redis.await;
	let config =
		RedisConfig::new(redis.url.clone()).with_password(DEFAULT_USER_PASSWORD.to_string());

	// Act
	let layer = RedisChannelLayer::new(config)
		.await
		.expect("connect with the password field");
	let received = round_trip(&layer).await;

	// Assert
	assert_eq!(
		received.map(|message| message.sender().to_string()),
		Some("user_1".to_string())
	);
}

#[rstest]
#[tokio::test]
async fn test_username_and_password_fields_authenticate_acl_user(
	#[future] authenticated_redis: AuthenticatedRedis,
) {
	// Arrange
	let redis = authenticated_redis.await;
	let config = RedisConfig::new(redis.url.clone())
		.with_username(ACL_USERNAME.to_string())
		.with_password(ACL_PASSWORD.to_string());

	// Act
	let layer = RedisChannelLayer::new(config)
		.await
		.expect("connect with the username and password fields");
	let received = round_trip(&layer).await;

	// Assert
	assert_eq!(
		received.map(|message| message.sender().to_string()),
		Some("user_1".to_string())
	);
}

#[rstest]
#[tokio::test]
async fn test_wrong_password_field_is_rejected(#[future] authenticated_redis: AuthenticatedRedis) {
	// Arrange
	let redis = authenticated_redis.await;
	let config = RedisConfig::new(redis.url.clone()).with_password("wrong-password".to_string());

	// Act
	let result = RedisChannelLayer::new(config).await;

	// Assert
	match result {
		Err(ChannelError::SendError(message)) => {
			assert!(message.starts_with("Redis connection error:"), "{message}");
			assert!(!message.contains("wrong-password"), "{message}");
		}
		Err(other) => panic!("expected a connection error, got {other:?}"),
		Ok(_) => panic!("a wrong password must not authenticate"),
	}
}
