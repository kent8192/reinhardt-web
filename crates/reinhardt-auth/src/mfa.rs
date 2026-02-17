//! Multi-Factor Authentication (MFA)
//!
//! Provides TOTP (Time-based One-Time Password) support for MFA.

use crate::{AuthenticationBackend, AuthenticationError, SimpleUser, User};
use reinhardt_http::Request;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// MFA authentication backend
///
/// Provides Time-based One-Time Password (TOTP) authentication.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::MfaManager;
///
/// let mfa = MfaManager::new("MyApp");
/// ```
pub struct MFAAuthentication {
	/// TOTP issuer name
	issuer: String,
	/// User secrets (username -> secret)
	secrets: Arc<Mutex<HashMap<String, String>>>,
	/// Time window for TOTP validation (in seconds)
	time_window: u64,
}

impl MFAAuthentication {
	/// Create a new MFA authentication backend
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::MfaManager;
	///
	/// let mfa = MfaManager::new("MyApp");
	/// ```
	pub fn new(issuer: impl Into<String>) -> Self {
		Self {
			issuer: issuer.into(),
			secrets: Arc::new(Mutex::new(HashMap::new())),
			time_window: 30,
		}
	}

	/// Set time window for TOTP validation
	pub fn time_window(mut self, seconds: u64) -> Self {
		self.time_window = seconds;
		self
	}

	/// Register a user with a secret
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::MfaManager;
	///
	/// let mfa = MfaManager::new("MyApp");
	/// mfa.register_user("alice", "SECRET_BASE32");
	/// ```
	pub fn register_user(&self, username: impl Into<String>, secret: impl Into<String>) {
		let mut secrets = self.secrets.lock().unwrap();
		secrets.insert(username.into(), secret.into());
	}

	/// Generate TOTP URL for QR code
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::MfaManager;
	///
	/// let mfa = MfaManager::new("MyApp");
	/// let url = mfa.generate_totp_url("alice", "SECRET_BASE32");
	/// assert!(url.starts_with("otpauth://totp/"));
	/// ```
	pub fn generate_totp_url(&self, username: &str, secret: &str) -> String {
		format!(
			"otpauth://totp/{}:{}?secret={}&issuer={}",
			self.issuer, username, secret, self.issuer
		)
	}

	/// Verify TOTP code
	///
	/// Verifies a TOTP code using the RFC 6238 algorithm.
	/// The secret must be a valid base32-encoded string.
	pub fn verify_totp(&self, username: &str, code: &str) -> Result<bool, AuthenticationError> {
		let secrets = self.secrets.lock().unwrap();

		if let Some(secret) = secrets.get(username) {
			// Decode base32 secret
			let secret_bytes = match data_encoding::BASE32_NOPAD.decode(secret.as_bytes()) {
				Ok(bytes) => bytes,
				Err(_) => return Err(AuthenticationError::InvalidCredentials),
			};

			// Get current timestamp
			let current_time = std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_secs();

			// Calculate time step
			let time_step = current_time / self.time_window;

			// Generate TOTP for current time window
			let totp = totp_lite::totp_custom::<totp_lite::Sha1>(
				self.time_window,
				6,
				&secret_bytes,
				time_step,
			);

			Ok(totp == code)
		} else {
			Err(AuthenticationError::UserNotFound)
		}
	}

	/// Get the secret for a user (for testing purposes)
	///
	/// Returns the stored TOTP secret for the given user, or None if not registered.
	pub fn get_secret(&self, username: &str) -> Option<String> {
		let secrets = self.secrets.lock().unwrap();
		secrets.get(username).cloned()
	}
}

impl Default for MFAAuthentication {
	fn default() -> Self {
		Self::new("Reinhardt")
	}
}

