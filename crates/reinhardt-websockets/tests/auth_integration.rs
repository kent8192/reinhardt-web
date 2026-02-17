//! WebSocket authentication integration tests
//!
//! Tests Cookie, Token, and Query authentication flows with proper mocking.

use reinhardt_websockets::auth::{
	AuthError, AuthResult, AuthUser, AuthenticatedConnection, AuthorizationPolicy,
	PermissionBasedPolicy, SimpleAuthUser, TokenAuthenticator, WebSocketAuthenticator,
};
use reinhardt_websockets::{Message, WebSocketConnection};
use rstest::rstest;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Mock authenticator for testing
struct MockAuthenticator {
	valid_tokens: std::collections::HashMap<String, SimpleAuthUser>,
}

impl MockAuthenticator {
	fn new() -> Self {
		let mut valid_tokens = std::collections::HashMap::new();
		valid_tokens.insert(
			"valid_token_123".to_string(),
			SimpleAuthUser::new(
				"user_1".to_string(),
				"alice".to_string(),
				vec!["chat.read".to_string(), "chat.write".to_string()],
			),
		);
		valid_tokens.insert(
			"admin_token_456".to_string(),
			SimpleAuthUser::new(
				"user_2".to_string(),
				"bob".to_string(),
				vec![
					"chat.read".to_string(),
					"chat.write".to_string(),
					"chat.admin".to_string(),
				],
			),
		);
		Self { valid_tokens }
	}

	fn authenticate_token(&self, token: &str) -> AuthResult<SimpleAuthUser> {
		self.valid_tokens
			.get(token)
			.cloned()
			.ok_or(AuthError::InvalidCredentials)
	}
}

/// Test: Token-based authentication - valid token
#[rstest]
#[tokio::test]
async fn test_token_authentication_valid() {
	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.read".to_string()],
	);

	let authenticator = TokenAuthenticator::new(vec![("token123".to_string(), user)]);

	let (tx, _rx) = mpsc::unbounded_channel();
	let conn = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx));

	let auth_user = authenticator.authenticate(&conn, "token123").await.unwrap();
	assert_eq!(auth_user.username(), "alice");
	assert!(auth_user.is_authenticated());
}

/// Test: Token-based authentication - invalid token
#[rstest]
#[tokio::test]
async fn test_token_authentication_invalid() {
	let authenticator = TokenAuthenticator::new(vec![]);

	let (tx, _rx) = mpsc::unbounded_channel();
	let conn = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx));

	let result = authenticator.authenticate(&conn, "invalid_token").await;
	assert!(result.is_err());
	assert!(matches!(result.unwrap_err(), AuthError::InvalidCredentials));
}

/// Test: Token-based authentication - multiple users
#[rstest]
#[tokio::test]
async fn test_token_authentication_multiple_users() {
	let user1 = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.read".to_string()],
	);
	let user2 = SimpleAuthUser::new(
		"user_2".to_string(),
		"bob".to_string(),
		vec!["chat.admin".to_string()],
	);

	let authenticator = TokenAuthenticator::new(vec![
		("token_alice".to_string(), user1),
		("token_bob".to_string(), user2),
	]);

	let (tx1, _rx1) = mpsc::unbounded_channel();
	let conn1 = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx1));

	let (tx2, _rx2) = mpsc::unbounded_channel();
	let conn2 = Arc::new(WebSocketConnection::new("conn_2".to_string(), tx2));

	let auth_user1 = authenticator
		.authenticate(&conn1, "token_alice")
		.await
		.unwrap();
	assert_eq!(auth_user1.username(), "alice");
	assert!(auth_user1.has_permission("chat.read"));

	let auth_user2 = authenticator
		.authenticate(&conn2, "token_bob")
		.await
		.unwrap();
	assert_eq!(auth_user2.username(), "bob");
	assert!(auth_user2.has_permission("chat.admin"));
}

