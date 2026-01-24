//! Integration with reinhardt-pages
//!
//! This module provides integration between reinhardt-websockets and reinhardt-pages,
//! enabling WebSocket connections to use the same authentication and session management
//! as the HTTP layer.
//!
//! ## Overview
//!
//! This integration allows WebSocket connections to authenticate using the same
//! Cookie/session-based authentication as reinhardt-pages HTTP requests. Cookies
//! from the user's browser session are automatically included in the WebSocket
//! handshake, allowing the server to authenticate the connection.
//!
//! ## Server-Side Setup
//!
//! On the server side, use [`PagesAuthenticator`] to validate WebSocket connections
//! using session cookies:
//!
//! ```ignore
//! use reinhardt_websockets::integration::pages::PagesAuthenticator;
//! use reinhardt_websockets::{WebSocketConsumer, WebSocketConnection, Message};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! pub struct ChatHandler {
//!     authenticator: Arc<PagesAuthenticator>,
//! }
//!
//! #[async_trait]
//! impl WebSocketConsumer for ChatHandler {
//!     async fn on_connect(&self, connection: &Arc<WebSocketConnection>) -> WebSocketResult<()> {
//!         // Extract cookies from handshake headers
//!         let cookies = "sessionid=abc123; csrftoken=xyz789"; // From HTTP headers
//!
//!         // Authenticate using session cookies
//!         let user = self.authenticator
//!             .authenticate_from_cookies(cookies)
//!             .await?;
//!
//!         log::info!("User {} connected", user.username());
//!         Ok(())
//!     }
//!
//!     async fn on_message(
//!         &self,
//!         connection: &Arc<WebSocketConnection>,
//!         message: Message,
//!     ) -> WebSocketResult<()> {
//!         // Handle message
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Client-Side Setup
//!
//! On the client side (WASM), use the `use_websocket` hook from reinhardt-pages:
//!
//! ```ignore
//! use reinhardt_pages::reactive::hooks::{use_websocket, use_effect, UseWebSocketOptions};
//! use reinhardt_pages::reactive::hooks::{ConnectionState, WebSocketMessage};
//!
//! fn chat_component(room_id: String) -> View {
//!     let ws = use_websocket(
//!         &format!("ws://localhost:8000/ws/chat/{}", room_id),
//!         UseWebSocketOptions::default()
//!     );
//!
//!     use_effect({
//!         let ws = ws.clone();
//!         move || {
//!             if let Some(WebSocketMessage::Text(text)) = ws.latest_message().get() {
//!                 log!("Received: {}", text);
//!             }
//!             None::<fn()>
//!         }
//!     });
//!
//!     page!(|| {
//!         div {
//!             button {
//!                 @click: move |_| {
//!                     ws.send_text("Hello!".to_string()).ok();
//!                 },
//!                 "Send"
//!             }
//!         }
//!     })()
//! }
//! ```
//!
//! ## Authentication Flow
//!
//! 1. User authenticates via HTTP (Cookie/session created by reinhardt-pages)
//! 2. User's browser stores session cookie
//! 3. Client-side JavaScript initiates WebSocket connection
//! 4. Browser automatically includes cookies in WebSocket handshake
//! 5. Server extracts session ID from cookies using [`PagesAuthenticator::authenticate_from_cookies`]
//! 6. Server validates session and retrieves user information
//! 7. WebSocket connection is authenticated and associated with the user

use crate::auth::{AuthError, AuthResult, AuthUser, WebSocketAuthenticator};
use crate::connection::WebSocketConnection;
use async_trait::async_trait;
use std::sync::Arc;

/// User authenticated from reinhardt-pages session
///
/// This struct wraps user information extracted from reinhardt-pages'
/// Cookie/session-based authentication system.
#[derive(Debug, Clone)]
pub struct PagesAuthUser {
	/// User ID
	pub user_id: String,
	/// Username
	pub username: String,
	/// User permissions
	pub permissions: Vec<String>,
	/// Whether the user is a superuser
	pub is_superuser: bool,
}

impl AuthUser for PagesAuthUser {
	fn id(&self) -> &str {
		&self.user_id
	}

	fn username(&self) -> &str {
		&self.username
	}

	fn is_authenticated(&self) -> bool {
		!self.user_id.is_empty()
	}

	fn has_permission(&self, permission: &str) -> bool {
		self.is_superuser || self.permissions.contains(&permission.to_string())
	}
}

/// Authenticator that integrates with reinhardt-pages' Cookie/session authentication
///
/// This authenticator extracts session information from WebSocket handshake cookies
/// and validates them against reinhardt-pages' session store.
///
/// # Example
///
/// ```ignore
/// use reinhardt_websockets::integration::pages::PagesAuthenticator;
///
/// let authenticator = PagesAuthenticator::new();
///
/// // During WebSocket handshake, extract cookies from HTTP headers
/// let cookie_header = "sessionid=abc123; csrftoken=xyz789";
/// let user = authenticator.authenticate_from_cookies(cookie_header).await?;
/// ```
pub struct PagesAuthenticator {
	// Session store reference will be added when reinhardt-pages session management is ready
	// See: https://github.com/kent8192/reinhardt-web/issues/22
}

impl PagesAuthenticator {
	/// Create a new PagesAuthenticator
	pub fn new() -> Self {
		Self {}
	}