#[async_trait::async_trait]
impl AuthenticationBackend for MFAAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		// Extract username and MFA code from request headers
		let username = request
			.headers
			.get("X-Username")
			.and_then(|v| v.to_str().ok());
		let code = request
			.headers
			.get("X-MFA-Code")
			.and_then(|v| v.to_str().ok());

		match (username, code) {
			(Some(user), Some(mfa_code)) => {
				if self.verify_totp(user, mfa_code)? {
					Ok(Some(Box::new(SimpleUser {
						id: Uuid::new_v4(),
						username: user.to_string(),
						email: format!("{}@example.com", user),
						is_active: true,
						is_admin: false,
						is_staff: false,
						is_superuser: false,
					})))
				} else {
					Err(AuthenticationError::InvalidCredentials)
				}
			}
			_ => Ok(None),
		}
	}

	async fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		// Check if user exists in our secrets store
		let secrets = self.secrets.lock().unwrap();
		if secrets.contains_key(user_id) {
			Ok(Some(Box::new(SimpleUser {
				id: Uuid::new_v4(),
				username: user_id.to_string(),
				email: format!("{}@example.com", user_id),
				is_active: true,
				is_admin: false,
				is_staff: false,
				is_superuser: false,
			})))
		} else {
			Ok(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method};
	use rstest::rstest;

	#[rstest]
	fn test_mfa_registration() {
		let mfa = MFAAuthentication::new("TestApp");
		mfa.register_user("alice", "JBSWY3DPEHPK3PXP");

		let secrets = mfa.secrets.lock().unwrap();
		assert!(secrets.contains_key("alice"));
	}

	#[rstest]
	fn test_generate_totp_url() {
		let mfa = MFAAuthentication::new("TestApp");
		let url = mfa.generate_totp_url("alice", "SECRET");

		assert!(url.contains("otpauth://totp/"));
		assert!(url.contains("alice"));
		assert!(url.contains("SECRET"));
		assert!(url.contains("TestApp"));
	}

	#[rstest]
	fn test_verify_totp_valid_code() {
		let mfa = MFAAuthentication::new("TestApp");
		// Use a known base32 secret for testing
		let secret = "JBSWY3DPEHPK3PXP"; // Base32 encoded "Hello!"
		mfa.register_user("alice", secret);

		// Generate TOTP for current time
		let current_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		let time_step = current_time / 30;
		let secret_bytes = data_encoding::BASE32_NOPAD
			.decode(secret.as_bytes())
			.unwrap();
		let totp = totp_lite::totp_custom::<totp_lite::Sha1>(30, 6, &secret_bytes, time_step);

		let result = mfa.verify_totp("alice", &totp);
		assert!(result.is_ok());
		assert!(result.unwrap());
	}

	#[rstest]
	fn test_verify_totp_invalid_code() {
		let mfa = MFAAuthentication::new("TestApp");
		let secret = "JBSWY3DPEHPK3PXP";
		mfa.register_user("alice", secret);

		// Invalid TOTP code
		let result = mfa.verify_totp("alice", "000000");
		assert!(result.is_ok());
		assert!(!result.unwrap());
	}

	#[rstest]
	fn test_verify_totp_unregistered_user() {
		let mfa = MFAAuthentication::new("TestApp");

		let result = mfa.verify_totp("alice", "123456");
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_mfa_authentication_with_valid_code() {
		let mfa = MFAAuthentication::new("TestApp");
		let secret = "JBSWY3DPEHPK3PXP";
		mfa.register_user("alice", secret);

		// Generate valid TOTP code
		let current_time = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		let time_step = current_time / 30;
		let secret_bytes = data_encoding::BASE32_NOPAD
			.decode(secret.as_bytes())
			.unwrap();
		let totp = totp_lite::totp_custom::<totp_lite::Sha1>(30, 6, &secret_bytes, time_step);

		let mut headers = HeaderMap::new();
		headers.insert("X-Username", "alice".parse().unwrap());
		headers.insert("X-MFA-Code", totp.parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let result = mfa.authenticate(&request).await.unwrap();
		assert!(result.is_some());
		assert_eq!(result.unwrap().get_username(), "alice");
	}

	#[rstest]
	#[tokio::test]
	async fn test_mfa_authentication_without_headers() {
		let mfa = MFAAuthentication::new("TestApp");

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.body(Bytes::new())
			.build()
			.unwrap();

		let result = mfa.authenticate(&request).await.unwrap();
		assert!(result.is_none());
	}

	#[rstest]
	fn test_time_window_configuration() {
		let mfa = MFAAuthentication::new("TestApp").time_window(60);
		assert_eq!(mfa.time_window, 60);
	}
}