/// Test: Permission-based authorization - authorized action
#[rstest]
#[tokio::test]
async fn test_authorization_policy_authorized() {
	let policy =
		PermissionBasedPolicy::new(vec![("send_message".to_string(), "chat.write".to_string())]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let result = policy.authorize(&user, "send_message", None).await;
	assert!(result.is_ok());
}

/// Test: Permission-based authorization - denied action
#[rstest]
#[tokio::test]
async fn test_authorization_policy_denied() {
	let policy = PermissionBasedPolicy::new(vec![(
		"delete_message".to_string(),
		"chat.admin".to_string(),
	)]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let result = policy.authorize(&user, "delete_message", None).await;
	assert!(result.is_err());
	assert!(matches!(
		result.unwrap_err(),
		AuthError::AuthorizationDenied(_)
	));
}

/// Test: Permission-based authorization - unknown action
#[rstest]
#[tokio::test]
async fn test_authorization_policy_unknown_action() {
	let policy = PermissionBasedPolicy::new(vec![]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let result = policy.authorize(&user, "unknown_action", None).await;
	assert!(result.is_err());
	assert!(matches!(
		result.unwrap_err(),
		AuthError::AuthorizationDenied(_)
	));
}

/// Test: AuthenticatedConnection - send with authorization
#[rstest]
#[tokio::test]
async fn test_authenticated_connection_send_with_auth_success() {
	let policy =
		PermissionBasedPolicy::new(vec![("send_message".to_string(), "chat.write".to_string())]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let (tx, mut rx) = mpsc::unbounded_channel();
	let conn = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx));
	let auth_conn = AuthenticatedConnection::new(conn, Box::new(user));

	let msg = Message::text("Hello from authenticated user".to_string());
	auth_conn.send_with_auth(msg, &policy).await.unwrap();

	let received = rx.recv().await.unwrap();
	match received {
		Message::Text { data } => assert_eq!(data, "Hello from authenticated user"),
		_ => panic!("Expected text message"),
	}
}

/// Test: AuthenticatedConnection - send with authorization denied
#[rstest]
#[tokio::test]
async fn test_authenticated_connection_send_with_auth_denied() {
	let policy =
		PermissionBasedPolicy::new(vec![("send_message".to_string(), "chat.admin".to_string())]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let (tx, _rx) = mpsc::unbounded_channel();
	let conn = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx));
	let auth_conn = AuthenticatedConnection::new(conn, Box::new(user));

	let msg = Message::text("Hello".to_string());
	let result = auth_conn.send_with_auth(msg, &policy).await;

	assert!(result.is_err());
}

/// Test: Mock authenticator - valid token
#[rstest]
#[tokio::test]
async fn test_mock_authenticator_valid_token() {
	let mock_auth = MockAuthenticator::new();

	let user = mock_auth.authenticate_token("valid_token_123").unwrap();
	assert_eq!(user.username(), "alice");
	assert!(user.has_permission("chat.read"));
	assert!(user.has_permission("chat.write"));
	assert!(!user.has_permission("chat.admin"));
}

/// Test: Mock authenticator - admin token
#[rstest]
#[tokio::test]
async fn test_mock_authenticator_admin_token() {
	let mock_auth = MockAuthenticator::new();

	let user = mock_auth.authenticate_token("admin_token_456").unwrap();
	assert_eq!(user.username(), "bob");
	assert!(user.has_permission("chat.admin"));
}

/// Test: Mock authenticator - invalid token
#[rstest]
#[tokio::test]
async fn test_mock_authenticator_invalid_token() {
	let mock_auth = MockAuthenticator::new();

	let result = mock_auth.authenticate_token("invalid_token");
	assert!(result.is_err());
	assert!(matches!(result.unwrap_err(), AuthError::InvalidCredentials));
}

/// Test: User permission checks
#[rstest]
#[tokio::test]
async fn test_user_permission_checks() {
	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["read".to_string(), "write".to_string()],
	);

	assert!(user.is_authenticated());
	assert!(user.has_permission("read"));
	assert!(user.has_permission("write"));
	assert!(!user.has_permission("admin"));
	assert!(!user.has_permission("delete"));
}

/// Test: Multiple permission levels
#[rstest]
#[tokio::test]
async fn test_multiple_permission_levels() {
	let policy = PermissionBasedPolicy::new(vec![
		("read_message".to_string(), "chat.read".to_string()),
		("send_message".to_string(), "chat.write".to_string()),
		("delete_message".to_string(), "chat.admin".to_string()),
	]);

	// Regular user
	let regular_user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.read".to_string(), "chat.write".to_string()],
	);

	assert!(
		policy
			.authorize(&regular_user, "read_message", None)
			.await
			.is_ok()
	);
	assert!(
		policy
			.authorize(&regular_user, "send_message", None)
			.await
			.is_ok()
	);
	assert!(
		policy
			.authorize(&regular_user, "delete_message", None)
			.await
			.is_err()
	);

	// Admin user
	let admin_user = SimpleAuthUser::new(
		"user_2".to_string(),
		"bob".to_string(),
		vec![
			"chat.read".to_string(),
			"chat.write".to_string(),
			"chat.admin".to_string(),
		],
	);

	assert!(
		policy
			.authorize(&admin_user, "read_message", None)
			.await
			.is_ok()
	);
	assert!(
		policy
			.authorize(&admin_user, "send_message", None)
			.await
			.is_ok()
	);
	assert!(
		policy
			.authorize(&admin_user, "delete_message", None)
			.await
			.is_ok()
	);
}

/// Test: AuthenticatedConnection - multiple messages with authorization
#[rstest]
#[tokio::test]
async fn test_authenticated_connection_multiple_messages() {
	let policy =
		PermissionBasedPolicy::new(vec![("send_message".to_string(), "chat.write".to_string())]);

	let user = SimpleAuthUser::new(
		"user_1".to_string(),
		"alice".to_string(),
		vec!["chat.write".to_string()],
	);

	let (tx, mut rx) = mpsc::unbounded_channel();
	let conn = Arc::new(WebSocketConnection::new("conn_1".to_string(), tx));
	let auth_conn = AuthenticatedConnection::new(conn, Box::new(user));

	// Send multiple messages
	for i in 0..5 {
		let msg = Message::text(format!("Message {}", i));
		auth_conn.send_with_auth(msg, &policy).await.unwrap();
	}

	// Verify all messages were sent
	for i in 0..5 {
		let received = rx.recv().await.unwrap();
		match received {
			Message::Text { data } => assert_eq!(data, format!("Message {}", i)),
			_ => panic!("Expected text message"),
		}
	}
}

/// Test: User without authentication flag
#[rstest]
#[tokio::test]
async fn test_unauthenticated_user() {
	// User with empty ID is considered unauthenticated
	let user = SimpleAuthUser::new("".to_string(), "guest".to_string(), vec![]);

	assert!(!user.is_authenticated());
	assert_eq!(user.username(), "guest");
	assert!(!user.has_permission("chat.read"));
}