	/// Authenticate a user from Cookie header string
	///
	/// This method extracts the session ID from the Cookie header and validates it
	/// against the session store to retrieve user information.
	///
	/// # Arguments
	///
	/// * `cookies` - Cookie header string (e.g., "sessionid=abc123; csrftoken=xyz789")
	///
	/// # Returns
	///
	/// Returns the authenticated user on success, or an error if:
	/// - Session ID is missing or invalid
	/// - Session has expired
	/// - User does not exist
	///
	/// # Errors
	///
	/// Returns `AuthError::AuthenticationFailed` if authentication fails or if
	/// session store integration is not yet available.
	///
	/// # Current Limitations
	///
	/// Session store integration with reinhardt-pages is not yet implemented.
	/// This method will return an error indicating the feature is unavailable.
	/// Track progress at: <https://github.com/kent8192/reinhardt-web/issues/22>
	pub async fn authenticate_from_cookies(&self, cookies: &str) -> AuthResult<Box<dyn AuthUser>> {
		// Extract session ID from cookies
		let session_id = self.extract_session_id(cookies)?;

		// Session store integration is not yet implemented
		// See: https://github.com/kent8192/reinhardt-web/issues/22
		let _ = session_id; // Acknowledge session ID was extracted
		Err(AuthError::AuthenticationFailed(
			"Session store integration not yet available. \
			 This feature requires reinhardt-pages session management. \
			 See https://github.com/kent8192/reinhardt-web/issues/22 for progress."
				.to_string(),
		))
	}

	/// Extract session ID from Cookie header
	///
	/// Parses the Cookie header string and extracts the session ID.
	///
	/// # Arguments
	///
	/// * `cookies` - Cookie header string
	///
	/// # Returns
	///
	/// Returns the session ID if found, or an error if not found.
	fn extract_session_id(&self, cookies: &str) -> AuthResult<String> {
		// Parse cookies and look for sessionid
		for cookie in cookies.split(';') {
			let cookie = cookie.trim();
			if let Some((name, value)) = cookie.split_once('=')
				&& name.trim() == "sessionid"
			{
				return Ok(value.trim().to_string());
			}
		}

		Err(AuthError::AuthenticationFailed(
			"Session ID not found in cookies".to_string(),
		))
	}
}

impl Default for PagesAuthenticator {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl WebSocketAuthenticator for PagesAuthenticator {
	async fn authenticate(
		&self,
		_connection: &Arc<WebSocketConnection>,
		credentials: &str,
	) -> AuthResult<Box<dyn AuthUser>> {
		// credentials parameter contains the Cookie header string
		self.authenticate_from_cookies(credentials).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pages_auth_user_creation() {
		let user = PagesAuthUser {
			user_id: "user_1".to_string(),
			username: "alice".to_string(),
			permissions: vec!["chat.read".to_string(), "chat.write".to_string()],
			is_superuser: false,
		};

		assert_eq!(user.id(), "user_1");
		assert_eq!(user.username(), "alice");
		assert!(user.is_authenticated());
		assert!(user.has_permission("chat.read"));
		assert!(user.has_permission("chat.write"));
		assert!(!user.has_permission("admin.delete"));
	}

	#[test]
	fn test_pages_auth_user_superuser() {
		let user = PagesAuthUser {
			user_id: "admin_1".to_string(),
			username: "admin".to_string(),
			permissions: vec![],
			is_superuser: true,
		};

		// Superuser has all permissions
		assert!(user.has_permission("any.permission"));
		assert!(user.has_permission("admin.delete"));
	}

	#[test]
	fn test_extract_session_id_success() {
		let auth = PagesAuthenticator::new();
		let cookies = "sessionid=abc123; csrftoken=xyz789";
		let session_id = auth.extract_session_id(cookies).unwrap();
		assert_eq!(session_id, "abc123");
	}

	#[test]
	fn test_extract_session_id_with_spaces() {
		let auth = PagesAuthenticator::new();
		let cookies = "sessionid = abc123 ; csrftoken = xyz789";
		let session_id = auth.extract_session_id(cookies).unwrap();
		assert_eq!(session_id, "abc123");
	}

	#[test]
	fn test_extract_session_id_not_found() {
		let auth = PagesAuthenticator::new();
		let cookies = "csrftoken=xyz789; other=value";
		let result = auth.extract_session_id(cookies);
		assert!(result.is_err());
	}

	#[test]
	fn test_extract_session_id_empty_cookies() {
		let auth = PagesAuthenticator::new();
		let cookies = "";
		let result = auth.extract_session_id(cookies);
		assert!(result.is_err());
	}

	#[test]
	fn test_pages_authenticator_default() {
		let auth = PagesAuthenticator::default();
		let cookies = "sessionid=test123";
		let session_id = auth.extract_session_id(cookies).unwrap();
		assert_eq!(session_id, "test123");
	}

	#[tokio::test]
	async fn test_authenticate_from_cookies_returns_error() {
		let auth = PagesAuthenticator::new();
		let cookies = "sessionid=valid_session_id";
		let result = auth.authenticate_from_cookies(cookies).await;

		// Should return error indicating feature not available
		assert!(result.is_err());
		let err = result.unwrap_err();
		match err {
			AuthError::AuthenticationFailed(msg) => {
				assert!(msg.contains("Session store integration not yet available"));
				assert!(msg.contains("https://github.com/kent8192/reinhardt-web/issues/22"));
			}
			_ => panic!("Expected AuthenticationFailed error"),
		}
	}

	#[tokio::test]
	async fn test_authenticate_from_cookies_missing_session_id() {
		let auth = PagesAuthenticator::new();
		let cookies = "csrftoken=xyz789"; // No sessionid
		let result = auth.authenticate_from_cookies(cookies).await;

		// Should return error for missing session ID
		assert!(result.is_err());
		let err = result.unwrap_err();
		match err {
			AuthError::AuthenticationFailed(msg) => {
				assert!(msg.contains("Session ID not found"));
			}
			_ => panic!("Expected AuthenticationFailed error"),
		}
	}
}
